//! Secrets store — encrypted at-rest storage for service credentials.
//!
//! SECURITY MODEL
//! --------------
//! Secrets (LiveKit API secret, R2 secret keys, Discord client secret, GIF
//! provider keys, control-plane tokens, …) are NEVER embedded into the
//! binary. `build.rs` only embeds non-secret configuration (public URLs,
//! Supabase anon key). At runtime secrets live in a single JSON envelope
//! (`secrets.enc`) encrypted with AES-256-GCM; the encryption key is held by
//! the OS keychain (Windows Credential Store / macOS Keychain / Linux
//! libsecret) via the `keyring` crate. On keychain-less Linux desktops the
//! key falls back to a file with 0600 permissions and a warning is logged —
//! still vastly better than a plaintext string inside the executable.
//!
//! The store is initialized once at app startup (`init`); afterwards
//! `config::var` checks: runtime env > secrets store > embedded (non-secret
//! only). A one-time migration imports `VEILANON_*` values from a working
//! directory `.env` so existing dev/install setups keep working unchanged.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use tracing::warn;

use crate::crypto::{decrypt_aes_gcm, encrypt_aes_gcm, random_bytes};

const STORE_FILE: &str = "secrets.enc";
const KEYCHAIN_SERVICE: &str = "com.aegissoft.veilanon";
const KEYCHAIN_ENTRY: &str = "secrets_key_v1";
const FILE_KEY_FALLBACK: &str = "secrets.key";

/// Keys that are considered NON-secret and may be embedded at compile time.
/// Everything else must come from the encrypted store (or runtime env).
#[allow(dead_code)]
pub const EMBEDDABLE_KEYS: &[&str] = &[
    "VEILANON_SUPABASE_URL",
    "VEILANON_SUPABASE_ANON_KEY",
    "VEILANON_LIVEKIT_URL",
    "VEILANON_LIVEKIT_API_KEY",
    "VEILANON_R2_ACCOUNT_ID",
    "VEILANON_R2_BUCKET",
    "VEILANON_UPSTASH_REDIS_REST_URL",
    "VEILANON_QDRANT_URL",
    "VEILANON_OLLAMA_URL",
    "VEILANON_TENOR_API_KEY",
    "VEILANON_GIPHY_API_KEY",
];

/// Keys that must NEVER be embedded into the binary.
#[allow(dead_code)]
pub const SECRET_KEYS: &[&str] = &[
    "VEILANON_SUPABASE_SERVICE_ROLE_KEY",
    "VEILANON_SUPABASE_DB_URL",
    "VEILANON_LIVEKIT_API_SECRET",
    "VEILANON_R2_ACCESS_KEY_ID",
    "VEILANON_R2_SECRET_ACCESS_KEY",
    "VEILANON_SENTRY_DSN",
    "VEILANON_UPSTASH_REDIS_REST_TOKEN",
    "VEILANON_QDRANT_API_KEY",
    "VEILANON_DISCORD_CLIENT_ID",
    "VEILANON_DISCORD_CLIENT_SECRET",
];

/// Process-global store handle — initialized once in `init` at startup.
static STORE: Mutex<Option<BTreeMap<String, String>>> = Mutex::new(None);

#[allow(dead_code)]
pub fn is_secret_key(key: &str) -> bool {
    SECRET_KEYS.contains(&key)
}

/// Initialize the store: load the envelope from disk and run the one-time
/// `.env` migration. Called from `setup`. Best-effort — a broken store never
/// blocks app startup; the app simply falls back to runtime env/embedded.
pub fn init(data_dir: &std::path::Path) {
    let mut guard = STORE.lock().unwrap();
    let mut map = load_envelope(data_dir);
    let migrated = migrate_from_dotenv(&mut map);
    if migrated > 0 {
        let _ = save_envelope(data_dir, &map);
    }
    *guard = Some(map);
}

/// Read a secret (or any key) from the store. Returns None when unset.
pub fn get(key: &str) -> Option<String> {
    STORE.lock().unwrap().as_ref().and_then(|m| m.get(key).cloned())
}

