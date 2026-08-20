//! OS Keychain integration
//! 
//! Stores the encrypted master key and identity key material in the OS keychain.
//! On Windows: Windows Credential Store
//! On macOS: Keychain
//! On Linux: libsecret / Secret Service API
//! 
//! SECURITY:
//! The identity bundle is encrypted with a random 32-byte *master key*. The
//! master key itself is wrapped twice and stored alongside the bundle:
//!   1. Passphrase wrap  — AES-256-GCM(master_key, Argon2id(passphrase))
//!   2. Recovery wrap    — AES-256-GCM(master_key, SHA-256("veilanon-recovery-v1" || recovery_entropy))
//! 
//! This is what makes the recovery code actually usable: the code is derived
//! from entropy that is NOT gated behind the passphrase, so a forgotten
//! passphrase can be replaced with a new one (recover_identity). Without the
//! passphrase OR the recovery code, the master key — and therefore the bundle —
//! remains unrecoverable.
//! 
//! v1 bundles (bundle encrypted directly with the passphrase-derived key,
//! entropy trapped inside) are still readable via load_keys for migration;
//! recovery is unavailable for them by design.

use keyring::Entry;
use std::path::Path;
use anyhow::Result;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ring::digest;
use crate::error::{VeilError, VeilResult};
use crate::crypto::{derive_key_from_passphrase, encrypt_aes_gcm, decrypt_aes_gcm, random_bytes};
use zeroize::Zeroize;

const SERVICE_NAME: &str = "com.aegissoft.veilanon";
const IDENTITY_KEY_ENTRY: &str = "identity_keys";
const AUTO_UNLOCK_ENTRY: &str = "auto_unlock_passphrase";
const SALT_FILE: &str = "kdf_salt.bin";
#[allow(dead_code)] // v1 is read-only; new writes use V2
const BUNDLE_VERSION_V1: u8 = 1;
const BUNDLE_VERSION_V2: u8 = 2;

/// Domain separator for the recovery-wrap key derivation.
const RECOVERY_DOMAIN: &[u8] = b"veilanon-recovery-v1";

pub struct KeyStore {
    data_dir: std::path::PathBuf,
    /// OS keychain service name — overridable so tests never touch the real
    /// application credentials.
    service_name: String,
    /// In-memory mirror of the recovery entropy, populated on save/load.
    /// Lets `verify_recovery_code` compare without re-deriving the passphrase.
    recovery_entropy_cache: std::sync::Mutex<Option<[u8; 24]>>,
}

/// Stored key bundle (v2) — serialized, then the payload is encrypted with a
/// random master key; the master key is wrapped for passphrase AND recovery.
#[derive(serde::Serialize, serde::Deserialize)]
struct EncryptedKeyBundle {
    version: u8,
    /// Bundle payload encrypted with the master key (base64)
    ciphertext: String,
    /// Nonce for the payload encryption (base64)
    nonce: String,
    /// Master key wrapped with the passphrase-derived key (base64)
    pass_ciphertext: String,
    pass_nonce: String,
    /// Master key wrapped with the recovery-entropy-derived key (base64)
    recover_ciphertext: String,
    recover_nonce: String,
}

/// Legacy (v1) bundle — bundle encrypted directly with the passphrase key.
#[derive(serde::Serialize, serde::Deserialize)]
struct EncryptedKeyBundleV1 {
    ciphertext: String,
    nonce: String,
}

/// Decrypted key material — zeroized on drop
/// Layout: dh_private(32) || signing_private(32) || db_key(32) || recovery_entropy(24) = 120 bytes
#[derive(Zeroize)]
#[zeroize(drop)]
pub struct DecryptedKeyBundle {
    pub dh_private: [u8; 32],
    pub signing_private: [u8; 32],
    pub db_key: [u8; 32],
    /// 24 bytes of random entropy from which the recovery code is derived
    pub recovery_entropy: [u8; 24],
}

/// Bundle payload size in bytes (see `DecryptedKeyBundle`)
const BUNDLE_LEN: usize = 120;

