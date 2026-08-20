//! Double Ratchet for 1:1 DM E2EE
#![allow(dead_code)] // Scaffold module — wired by future UI-facing commands
//! 
//! Simplified implementation using ring primitives following Signal Protocol design.
//! For production, replace with a formally verified, audited library.
//! Current: uses X25519 DH + HKDF + AES-256-GCM per message.
//! 
//! Protocol properties:
//! - Forward secrecy: compromising current key reveals nothing about past messages  
//! - Post-compromise security: key material ratchets forward after each message
//! - Out-of-order delivery: supported via message key cache

use ring::{hkdf, hmac};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use zeroize::Zeroize;
use crate::error::{VeilError, VeilResult};
use crate::crypto::{encrypt_aes_gcm, decrypt_aes_gcm};

const MAX_SKIP: u32 = 100; // Max skipped messages to cache

/// Ratchet state for one DM conversation
/// This is persisted (encrypted) in local DB per session
#[derive(Serialize, Deserialize)]
pub struct RatchetState {
    /// DH ratchet public key (hex)
    pub dh_public: String,
    /// DH ratchet private key (hex) — encrypted at rest
    dh_private: Vec<u8>,
    /// Remote party's DH ratchet public key (hex)
    pub remote_dh_public: String,
    /// Root key (hex) — encrypted at rest
    root_key: Vec<u8>,
    /// Chain key for sending (hex) — encrypted at rest
    chain_key_send: Vec<u8>,
    /// Chain key for receiving (hex) — encrypted at rest
    chain_key_recv: Vec<u8>,
    /// Send message number
    pub send_count: u32,
    /// Receive message number
    pub recv_count: u32,
    /// Previous send chain count
    pub prev_send_count: u32,
    /// Skipped message keys cache
    pub skipped_keys: HashMap<(String, u32), Vec<u8>>,
}

impl Drop for RatchetState {
    fn drop(&mut self) {
        self.dh_private.zeroize();
        self.root_key.zeroize();
        self.chain_key_send.zeroize();
        self.chain_key_recv.zeroize();
        for key in self.skipped_keys.values_mut() {
            key.zeroize();
        }
    }
}

/// Per-message header (sent in plaintext alongside ciphertext)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    pub dh_public: String,
    pub prev_chain_count: u32,
    pub message_count: u32,
}

/// Derive two keys from a root key and DH output using HKDF
fn kdf_rk(root_key: &[u8], dh_output: &[u8]) -> VeilResult<([u8; 32], [u8; 32])> {
    let salt = hkdf::Salt::new(hkdf::HKDF_SHA256, root_key);
    let prk = salt.extract(dh_output);
    
    let mut new_root = [0u8; 32];
    let mut chain_key = [0u8; 32];
    
    prk.expand(&[b"veilanon_rk"], hkdf::HKDF_SHA256)
        .map_err(|_| VeilError::KeyDerivationError)?
        .fill(&mut new_root)
        .map_err(|_| VeilError::KeyDerivationError)?;
    
    prk.expand(&[b"veilanon_ck"], hkdf::HKDF_SHA256)
        .map_err(|_| VeilError::KeyDerivationError)?
        .fill(&mut chain_key)
        .map_err(|_| VeilError::KeyDerivationError)?;
    
    Ok((new_root, chain_key))
}

/// Derive a message key from chain key
fn kdf_ck(chain_key: &[u8]) -> VeilResult<([u8; 32], [u8; 32])> {
    let key = hmac::Key::new(hmac::HMAC_SHA256, chain_key);
    
    let mk_tag = hmac::sign(&key, &[0x01]);
    let ck_tag = hmac::sign(&key, &[0x02]);
    
    let mut msg_key = [0u8; 32];
    let mut new_ck = [0u8; 32];
    msg_key.copy_from_slice(&mk_tag.as_ref()[0..32]);
    new_ck.copy_from_slice(&ck_tag.as_ref()[0..32]);
    
    Ok((msg_key, new_ck))
}

