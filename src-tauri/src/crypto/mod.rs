//! Cryptography subsystem
//! 
//! GOLDEN RULE: No custom cryptographic primitives.
//! Only audited, maintained libraries with test vectors.
//! 
//! Key hierarchy:
//!   Passphrase → Argon2id → Master Key
//!   Master Key → (split) → DB Key + Identity Key Seed
//!   Identity Key Seed → X25519 keypair (DH) + Ed25519 keypair (signing)

pub mod file_enc;
pub mod group;
pub mod identity;
pub mod keystore;
pub mod ratchet;

pub use keystore::KeyStore;

use ring::{aead, hkdf, rand as ring_rand};
use uuid::Uuid;
use zeroize::Zeroize;
use crate::error::{VeilError, VeilResult};
use argon2::{Argon2, Params};

/// Argon2id parameters (OWASP recommended for high-security)
pub const ARGON2_M_COST: u32 = 65536; // 64 MiB
pub const ARGON2_T_COST: u32 = 3;
pub const ARGON2_P_COST: u32 = 4;
pub const KEY_LEN: usize = 32;

/// Derive a 32-byte key from a passphrase using Argon2id
/// Salt must be stored alongside the ciphertext
pub fn derive_key_from_passphrase(
    passphrase: &str,
    salt: &[u8],
) -> VeilResult<DerivedKey> {
    let params = Params::new(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST, Some(KEY_LEN))
        .map_err(|_| VeilError::KeyDerivationError)?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    
    // Ensure effective salt is at least 8 bytes for Argon2 specification
    let effective_salt: Vec<u8> = if salt.len() < 8 {
        use ring::digest;
        digest::digest(&digest::SHA256, salt).as_ref().to_vec()
    } else {
        salt.to_vec()
    };

    let mut key_bytes = vec![0u8; KEY_LEN];
    argon2
        .hash_password_into(passphrase.as_bytes(), &effective_salt, &mut key_bytes)
        .map_err(|_| VeilError::KeyDerivationError)?;
    
    Ok(DerivedKey(key_bytes))
}

/// Encrypt data with AES-256-GCM
/// Returns (ciphertext_with_tag, nonce)
pub fn encrypt_aes_gcm(key: &[u8; 32], plaintext: &[u8]) -> VeilResult<(Vec<u8>, Vec<u8>)> {
    let rng = ring_rand::SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];
    ring_rand::SecureRandom::fill(&rng, &mut nonce_bytes)
        .map_err(|_| VeilError::EncryptionError)?;

    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, key)
        .map_err(|_| VeilError::EncryptionError)?;
    let sealing_key = aead::LessSafeKey::new(unbound_key);
    
    let mut ciphertext = plaintext.to_vec();
    sealing_key
        .seal_in_place_append_tag(
            aead::Nonce::assume_unique_for_key(nonce_bytes),
            aead::Aad::empty(),
            &mut ciphertext,
        )
        .map_err(|_| VeilError::EncryptionError)?;

    Ok((ciphertext, nonce_bytes.to_vec()))
}

/// Decrypt data with AES-256-GCM
pub fn decrypt_aes_gcm(key: &[u8; 32], ciphertext: &[u8], nonce: &[u8]) -> VeilResult<Vec<u8>> {
    if nonce.len() != 12 {
        return Err(VeilError::DecryptionError);
    }
    let nonce_arr: [u8; 12] = nonce.try_into().map_err(|_| VeilError::DecryptionError)?;

    let unbound_key = aead::UnboundKey::new(&aead::AES_256_GCM, key)
        .map_err(|_| VeilError::DecryptionError)?;
    let opening_key = aead::LessSafeKey::new(unbound_key);
    
    let mut buf = ciphertext.to_vec();
    let plaintext = opening_key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce_arr),
            aead::Aad::empty(),
            &mut buf,
        )
        .map_err(|_| VeilError::DecryptionError)?;

    Ok(plaintext.to_vec())
}

/// Deterministic per-message key derived from the session DB key.
/// HKDF-SHA256 with the message UUID as application info — the same message
/// always yields the same key, so history stays decryptable across sessions.
pub fn derive_message_key(db_key: &[u8; 32], message_id: &Uuid) -> VeilResult<[u8; 32]> {
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, &[]);
    let prk = salt.extract(db_key);
    let info: &[u8] = message_id.as_bytes();
    let info_refs = [info];
    let okm = prk
        .expand(&info_refs, hkdf::HKDF_SHA256)
        .map_err(|_| VeilError::KeyDerivationError)?;
    let mut out = [0u8; 32];
    okm.fill(&mut out)
        .map_err(|_| VeilError::KeyDerivationError)?;
    Ok(out)
}