/// Format recovery entropy as a grouped hex code: XXXX-XXXX-... (8-char groups)
pub fn format_recovery_code(entropy: &[u8; 24]) -> String {
    hex::encode(entropy)
        .chars()
        .collect::<Vec<char>>()
        .chunks(8)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("-")
}

/// Parse a recovery code (optionally dash-grouped, VEIL- prefixed, or hex) back into raw entropy bytes (24 bytes).
pub fn parse_recovery_code(code: &str) -> VeilResult<Vec<u8>> {
    let trimmed = code.trim();
    if trimmed.is_empty() {
        return Err(VeilError::InvalidRecoveryCode);
    }

    // Strip common prefix variations: VEIL-, veil-, VEIL_, veil:
    let without_prefix = if let Some(rest) = trimmed
        .strip_prefix("VEIL-")
        .or_else(|| trimmed.strip_prefix("veil-"))
        .or_else(|| trimmed.strip_prefix("VEIL_"))
        .or_else(|| trimmed.strip_prefix("veil_"))
        .or_else(|| trimmed.strip_prefix("VEIL:"))
        .or_else(|| trimmed.strip_prefix("veil:"))
    {
        rest.trim()
    } else {
        trimmed
    };

    // 1. Try pure hex parse without dashes, spaces, colons, underscores
    let compact: String = without_prefix
        .chars()
        .filter(|c| *c != '-' && *c != ' ' && *c != ':' && *c != '_')
        .collect();

    if compact.len() == 48 {
        if let Ok(bytes) = hex::decode(&compact) {
            if bytes.len() == 24 {
                return Ok(bytes);
            }
        }
    }

    // 2. Try Base64 / Base64URL decoding
    if let Ok(bytes) = B64.decode(&compact) {
        if bytes.len() == 24 {
            return Ok(bytes);
        }
    }
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    if let Ok(bytes) = URL_SAFE_NO_PAD.decode(&compact) {
        if bytes.len() == 24 {
            return Ok(bytes);
        }
    }

    // 3. Try raw ASCII / UTF-8 if length is exactly 24 bytes
    if without_prefix.as_bytes().len() == 24 {
        return Ok(without_prefix.as_bytes().to_vec());
    }

    // 4. Try any valid hex decode
    if let Ok(bytes) = hex::decode(&compact) {
        if bytes.len() == 24 {
            return Ok(bytes);
        }
    }

    Err(VeilError::InvalidRecoveryCode)
}

/// Derive the recovery-wrap key from the recovery entropy.
/// SHA-256 is sufficient here: the entropy is 24 random bytes (192 bits).
fn recovery_key_from_entropy(entropy: &[u8; 24]) -> [u8; 32] {
    let mut ctx = digest::Context::new(&digest::SHA256);
    ctx.update(RECOVERY_DOMAIN);
    ctx.update(entropy);
    let out = ctx.finish();
    let mut key = [0u8; 32];
    key.copy_from_slice(out.as_ref());
    key
}

impl KeyStore {
    pub fn new(data_dir: &Path) -> Result<Self> {
        Self::with_service(data_dir, SERVICE_NAME)
    }