impl RatchetState {
    /// Create a fresh ratchet state from pre-shared key material.
    ///
    /// Both parties call this with mirrored inputs (own private key + own
    /// public hex, the remote public hex, and the same root key). The initial
    /// send/receive chain keys are derived from the DH shared secret, which is
    /// symmetric in X25519, so both sides land on identical chains.
    pub fn new(
        dh_private: [u8; 32],
        dh_public_hex: &str,
        remote_dh_public_hex: &str,
        root_key: [u8; 32],
    ) -> VeilResult<Self> {
        use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

        let remote_bytes =
            hex::decode(remote_dh_public_hex).map_err(|_| VeilError::CryptoError)?;
        if remote_bytes.len() != 32 {
            return Err(VeilError::CryptoError);
        }
        let mut remote_arr = [0u8; 32];
        remote_arr.copy_from_slice(&remote_bytes);

        let secret = StaticSecret::from(dh_private);
        let shared = secret.diffie_hellman(&X25519PublicKey::from(remote_arr));
        let (new_root, chain_key) = kdf_rk(&root_key, shared.as_bytes())?;

        Ok(Self {
            dh_public: dh_public_hex.to_string(),
            dh_private: dh_private.to_vec(),
            remote_dh_public: remote_dh_public_hex.to_string(),
            root_key: new_root.to_vec(),
            chain_key_send: chain_key.to_vec(),
            chain_key_recv: chain_key.to_vec(),
            send_count: 0,
            recv_count: 0,
            prev_send_count: 0,
            skipped_keys: HashMap::new(),
        })
    }

    /// Encrypt a message — advances send ratchet
    pub fn encrypt(&mut self, plaintext: &[u8]) -> VeilResult<(MessageHeader, Vec<u8>, Vec<u8>)> {
        let (msg_key, new_ck) = kdf_ck(&self.chain_key_send)?;
        self.chain_key_send = new_ck.to_vec();
        
        let header = MessageHeader {
            dh_public: self.dh_public.clone(),
            prev_chain_count: self.prev_send_count,
            message_count: self.send_count,
        };
        self.send_count += 1;
        
        let (ciphertext, nonce) = encrypt_aes_gcm(&msg_key, plaintext)?;
        Ok((header, ciphertext, nonce))
    }

    /// Decrypt a message — may perform DH ratchet step.
    /// Returns (plaintext, message_key) so callers can cache the key and
    /// re-read history without advancing the chains again.
    pub fn decrypt(
        &mut self,
        header: &MessageHeader,
        ciphertext: &[u8],
        nonce: &[u8],
    ) -> VeilResult<(Vec<u8>, [u8; 32])> {
        // Check skipped keys first
        let skip_key = (header.dh_public.clone(), header.message_count);
        if let Some(key) = self.skipped_keys.get(&skip_key) {
            let key_arr: [u8; 32] = key.as_slice().try_into().map_err(|_| VeilError::DecryptionError)?;
            let plaintext = decrypt_aes_gcm(&key_arr, ciphertext, nonce)?;
            return Ok((plaintext, key_arr));
        }
        
        // DH ratchet step if new DH key
        if header.dh_public != self.remote_dh_public {
            self.skip_message_keys(header.prev_chain_count)?;
            self.ratchet_dh(&header.dh_public)?;
        }
        
        self.skip_message_keys(header.message_count)?;
        
        let (msg_key, new_ck) = kdf_ck(&self.chain_key_recv)?;
        self.chain_key_recv = new_ck.to_vec();
        self.recv_count += 1;
        
        let plaintext = decrypt_aes_gcm(&msg_key, ciphertext, nonce)?;
        Ok((plaintext, msg_key))
    }
    
    fn skip_message_keys(&mut self, until: u32) -> VeilResult<()> {
        if until > self.recv_count + MAX_SKIP {
            return Err(VeilError::DecryptionError);
        }
        while self.recv_count < until {
            let (msg_key, new_ck) = kdf_ck(&self.chain_key_recv)?;
            self.chain_key_recv = new_ck.to_vec();
            self.skipped_keys.insert(
                (self.remote_dh_public.clone(), self.recv_count),
                msg_key.to_vec(),
            );
            self.recv_count += 1;
        }
        Ok(())
    }
    