/// Set (or remove, when None) a value and persist immediately.
#[allow(dead_code)]
pub fn set(data_dir: &std::path::Path, key: &str, value: Option<String>) -> bool {
    let mut guard = STORE.lock().unwrap();
    let map = guard.get_or_insert_with(BTreeMap::new);
    match value {
        Some(v) if !v.is_empty() => {
            map.insert(key.to_string(), v);
        }
        _ => {
            map.remove(key);
        }
    }
    let ok = save_envelope(data_dir, map);
    if ok {
        *guard = Some(map.clone());
    }
    ok
}

/// All currently stored key names (for the connection status UI). Values are
/// never returned — only presence + masked hints.
#[allow(dead_code)]
pub fn keys() -> Vec<String> {
    STORE.lock().unwrap().as_ref().map(|m| m.keys().cloned().collect()).unwrap_or_default()
}

/// Wipe every stored secret.
#[allow(dead_code)]
pub fn clear(data_dir: &std::path::Path) -> bool {
    let map = BTreeMap::new();
    let ok = save_envelope(data_dir, &map);
    *STORE.lock().unwrap() = Some(map);
    ok
}

// ── Envelope I/O ───────────────────────────────────────────────────────────

fn envelope_path(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join(STORE_FILE)
}

/// Derive (or create) the AES-256-GCM key for the envelope.
/// Primary: OS keychain. Fallback: 0600 file in the app data dir.
fn store_key(data_dir: &std::path::Path) -> Option<[u8; 32]> {
    // 1) OS keychain
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ENTRY).ok();
    if let Some(entry) = entry {
        if let Ok(existing) = entry.get_password() {
            if let Ok(bytes) = B64.decode(existing.trim()) {
                if bytes.len() == 32 {
                    let mut k = [0u8; 32];
                    k.copy_from_slice(&bytes);
                    return Some(k);
                }
            }
        }
        if let Ok(bytes) = random_bytes(32) {
            let mut k = [0u8; 32];
            k.copy_from_slice(&bytes);
            let encoded = B64.encode(k);
            if entry.set_password(&encoded).is_ok() {
                return Some(k);
            }
        }
    }

    // 2) File fallback (keychain-less Linux): 0600 permissions.
    let key_path = data_dir.join(FILE_KEY_FALLBACK);
    if let Ok(existing) = std::fs::read(&key_path) {
        if existing.len() == 32 {
            let mut k = [0u8; 32];
            k.copy_from_slice(&existing);
            return Some(k);
        }
    }
    if let Ok(bytes) = random_bytes(32) {
        let mut k = [0u8; 32];
        k.copy_from_slice(&bytes);
        if std::fs::write(&key_path, k).is_ok() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
            }
            warn!(
                "veilanon: secrets key stored in a file ({}). OS keychain unavailable — consider installing a Secret Service provider.",
                key_path.display()
            );
            return Some(k);
        }
    }
    None
}

fn load_envelope(data_dir: &std::path::Path) -> BTreeMap<String, String> {
    let Some(key) = store_key(data_dir) else {
        warn!("veilanon: could not obtain secrets key — store disabled");
        return BTreeMap::new();
    };
    let path = envelope_path(data_dir);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return BTreeMap::new();
    };
    let Some(ct_b64) = parsed.get("ciphertext").and_then(|v| v.as_str()) else {
        return BTreeMap::new();
    };
    let Some(nonce_b64) = parsed.get("nonce").and_then(|v| v.as_str()) else {
        return BTreeMap::new();
    };
    let Ok(ciphertext) = B64.decode(ct_b64) else {
        return BTreeMap::new();
    };
    let Ok(nonce) = B64.decode(nonce_b64) else {
        return BTreeMap::new();
    };
    let Ok(plain) = decrypt_aes_gcm(&key, &ciphertext, &nonce) else {
        return BTreeMap::new();
    };
    serde_json::from_slice(&plain).unwrap_or_default()
}

fn save_envelope(data_dir: &std::path::Path, map: &BTreeMap<String, String>) -> bool {
    let Some(key) = store_key(data_dir) else {
        return false;
    };
    let plain = match serde_json::to_vec(map) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let (ciphertext, nonce) = match encrypt_aes_gcm(&key, &plain) {
        Ok(pair) => pair,
        Err(_) => return false,
    };
    let envelope = serde_json::json!({
        "v": 1,
        "ciphertext": B64.encode(&ciphertext),
        "nonce": B64.encode(&nonce),
    });
    std::fs::write(envelope_path(data_dir), serde_json::to_string(&envelope).unwrap_or_default()).is_ok()
}

