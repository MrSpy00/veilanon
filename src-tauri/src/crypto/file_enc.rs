//! File encryption
//! 
//! Files are encrypted client-side with a random per-file content key.
//! The content key itself is encrypted with the message key and stored
//! alongside the AttachmentRef. The server only ever sees ciphertext blobs.
//! 
//! Encryption: ChaCha20-Poly1305 (streaming-friendly, authenticated)
//! Key: 256-bit random per-file key
//! Max chunk size: 64 KiB for streaming uploads

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Key, Nonce,
};
use crate::error::{VeilError, VeilResult};
use crate::crypto::random_bytes;
use zeroize::Zeroize;

#[allow(dead_code)] // streaming uploads land in a later iteration
pub const CHUNK_SIZE: usize = 65536; // 64 KiB
pub const KEY_SIZE: usize = 32;
pub const NONCE_SIZE: usize = 12;

/// Encrypt a file in memory (for small files < 100MB)
/// Returns (ciphertext, content_key, nonce)
pub fn encrypt_file(plaintext: &[u8]) -> VeilResult<EncryptedFile> {
    let key_bytes = random_bytes(KEY_SIZE)?;
    let nonce_bytes = random_bytes(NONCE_SIZE)?;
    
    let key = Key::from_slice(&key_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let cipher = ChaCha20Poly1305::new(key);
    
    let ciphertext = cipher.encrypt(nonce, plaintext)
        .map_err(|_| VeilError::EncryptionError)?;
    
    Ok(EncryptedFile {
        ciphertext,
        content_key: key_bytes,
        nonce: nonce_bytes,
    })
}

/// Decrypt a file
#[allow(dead_code)] // used by download_file once storage lands
pub fn decrypt_file(
    ciphertext: &[u8],
    content_key: &[u8],
    nonce: &[u8],
) -> VeilResult<Vec<u8>> {
    if content_key.len() != KEY_SIZE || nonce.len() != NONCE_SIZE {
        return Err(VeilError::DecryptionError);
    }
    let key = Key::from_slice(content_key);
    let nonce = Nonce::from_slice(nonce);
    let cipher = ChaCha20Poly1305::new(key);
    
    cipher.decrypt(nonce, ciphertext)
        .map_err(|_| VeilError::DecryptionError)
}

/// Result of file encryption — content_key is zeroized on drop
pub struct EncryptedFile {
    #[allow(dead_code)] // consumed by the upload path once storage lands
    pub ciphertext: Vec<u8>,
    pub content_key: Vec<u8>,
    #[allow(dead_code)] // consumed by the upload path once storage lands
    pub nonce: Vec<u8>,
}

impl Drop for EncryptedFile {
    fn drop(&mut self) {
        self.content_key.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_enc_empty_payload_roundtrip() {
        let plaintext = b"";
        let encrypted = encrypt_file(plaintext).expect("encryption of empty payload should succeed");
        assert_eq!(encrypted.ciphertext.len(), 16, "empty plaintext ciphertext must contain 16-byte Poly1305 tag");
        assert_eq!(encrypted.content_key.len(), KEY_SIZE);
        assert_eq!(encrypted.nonce.len(), NONCE_SIZE);

        let decrypted = decrypt_file(&encrypted.ciphertext, &encrypted.content_key, &encrypted.nonce)
            .expect("decryption of empty payload should succeed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn file_enc_small_payload_roundtrip() {
        let plaintext = b"VeilAnon zero-knowledge authenticated file encryption payload.";
        let encrypted = encrypt_file(plaintext).expect("encryption should succeed");
        assert_eq!(encrypted.ciphertext.len(), plaintext.len() + 16);

        let decrypted = decrypt_file(&encrypted.ciphertext, &encrypted.content_key, &encrypted.nonce)
            .expect("decryption should succeed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn file_enc_large_and_chunk_boundary_payload_roundtrip() {
        // 1. Exact 64 KiB chunk boundary
        let chunk_data = vec![0x5A; CHUNK_SIZE];
        let encrypted_chunk = encrypt_file(&chunk_data).expect("chunk encryption should succeed");
        assert_eq!(encrypted_chunk.ciphertext.len(), CHUNK_SIZE + 16);
        let decrypted_chunk = decrypt_file(&encrypted_chunk.ciphertext, &encrypted_chunk.content_key, &encrypted_chunk.nonce)
            .expect("chunk decryption should succeed");
        assert_eq!(decrypted_chunk, chunk_data);

        // 2. Multi-chunk 256 KiB payload
        let large_data = vec![0xA5; CHUNK_SIZE * 4];
        let encrypted_large = encrypt_file(&large_data).expect("large payload encryption should succeed");
        assert_eq!(encrypted_large.ciphertext.len(), (CHUNK_SIZE * 4) + 16);
        let decrypted_large = decrypt_file(&encrypted_large.ciphertext, &encrypted_large.content_key, &encrypted_large.nonce)
            .expect("large payload decryption should succeed");
        assert_eq!(decrypted_large, large_data);
    }

    #[test]
    fn file_enc_corrupted_ciphertext_fails() {
        let plaintext = b"Confidential financial and identity records";
        let encrypted = encrypt_file(plaintext).expect("encryption should succeed");
        let mut corrupted = encrypted.ciphertext.clone();
        corrupted[0] ^= 0x01; // flip single bit in ciphertext body

        let res = decrypt_file(&corrupted, &encrypted.content_key, &encrypted.nonce);
        assert!(res.is_err(), "decryption of corrupted ciphertext must return error");
    }

    #[test]
    fn file_enc_corrupted_auth_tag_fails() {
        let plaintext = b"Confidential financial and identity records";
        let encrypted = encrypt_file(plaintext).expect("encryption should succeed");
        let mut corrupted = encrypted.ciphertext.clone();
        let last_idx = corrupted.len() - 1;
        corrupted[last_idx] ^= 0x01; // flip single bit in Poly1305 MAC tag

        let res = decrypt_file(&corrupted, &encrypted.content_key, &encrypted.nonce);
        assert!(res.is_err(), "decryption with altered MAC tag must return error");
    }

    #[test]
    fn file_enc_wrong_key_fails() {
        let plaintext = b"Top secret file payload";
        let encrypted = encrypt_file(plaintext).expect("encryption should succeed");
        let wrong_key = vec![0xFF; KEY_SIZE];

        let res = decrypt_file(&encrypted.ciphertext, &wrong_key, &encrypted.nonce);
        assert!(res.is_err(), "decryption with wrong key must fail");
    }

    #[test]
    fn file_enc_invalid_key_length_rejected() {
        let plaintext = b"Test invalid key lengths";
        let encrypted = encrypt_file(plaintext).expect("encryption should succeed");

        let short_key = vec![0u8; 16];
        let long_key = vec![0u8; 64];

        assert!(decrypt_file(&encrypted.ciphertext, &short_key, &encrypted.nonce).is_err());
        assert!(decrypt_file(&encrypted.ciphertext, &long_key, &encrypted.nonce).is_err());
        assert!(decrypt_file(&encrypted.ciphertext, &[], &encrypted.nonce).is_err());
    }

    #[test]
    fn file_enc_corrupted_and_invalid_nonce_fails() {
        let plaintext = b"Test nonce tampering and length validation";
        let encrypted = encrypt_file(plaintext).expect("encryption should succeed");

        // 1. Bit flip in nonce
        let mut tampered_nonce = encrypted.nonce.clone();
        tampered_nonce[0] ^= 0x80;
        assert!(decrypt_file(&encrypted.ciphertext, &encrypted.content_key, &tampered_nonce).is_err());

        // 2. Nonce length validation
        let short_nonce = vec![0u8; 8];
        let long_nonce = vec![0u8; 16];
        assert!(decrypt_file(&encrypted.ciphertext, &encrypted.content_key, &short_nonce).is_err());
        assert!(decrypt_file(&encrypted.ciphertext, &encrypted.content_key, &long_nonce).is_err());
        assert!(decrypt_file(&encrypted.ciphertext, &encrypted.content_key, &[]).is_err());
    }

    #[test]
    fn file_enc_generates_unique_keys_and_nonces() {
        let plaintext = b"Identical plaintext for both encryptions";
        let enc1 = encrypt_file(plaintext).expect("first enc");
        let enc2 = encrypt_file(plaintext).expect("second enc");

        assert_ne!(enc1.content_key, enc2.content_key, "subsequent encryptions must generate fresh keys");
        assert_ne!(enc1.nonce, enc2.nonce, "subsequent encryptions must generate fresh nonces");
        assert_ne!(enc1.ciphertext, enc2.ciphertext, "ciphertexts must be distinct due to unique nonces/keys");
    }
}