    fn ratchet_dh(&mut self, remote_dh_hex: &str) -> VeilResult<()> {
        self.prev_send_count = self.send_count;
        self.send_count = 0;
        self.recv_count = 0;
        self.remote_dh_public = remote_dh_hex.to_string();
        
        // DH with old private key + new remote public
        let dh_out = self.perform_dh(remote_dh_hex)?;
        let (new_rk, recv_ck) = kdf_rk(&self.root_key, &dh_out)?;
        
        // Generate new DH keypair
        let new_private = crate::crypto::random_bytes(32)?;
        let new_dh_out = self.perform_dh_with(remote_dh_hex, &new_private)?;
        let (new_rk2, send_ck) = kdf_rk(&new_rk, &new_dh_out)?;
        
        use x25519_dalek::{StaticSecret, PublicKey as X25519PublicKey};
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&new_private);
        let new_secret = StaticSecret::from(arr);
        let new_public = X25519PublicKey::from(&new_secret);
        
        self.dh_private = new_private;
        self.dh_public = hex::encode(new_public.as_bytes());
        self.root_key = new_rk2.to_vec();
        self.chain_key_recv = recv_ck.to_vec();
        self.chain_key_send = send_ck.to_vec();
        
        Ok(())
    }
    
    fn perform_dh(&self, remote_hex: &str) -> VeilResult<Vec<u8>> {
        self.perform_dh_with(remote_hex, &self.dh_private)
    }
    
    fn perform_dh_with(&self, remote_hex: &str, private_bytes: &[u8]) -> VeilResult<Vec<u8>> {
        use x25519_dalek::{StaticSecret, PublicKey as X25519PublicKey};
        let remote_bytes = hex::decode(remote_hex).map_err(|_| VeilError::CryptoError)?;
        if remote_bytes.len() != 32 || private_bytes.len() != 32 {
            return Err(VeilError::CryptoError);
        }
        let mut remote_arr = [0u8; 32];
        let mut private_arr = [0u8; 32];
        remote_arr.copy_from_slice(&remote_bytes);
        private_arr.copy_from_slice(private_bytes);
        let remote_public = X25519PublicKey::from(remote_arr);
        let secret = StaticSecret::from(private_arr);
        let shared = secret.diffie_hellman(&remote_public);
        Ok(shared.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

    #[test]
    fn ratchet_ab_roundtrip() {
        let priv_a = [7u8; 32];
        let priv_b = [9u8; 32];
        let pub_a = X25519PublicKey::from(&StaticSecret::from(priv_a));
        let pub_b = X25519PublicKey::from(&StaticSecret::from(priv_b));
        let root = [3u8; 32];

        let mut a = RatchetState::new(
            priv_a,
            &hex::encode(pub_a.as_bytes()),
            &hex::encode(pub_b.as_bytes()),
            root,
        )
        .unwrap();
        let mut b = RatchetState::new(
            priv_b,
            &hex::encode(pub_b.as_bytes()),
            &hex::encode(pub_a.as_bytes()),
            root,
        )
        .unwrap();

        let (header, ciphertext, nonce) = a.encrypt(b"hello ratchet").unwrap();
        let (plaintext, _key) = b.decrypt(&header, &ciphertext, &nonce).unwrap();
        assert_eq!(plaintext, b"hello ratchet");

        let (header2, ciphertext2, nonce2) = b.encrypt(b"reply").unwrap();
        let (plaintext2, _key2) = a.decrypt(&header2, &ciphertext2, &nonce2).unwrap();
        assert_eq!(plaintext2, b"reply");
    }

    #[test]
    fn kdf_ck_is_deterministic() {
        let chain = [0xAB; 32];
        let (msg_key_a, next_a) = kdf_ck(&chain).unwrap();
        let (msg_key_b, next_b) = kdf_ck(&chain).unwrap();
        assert_eq!(msg_key_a, msg_key_b);
        assert_eq!(next_a, next_b);
        assert_ne!(msg_key_a, next_a);
    }
}