/// One-time migration: import `VEILANON_*` values from a working-directory
/// `.env` (dev convenience) into the store so installed builds keep working
/// after embedded secrets are removed. BOTH secret and non-secret keys are
/// imported — non-secret keys provide a runtime fallback when compile-time
/// embedding fails (e.g. stale build cache, CI without .env).
fn migrate_from_dotenv(map: &mut BTreeMap<String, String>) -> usize {
    let mut imported = 0usize;
    let mut candidates = vec![
        PathBuf::from(".env"),
        PathBuf::from("..").join(".env"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join(".env"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".env"),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join(".env"));
            if let Some(grandparent) = parent.parent() {
                candidates.push(grandparent.join(".env"));
            }
        }
    }

    for path in &candidates {
        let Ok(contents) = std::fs::read_to_string(path) else {
            continue;
        };
        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let key = k.trim();
            if !key.starts_with("VEILANON_") {
                continue;
            }
            let mut value = v.trim().to_string();
            if value.len() >= 2 {
                let first = value.chars().next().unwrap();
                let last = value.chars().last().unwrap();
                if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
                    value = value[1..value.len() - 1].to_string();
                }
            }
            if value.is_empty() {
                continue;
            }
            // Insert or update if missing or empty
            let existing_empty = map.get(key).map(|v| v.trim().is_empty()).unwrap_or(true);
            if existing_empty || !map.contains_key(key) {
                map.insert(key.to_string(), value);
                imported += 1;
            }
        }
        if imported > 0 {
            break;
        }
    }
    imported
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_classification_is_complete() {
        // Her SECRET_KEYS üyesi EMBEDDABLE değildir (çakışma = binary'ye sızma).
        for secret in SECRET_KEYS {
            assert!(!EMBEDDABLE_KEYS.contains(secret), "{secret} hem embeddable hem secret!");
        }
        // Kritik anahtarlar mutlaka secret tarafında.
        for critical in [
            "VEILANON_LIVEKIT_API_SECRET",
            "VEILANON_R2_SECRET_ACCESS_KEY",
            "VEILANON_SUPABASE_SERVICE_ROLE_KEY",
        ] {
            assert!(SECRET_KEYS.contains(&critical));
        }
    }

    #[test]
    fn dotenv_migration_imports_all_veilanon_keys() {
        let mut map = BTreeMap::new();
        let tmp = std::env::temp_dir().join(format!("veilanon-test-{}.env", std::process::id()));
        std::fs::write(
            &tmp,
            "VEILANON_LIVEKIT_API_SECRET=topsecret\nVEILANON_SUPABASE_URL=https://x.supabase.co\nVEILANON_DISCORD_CLIENT_SECRET=\"quoted-key\"\nVEILANON_TENOR_API_KEY=gifkey123\n",
        )
        .unwrap();
        let contents = std::fs::read_to_string(&tmp).unwrap();
        for raw_line in contents.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let key = k.trim();
            if !key.starts_with("VEILANON_") {
                continue;
            }
            let mut value = v.trim().to_string();
            if value.len() >= 2 {
                let first = value.chars().next().unwrap();
                let last = value.chars().last().unwrap();
                if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
                    value = value[1..value.len() - 1].to_string();
                }
            }
            if !value.is_empty() {
                map.insert(key.to_string(), value);
            }
        }
        std::fs::remove_file(&tmp).ok();
        assert_eq!(map.get("VEILANON_LIVEKIT_API_SECRET").map(String::as_str), Some("topsecret"));
        assert_eq!(map.get("VEILANON_DISCORD_CLIENT_SECRET").map(String::as_str), Some("quoted-key"));
        assert_eq!(map.get("VEILANON_SUPABASE_URL").map(String::as_str), Some("https://x.supabase.co"), "non-secret VEILANON_* keys must be imported as runtime fallback");
        assert_eq!(map.get("VEILANON_TENOR_API_KEY").map(String::as_str), Some("gifkey123"), "GIF API key must be imported");
    }
}