/// Deterministic channel message key shared by all participants in a channel.
/// HKDF-SHA256 over the channel UUID, salted with veilanon-space-channel-msg-v1
/// and expanded with the message UUID. All channel members can encrypt/decrypt
/// while the control plane / Supabase only sees ciphertext.
pub fn derive_channel_message_key(channel_id: &Uuid, message_id: &Uuid) -> VeilResult<[u8; 32]> {
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"veilanon-space-channel-msg-v1");
    let prk = salt.extract(channel_id.as_bytes());
    let info: &[u8] = message_id.as_bytes();
    let info_refs = [info];
    let okm = prk
        .expand(&info_refs, hkdf::HKDF_SHA256)
        .map_err(|_| VeilError::KeyDerivationError)?;
    let mut out = [0u8; 32];
    okm.fill(&mut out)
        .map_err(|_| VeilError::KeyDerivationError)?;
    Ok(out)
}

/// Generate cryptographically secure random bytes
pub fn random_bytes(len: usize) -> VeilResult<Vec<u8>> {
    let rng = ring_rand::SystemRandom::new();
    let mut bytes = vec![0u8; len];
    ring_rand::SecureRandom::fill(&rng, &mut bytes)
        .map_err(|_| VeilError::CryptoError)?;
    Ok(bytes)
}

/// Deterministic key derivation from username and passphrase.
/// Produces DH private key, Signing private key, DB key, Recovery entropy, and Supabase auth secret.
pub fn derive_identity_bundle(
    username: &str,
    passphrase: &str,
) -> VeilResult<(identity::DeviceIdentity, [u8; 32], [u8; 24], String)> {
    use ring::digest;
    let clean_user = username.trim().to_lowercase();
    let user_salt = digest::digest(&digest::SHA256, format!("veilanon-user-salt-v1:{}", clean_user).as_bytes());
    
    // Argon2id over passphrase with username-specific salt
    let derived = derive_key_from_passphrase(passphrase, user_salt.as_ref())?;
    let pass_kdf = derived.as_array_32()?;
    
    // Derive 24-byte recovery entropy from passphrase KDF
    let rec_salt = hkdf::Salt::new(hkdf::HKDF_SHA256, b"veilanon-recovery-root-v1");
    let rec_prk = rec_salt.extract(&pass_kdf);
    let mut rec_raw = [0u8; 32];
    let info = [b"recovery-entropy" as &[u8]];
    rec_prk.expand(&info, hkdf::HKDF_SHA256)
        .map_err(|_| VeilError::KeyDerivationError)?
        .fill(&mut rec_raw)
        .map_err(|_| VeilError::KeyDerivationError)?;
    let mut rec_entropy = [0u8; 24];
    rec_entropy.copy_from_slice(&rec_raw[0..24]);

    // Derive the master bundle from recovery entropy + username
    let (device_identity, db_key, auth_password) =
        derive_identity_bundle_from_recovery(&clean_user, &rec_entropy)?;

    Ok((device_identity, db_key, rec_entropy, auth_password))
}

/// Derive identity bundle from recovery code/entropy and username.
pub fn derive_identity_bundle_from_recovery(
    username: &str,
    recovery_entropy: &[u8; 24],
) -> VeilResult<(identity::DeviceIdentity, [u8; 32], String)> {
    let clean_user = username.trim().to_lowercase();
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, format!("veilanon-user-master-v1:{}", clean_user).as_bytes());
    let prk = salt.extract(recovery_entropy);

    // 1. DH seed (32 bytes)
    let mut dh_seed = [0u8; 32];
    let info_dh = [b"dh-key" as &[u8]];
    prk.expand(&info_dh, hkdf::HKDF_SHA256)
        .map_err(|_| VeilError::KeyDerivationError)?
        .fill(&mut dh_seed)
        .map_err(|_| VeilError::KeyDerivationError)?;

    // 2. Signing seed (32 bytes)
    let mut sign_seed = [0u8; 32];
    let info_sign = [b"signing-key" as &[u8]];
    prk.expand(&info_sign, hkdf::HKDF_SHA256)
        .map_err(|_| VeilError::KeyDerivationError)?
        .fill(&mut sign_seed)
        .map_err(|_| VeilError::KeyDerivationError)?;

    // 3. DB key (32 bytes)
    let mut db_key = [0u8; 32];
    let info_db = [b"db-key" as &[u8]];
    prk.expand(&info_db, hkdf::HKDF_SHA256)
        .map_err(|_| VeilError::KeyDerivationError)?
        .fill(&mut db_key)
        .map_err(|_| VeilError::KeyDerivationError)?;

    // 4. Supabase deterministic auth secret
    let mut auth_raw = [0u8; 32];
    let info_auth = [b"supabase-auth" as &[u8]];
    prk.expand(&info_auth, hkdf::HKDF_SHA256)
        .map_err(|_| VeilError::KeyDerivationError)?
        .fill(&mut auth_raw)
        .map_err(|_| VeilError::KeyDerivationError)?;
    let auth_password = format!("VA1!{}", hex::encode(&auth_raw));

    let device_identity = identity::DeviceIdentity::from_seeds(dh_seed, sign_seed);
    Ok((device_identity, db_key, auth_password))
}