    /// Constructor with an explicit keychain service name (tests isolate
    /// themselves from the real application credentials).
    pub fn with_service(data_dir: &Path, service_name: &str) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        Ok(Self {
            data_dir: data_dir.to_path_buf(),
            service_name: service_name.to_string(),
            recovery_entropy_cache: std::sync::Mutex::new(None),
        })
    }

    /// Copy of the recovery entropy held in memory while authenticated
    pub fn recovery_entropy(&self) -> Option<[u8; 24]> {
        *self.recovery_entropy_cache.lock().ok()?
    }

    fn cache_recovery_entropy(&self, entropy: [u8; 24]) {
        if let Ok(mut cache) = self.recovery_entropy_cache.lock() {
            *cache = Some(entropy);
        }
    }

    /// Clear all cached key material (called on sign-out)
    pub fn clear_cached_secrets(&self) {
        if let Ok(mut cache) = self.recovery_entropy_cache.lock() {
            if let Some(mut entropy) = cache.take() {
                entropy.zeroize();
            }
        }
    }

    /// Serialize a bundle into the fixed 120-byte plaintext layout.
    fn serialize_bundle(keys: &DecryptedKeyBundle) -> Vec<u8> {
        let mut bundle_bytes = Vec::with_capacity(BUNDLE_LEN);
        bundle_bytes.extend_from_slice(&keys.dh_private);
        bundle_bytes.extend_from_slice(&keys.signing_private);
        bundle_bytes.extend_from_slice(&keys.db_key);
        bundle_bytes.extend_from_slice(&keys.recovery_entropy);
        bundle_bytes
    }

    /// Deserialize a 120-byte plaintext bundle, zeroizing the input buffer.
    fn deserialize_bundle(plaintext: &mut Vec<u8>) -> VeilResult<DecryptedKeyBundle> {
        if plaintext.len() < BUNDLE_LEN {
            plaintext.zeroize();
            return Err(VeilError::DecryptionError);
        }
        let mut dh_private = [0u8; 32];
        let mut signing_private = [0u8; 32];
        let mut db_key = [0u8; 32];
        let mut recovery_entropy = [0u8; 24];
        dh_private.copy_from_slice(&plaintext[0..32]);
        signing_private.copy_from_slice(&plaintext[32..64]);
        db_key.copy_from_slice(&plaintext[64..96]);
        recovery_entropy.copy_from_slice(&plaintext[96..120]);
        plaintext.zeroize();
        Ok(DecryptedKeyBundle { dh_private, signing_private, db_key, recovery_entropy })
    }

    /// Save key material with the v2 envelope: bundle encrypted under a fresh
    /// master key; the master key wrapped for the passphrase and the recovery
    /// code.
    pub fn save_keys(
        &self,
        passphrase: &str,
        keys: &DecryptedKeyBundle,
    ) -> VeilResult<()> {
        // 1. Encrypt the bundle with a fresh random master key.
        let mut master_key = [0u8; 32];
        let master_bytes = random_bytes(32)?;
        master_key.copy_from_slice(&master_bytes);

        let mut bundle_bytes = Self::serialize_bundle(keys);
        let (ciphertext, nonce) = encrypt_aes_gcm(&master_key, &bundle_bytes)?;
        bundle_bytes.zeroize();

        // 2. Wrap the master key with the passphrase-derived key (Argon2id).
        let salt = self.get_or_create_salt()?;
        let derived_key = derive_key_from_passphrase(passphrase, &salt)?;
        let pass_key = derived_key.as_array_32()?;
        let (pass_ciphertext, pass_nonce) = encrypt_aes_gcm(&pass_key, &master_key)?;

        // 3. Wrap the master key with the recovery-entropy-derived key.
        let rec_key = recovery_key_from_entropy(&keys.recovery_entropy);
        let (recover_ciphertext, recover_nonce) = encrypt_aes_gcm(&rec_key, &master_key)?;
        master_key.zeroize();

        // 4. Store in the OS keychain.
        let entry = Entry::new(&self.service_name, IDENTITY_KEY_ENTRY)
            .map_err(|_| VeilError::CryptoError)?;
        let bundle = EncryptedKeyBundle {
            version: BUNDLE_VERSION_V2,
            ciphertext: B64.encode(&ciphertext),
            nonce: B64.encode(&nonce),
            pass_ciphertext: B64.encode(&pass_ciphertext),
            pass_nonce: B64.encode(&pass_nonce),
            recover_ciphertext: B64.encode(&recover_ciphertext),
            recover_nonce: B64.encode(&recover_nonce),
        };
        let payload = serde_json::to_string(&bundle)
            .map_err(|_| VeilError::CryptoError)?;
        entry.set_password(&payload)
            .map_err(|_| VeilError::CryptoError)?;

        self.cache_recovery_entropy(keys.recovery_entropy);
        Ok(())
    }

    /// Load and decrypt key material using the user's passphrase.
    /// v2 envelopes unwrap the master key via the passphrase wrap; legacy v1
    /// bundles are decrypted directly (migration path).
    pub fn load_keys(&self, passphrase: &str) -> VeilResult<DecryptedKeyBundle> {
        let entry = Entry::new(&self.service_name, IDENTITY_KEY_ENTRY)
            .map_err(|_| VeilError::IdentityNotFound)?;
        let payload_str = entry.get_password()
            .map_err(|_| VeilError::IdentityNotFound)?;

        // Detect envelope version without losing field access.
        let parsed: serde_json::Value = serde_json::from_str(&payload_str)
            .map_err(|_| VeilError::DecryptionError)?;
        let version = parsed.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u8;

        if version == BUNDLE_VERSION_V2 {
            let bundle: EncryptedKeyBundle = serde_json::from_value(parsed)
                .map_err(|_| VeilError::DecryptionError)?;

            let salt = self.get_or_create_salt()?;
            let derived_key = derive_key_from_passphrase(passphrase, &salt)?;
            let pass_key = derived_key.as_array_32()?;

            let pass_ct = B64.decode(&bundle.pass_ciphertext)
                .map_err(|_| VeilError::DecryptionError)?;
            let pass_nonce = B64.decode(&bundle.pass_nonce)
                .map_err(|_| VeilError::DecryptionError)?;
            let mut master_key: [u8; 32] = decrypt_aes_gcm(&pass_key, &pass_ct, &pass_nonce)?
                .try_into()
                .map_err(|_| VeilError::DecryptionError)?;

            let ciphertext = B64.decode(&bundle.ciphertext)
                .map_err(|_| VeilError::DecryptionError)?;
            let nonce = B64.decode(&bundle.nonce)
                .map_err(|_| VeilError::DecryptionError)?;
            let mut plaintext = decrypt_aes_gcm(&master_key, &ciphertext, &nonce)?;
            master_key.zeroize();

            let keys = Self::deserialize_bundle(&mut plaintext)?;
            self.cache_recovery_entropy(keys.recovery_entropy);
            Ok(keys)
        } else {
            // Legacy v1: bundle encrypted directly with the passphrase key.
            let bundle: EncryptedKeyBundleV1 = serde_json::from_value(parsed)
                .map_err(|_| VeilError::DecryptionError)?;
            let salt = self.get_or_create_salt()?;
            let derived_key = derive_key_from_passphrase(passphrase, &salt)?;
            let key_arr = derived_key.as_array_32()?;

            let ciphertext = B64.decode(&bundle.ciphertext)
                .map_err(|_| VeilError::DecryptionError)?;
            let nonce = B64.decode(&bundle.nonce)
                .map_err(|_| VeilError::DecryptionError)?;
            let mut plaintext = decrypt_aes_gcm(&key_arr, &ciphertext, &nonce)?;
            let keys = Self::deserialize_bundle(&mut plaintext)?;
            self.cache_recovery_entropy(keys.recovery_entropy);
            Ok(keys)
        }
    }

    /// Recover key material using ONLY the recovery code, then re-wrap the
    /// bundle under a NEW passphrase. This is the "forgot my passphrase" path:
    /// the caller persists the fresh passphrase wrap and the identity is
    /// unlocked with the returned bundle.
    pub fn recover_keys(
        &self,
        recovery_code: &str,
        new_passphrase: &str,
    ) -> VeilResult<DecryptedKeyBundle> {
        if new_passphrase.len() < 8 {
            return Err(VeilError::InvalidInput(
                "Passphrase must be at least 8 characters".into(),
            ));
        }

        // Recovery entropy comes from the code itself — NOT from the bundle.
        let supplied = parse_recovery_code(recovery_code)?;
        if supplied.len() != 24 {
            return Err(VeilError::InvalidRecoveryCode);
        }
        let mut entropy = [0u8; 24];
        entropy.copy_from_slice(&supplied);

        let entry = Entry::new(&self.service_name, IDENTITY_KEY_ENTRY)
            .map_err(|_| VeilError::IdentityNotFound)?;
        let payload_str = entry.get_password()
            .map_err(|_| VeilError::IdentityNotFound)?;

        let parsed: serde_json::Value = serde_json::from_str(&payload_str)
            .map_err(|_| VeilError::DecryptionError)?;
        let version = parsed.get("version").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
        if version != BUNDLE_VERSION_V2 {
            // v1 bundles have the entropy trapped behind the passphrase —
            // recovery is impossible for them.
            return Err(VeilError::InvalidRecoveryCode);
        }

        let bundle: EncryptedKeyBundle = serde_json::from_value(parsed)
            .map_err(|_| VeilError::DecryptionError)?;

        // Unwrap the master key with the recovery-entropy-derived key.
        let rec_key = recovery_key_from_entropy(&entropy);
        let rec_ct = B64.decode(&bundle.recover_ciphertext)
            .map_err(|_| VeilError::DecryptionError)?;
        let rec_nonce = B64.decode(&bundle.recover_nonce)
            .map_err(|_| VeilError::DecryptionError)?;
        let mut master_key: [u8; 32] = decrypt_aes_gcm(&rec_key, &rec_ct, &rec_nonce)?
            .try_into()
            .map_err(|_| VeilError::InvalidRecoveryCode)?;

        // Decrypt the bundle.
        let ciphertext = B64.decode(&bundle.ciphertext)
            .map_err(|_| VeilError::DecryptionError)?;
        let nonce = B64.decode(&bundle.nonce)
            .map_err(|_| VeilError::DecryptionError)?;
        let mut plaintext = decrypt_aes_gcm(&master_key, &ciphertext, &nonce)?;
        master_key.zeroize();

        let keys = Self::deserialize_bundle(&mut plaintext)?;
        // The entropy embedded in the bundle must match the code, otherwise
        // something is inconsistent — fail closed.
        if keys.recovery_entropy != entropy {
            return Err(VeilError::InvalidRecoveryCode);
        }

        // Re-wrap under the new passphrase (fresh master key, same bundle).
        self.save_keys(new_passphrase, &keys)?;
        self.cache_recovery_entropy(keys.recovery_entropy);
        Ok(keys)
    }

    /// Check if identity exists
    pub fn has_identity(&self) -> bool {
        Entry::new(&self.service_name, IDENTITY_KEY_ENTRY)
            .and_then(|e| e.get_password())
            .is_ok()
    }

    /// Delete all stored keys (use with care — irreversible!)
    pub fn delete_keys(&self) -> VeilResult<()> {
        if let Ok(entry) = Entry::new(&self.service_name, IDENTITY_KEY_ENTRY) {
            let _ = entry.delete_credential();
        }
        self.clear_cached_secrets();
        let salt_path = self.data_dir.join(SALT_FILE);
        if salt_path.exists() {
            std::fs::remove_file(salt_path)?;
        }
        Ok(())
    }

    fn get_or_create_salt(&self) -> VeilResult<Vec<u8>> {
        let salt_path = self.data_dir.join(SALT_FILE);
        if salt_path.exists() {
            Ok(std::fs::read(&salt_path)?)
        } else {
            let salt = random_bytes(32)?;
            std::fs::write(&salt_path, &salt)?;
            Ok(salt)
        }
    }

    /// Save the session passphrase in the OS keychain and encrypted local fallback for instant unlock on startup.
    pub fn save_auto_unlock(&self, passphrase: &str) -> VeilResult<()> {
        let _ = Entry::new(&self.service_name, AUTO_UNLOCK_ENTRY)
            .and_then(|e| e.set_password(passphrase));

        let salt = self.get_or_create_salt()?;
        let derived = crate::crypto::derive_key_from_passphrase(&self.service_name, &salt)?;
        if derived.0.len() >= 32 {
            let mut key_arr = [0u8; 32];
            key_arr.copy_from_slice(&derived.0[..32]);
            let (ciphertext, nonce) = crate::crypto::encrypt_aes_gcm(&key_arr, passphrase.as_bytes())?;

            let mut data = Vec::with_capacity(nonce.len() + ciphertext.len());
            data.extend_from_slice(&nonce);
            data.extend_from_slice(&ciphertext);

            let path = self.data_dir.join("autounlock.bin");
            std::fs::write(path, data)?;
        }
        Ok(())
    }

    /// Load the session passphrase from the OS keychain or encrypted fallback.
    pub fn load_auto_unlock(&self) -> Option<String> {
        if let Ok(entry) = Entry::new(&self.service_name, AUTO_UNLOCK_ENTRY) {
            if let Ok(pass) = entry.get_password() {
                if !pass.is_empty() {
                    return Some(pass);
                }
            }
        }

        let path = self.data_dir.join("autounlock.bin");
        if path.exists() {
            if let Ok(data) = std::fs::read(&path) {
                if data.len() > 12 {
                    let nonce = &data[..12];
                    let ciphertext = &data[12..];
                    if let Ok(salt) = self.get_or_create_salt() {
                        if let Ok(derived) = crate::crypto::derive_key_from_passphrase(&self.service_name, &salt) {
                            if derived.0.len() >= 32 {
                                let mut key_arr = [0u8; 32];
                                key_arr.copy_from_slice(&derived.0[..32]);
                                if let Ok(plaintext) = crate::crypto::decrypt_aes_gcm(&key_arr, ciphertext, nonce) {
                                    if let Ok(s) = String::from_utf8(plaintext) {
                                        return Some(s);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Remove the session passphrase from the OS keychain and local storage.
    pub fn clear_auto_unlock(&self) -> VeilResult<()> {
        if let Ok(entry) = Entry::new(&self.service_name, AUTO_UNLOCK_ENTRY) {
            let _ = entry.delete_credential();
        }
        let path = self.data_dir.join("autounlock.bin");
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
        Ok(())
    }

    /// Check whether an auto-unlock credential exists.
    pub fn has_auto_unlock(&self) -> bool {
        if let Ok(entry) = Entry::new(&self.service_name, AUTO_UNLOCK_ENTRY) {
            if let Ok(pass) = entry.get_password() {
                if !pass.is_empty() {
                    return true;
                }
            }
        }
        let path = self.data_dir.join("autounlock.bin");
        path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::identity::DeviceIdentity;

    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("veilanon-keystore-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn test_keystore(dir: &std::path::Path) -> KeyStore {
        KeyStore::with_service(dir, &format!("com.aegissoft.veilanon.test-{}", uuid::Uuid::new_v4())).unwrap()
    }

    fn cleanup(ks: &KeyStore, dir: &std::path::Path) {
        ks.delete_keys().ok();
        std::fs::remove_dir_all(dir).ok();
    }

    fn test_bundle() -> DecryptedKeyBundle {
        let device = DeviceIdentity::generate().unwrap();
        let (dh_priv, sign_priv) = device.export_private_bytes();
        let mut dh_private = [0u8; 32];
        let mut signing_private = [0u8; 32];
        dh_private.copy_from_slice(&dh_priv);
        signing_private.copy_from_slice(&sign_priv);
        let db_key = [9u8; 32];
        let recovery_entropy = random_bytes(24).unwrap().try_into().unwrap();
        DecryptedKeyBundle { dh_private, signing_private, db_key, recovery_entropy }
    }

    #[test]
    fn recovery_code_round_trip_format() {
        let entropy = [0xABu8; 24];
        let code = format_recovery_code(&entropy);
        assert_eq!(code.len(), 8 * 6 + 5); // 48 hex + 5 dashes
        let parsed = parse_recovery_code(&code).unwrap();
        assert_eq!(parsed, entropy.to_vec());
        // Dash-free variant parses too.
        let compact = code.replace('-', "");
        assert_eq!(parse_recovery_code(&compact).unwrap(), entropy.to_vec());
        // Wrong length rejected.
        assert!(parse_recovery_code("ABCD").is_err());
    }

    #[test]
    fn v2_save_load_round_trip_with_passphrase() {
        let dir = temp_dir();
        let ks = test_keystore(&dir);
        let bundle = test_bundle();
        ks.save_keys("correct horse battery staple", &bundle).unwrap();

        let loaded = ks.load_keys("correct horse battery staple").unwrap();
        assert_eq!(loaded.dh_private, bundle.dh_private);
        assert_eq!(loaded.signing_private, bundle.signing_private);
        assert_eq!(loaded.db_key, bundle.db_key);
        assert_eq!(loaded.recovery_entropy, bundle.recovery_entropy);

        // Wrong passphrase must fail.
        assert!(ks.load_keys("wrong passphrase").is_err());
        cleanup(&ks, &dir);
    }

    #[test]
    fn v2_recovery_unlocks_and_rekeys() {
        let dir = temp_dir();
        let ks = test_keystore(&dir);
        let bundle = test_bundle();
        ks.save_keys("old-passphrase-123", &bundle).unwrap();
        let code = format_recovery_code(&bundle.recovery_entropy);

        // Recover with the code + a NEW passphrase.
        let recovered = ks.recover_keys(&code, "new-passphrase-456").unwrap();
        assert_eq!(recovered.db_key, bundle.db_key);
        assert_eq!(recovered.dh_private, bundle.dh_private);

        // Old passphrase no longer works…
        assert!(ks.load_keys("old-passphrase-123").is_err());
        // …new passphrase does.
        let reloaded = ks.load_keys("new-passphrase-456").unwrap();
        assert_eq!(reloaded.db_key, bundle.db_key);
        cleanup(&ks, &dir);
    }

    #[test]
    fn v2_recovery_rejects_wrong_code() {
        let dir = temp_dir();
        let ks = test_keystore(&dir);
        let bundle = test_bundle();
        ks.save_keys("old-passphrase-123", &bundle).unwrap();

        // Random wrong code (48 hex chars).
        let wrong = format_recovery_code(&[7u8; 24]);
        assert!(ks.recover_keys(&wrong, "new-passphrase-456").is_err());
        cleanup(&ks, &dir);
    }

    #[test]
    fn v1_bundle_still_loads_with_passphrase() {
        let dir = temp_dir();
        let ks = test_keystore(&dir);
        let bundle = test_bundle();

        // Simulate a legacy v1 envelope written by older builds.
        let salt = ks.get_or_create_salt().unwrap();
        let derived = derive_key_from_passphrase("legacy-passphrase", &salt).unwrap();
        let key = derived.as_array_32().unwrap();
        let mut raw = Vec::new();
        raw.extend_from_slice(&bundle.dh_private);
        raw.extend_from_slice(&bundle.signing_private);
        raw.extend_from_slice(&bundle.db_key);
        raw.extend_from_slice(&bundle.recovery_entropy);
        let (ciphertext, nonce) = encrypt_aes_gcm(&key, &raw).unwrap();
        raw.zeroize();
        let v1 = EncryptedKeyBundleV1 {
            ciphertext: B64.encode(&ciphertext),
            nonce: B64.encode(&nonce),
        };
        let entry = Entry::new(&ks.service_name, IDENTITY_KEY_ENTRY).unwrap(); entry.set_password(&serde_json::to_string(&v1).unwrap()).unwrap();

        let loaded = ks.load_keys("legacy-passphrase").unwrap();
        assert_eq!(loaded.db_key, bundle.db_key);
        assert_eq!(loaded.recovery_entropy, bundle.recovery_entropy);

        // Recovery is impossible for v1 bundles (entropy is trapped).
        let code = format_recovery_code(&bundle.recovery_entropy);
        assert!(ks.recover_keys(&code, "brand-new-passphrase").is_err());

        entry.delete_credential().ok();
        cleanup(&ks, &dir);
    }
}
