//! Application-layer at-rest encryption for sensitive columns
//! 
//! Plain SQLite has no native encryption. Every sensitive column value is
//! individually encrypted with AES-256-GCM using the DB key (derived from
//! the user's master key, held in memory only while authenticated).
//! 
//! Stored format per column: two base64 strings — ciphertext (with GCM tag)
//! and its 12-byte nonce. Nothing else is stored in plaintext.

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::crypto::{decrypt_aes_gcm, encrypt_aes_gcm};
use crate::error::{VeilError, VeilResult};

/// Encrypt `data` with `key`.
/// Returns `(ciphertext_b64, nonce_b64)` — both stored in the respective columns.
pub fn encrypt(key: &[u8; 32], data: &[u8]) -> VeilResult<(String, String)> {
    let (ciphertext, nonce) = encrypt_aes_gcm(key, data)?;
    Ok((B64.encode(&ciphertext), B64.encode(&nonce)))
}

/// Decrypt `(ciphertext_b64, nonce_b64)` with `key`.
pub fn decrypt(key: &[u8; 32], ciphertext_b64: &str, nonce_b64: &str) -> VeilResult<Vec<u8>> {
    let ciphertext = B64.decode(ciphertext_b64).map_err(|_| VeilError::DecryptionError)?;
    let nonce = B64.decode(nonce_b64).map_err(|_| VeilError::DecryptionError)?;
    if nonce.len() != 12 {
        return Err(VeilError::DecryptionError);
    }
    decrypt_aes_gcm(key, &ciphertext, &nonce)
}

/// Decrypt an optional column pair; `None`/empty columns decrypt to `None`.
#[allow(dead_code)] // profile bio decryption lands next
pub fn decrypt_optional(
    key: &[u8; 32],
    ciphertext_b64: Option<&str>,
    nonce_b64: Option<&str>,
) -> VeilResult<Option<Vec<u8>>> {
    match (ciphertext_b64, nonce_b64) {
        (Some(ct), Some(nonce)) if !ct.is_empty() => decrypt(key, ct, nonce).map(Some),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cipher_roundtrip() {
        let key = [0x5A; 32];
        let (ciphertext_b64, nonce_b64) = encrypt(&key, b"db secret").unwrap();
        let plaintext = decrypt(&key, &ciphertext_b64, &nonce_b64).unwrap();
        assert_eq!(plaintext, b"db secret");
    }

    #[test]
    fn cipher_wrong_key_fails() {
        let key = [0x11; 32];
        let other = [0x22; 32];
        let (ciphertext_b64, nonce_b64) = encrypt(&key, b"x").unwrap();
        assert!(decrypt(&other, &ciphertext_b64, &nonce_b64).is_err());
    }
}