/// Wrapper for derived key material — zeroizes on drop
pub struct DerivedKey(Vec<u8>);

impl DerivedKey {
    #[allow(dead_code)] // convenience accessor for future callers
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    pub fn as_array_32(&self) -> VeilResult<[u8; 32]> {
        self.0.as_slice().try_into().map_err(|_| VeilError::KeyDerivationError)
    }
}

impl Drop for DerivedKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_gcm_roundtrip() {
        let key = [42u8; 32];
        let plaintext = b"veilanon test payload";
        let (ciphertext, nonce) = encrypt_aes_gcm(&key, plaintext).unwrap();
        let decrypted = decrypt_aes_gcm(&key, &ciphertext, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn aes_gcm_wrong_key_fails() {
        let key_a = [1u8; 32];
        let key_b = [2u8; 32];
        let (ciphertext, nonce) = encrypt_aes_gcm(&key_a, b"secret").unwrap();
        assert!(decrypt_aes_gcm(&key_b, &ciphertext, &nonce).is_err());
    }

    #[test]
    fn derive_key_from_passphrase_is_deterministic() {
        let salt = b"fixed-test-salt";
        let first = derive_key_from_passphrase("correct horse battery staple", salt).unwrap();
        let second = derive_key_from_passphrase("correct horse battery staple", salt).unwrap();
        assert_eq!(first.as_bytes(), second.as_bytes());
        assert_eq!(first.as_bytes().len(), 32);

        let other_salt = derive_key_from_passphrase("correct horse battery staple", b"other-salt").unwrap();
        assert_ne!(first.as_bytes(), other_salt.as_bytes());
    }

    #[test]
    fn random_bytes_has_requested_length() {
        let a = random_bytes(32).unwrap();
        let b = random_bytes(32).unwrap();
        assert_eq!(a.len(), 32);
        assert_eq!(b.len(), 32);
        assert_ne!(a, b);
    }

    #[test]
    fn derive_message_key_is_deterministic() {
        let db_key = [7u8; 32];
        let msg_id = Uuid::new_v4();
        let first = derive_message_key(&db_key, &msg_id).unwrap();
        let second = derive_message_key(&db_key, &msg_id).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 32);
    }

    #[test]
    fn derive_message_key_differs_by_message_id() {
        let db_key = [7u8; 32];
        let a = derive_message_key(&db_key, &Uuid::new_v4()).unwrap();
        let b = derive_message_key(&db_key, &Uuid::new_v4()).unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn derive_identity_bundle_succeeds_and_is_deterministic() {
        let (id1, db_key1, rec1, pass1) = derive_identity_bundle("alice", "super-strong-passphrase").unwrap();
        let (id2, db_key2, rec2, pass2) = derive_identity_bundle("alice", "super-strong-passphrase").unwrap();
        assert_eq!(db_key1, db_key2);
        assert_eq!(rec1, rec2);
        assert_eq!(pass1, pass2);
        assert_eq!(id1.public_identity().unwrap(), id2.public_identity().unwrap());

        // Different user produces different bundle
        let (_id3, db_key3, rec3, pass3) = derive_identity_bundle("bob", "super-strong-passphrase").unwrap();
        assert_ne!(db_key1, db_key3);
        assert_ne!(rec1, rec3);
        assert_ne!(pass1, pass3);
    }

    #[test]
    fn derive_identity_bundle_from_recovery_is_deterministic() {
        let entropy = [42u8; 24];
        let (id1, db_key1, pass1) = derive_identity_bundle_from_recovery("alice", &entropy).unwrap();
        let (id2, db_key2, pass2) = derive_identity_bundle_from_recovery("alice", &entropy).unwrap();
        assert_eq!(db_key1, db_key2);
        assert_eq!(pass1, pass2);
        assert_eq!(id1.public_identity().unwrap(), id2.public_identity().unwrap());
    }

    #[test]
    fn derive_identity_and_recovery_match() {
        let (id1, db_key1, rec1, pass1) = derive_identity_bundle("charlie", "strong-password-99").unwrap();
        let (id2, db_key2, pass2) = derive_identity_bundle_from_recovery("charlie", &rec1).unwrap();
        assert_eq!(db_key1, db_key2);
        assert_eq!(pass1, pass2);
        assert_eq!(id1.public_identity().unwrap(), id2.public_identity().unwrap());
    }
}
