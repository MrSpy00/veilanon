//! Identity key management
//! 
//! Each device generates:
//! - X25519 keypair for Diffie-Hellman key agreement
//! - Ed25519 keypair for signing
//! 
//! Private keys NEVER leave this module. Public keys are safe to export.

use ed25519_dalek::{SigningKey, VerifyingKey, Signer, Verifier, Signature};
use x25519_dalek::{StaticSecret, PublicKey as X25519PublicKey};
use zeroize::Zeroize;
use rand::rngs::OsRng;
use crate::error::{VeilError, VeilResult};

/// Device identity keypairs — private keys are zeroized on drop
#[derive(Zeroize, Clone)]
#[zeroize(drop)]
pub struct DeviceIdentity {
    /// X25519 private key for DH key agreement
    dh_private: [u8; 32],
    /// Ed25519 signing private key
    signing_private: [u8; 32],
}

/// Public portion of device identity — safe for IPC and transmission
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DevicePublicIdentity {
    pub dh_public_key: String,    // hex-encoded X25519 public key
    pub signing_public_key: String, // hex-encoded Ed25519 verifying key
    pub fingerprint: String,       // SHA-256 of both public keys
}

impl DeviceIdentity {
    /// Generate a new random device identity
    pub fn generate() -> VeilResult<Self> {
        let dh_secret = StaticSecret::random_from_rng(OsRng);
        let signing_key = SigningKey::generate(&mut OsRng);
        
        Ok(Self {
            dh_private: dh_secret.to_bytes(),
            signing_private: signing_key.to_bytes(),
        })
    }

    /// Restore from stored bytes (called after decrypting from keystore)
    pub fn from_bytes(dh_private: [u8; 32], signing_private: [u8; 32]) -> Self {
        Self { dh_private, signing_private }
    }

    /// Create deterministic device identity from 32-byte seed for DH and 32-byte seed for signing
    pub fn from_seeds(dh_seed: [u8; 32], signing_seed: [u8; 32]) -> Self {
        let dh_secret = StaticSecret::from(dh_seed);
        let signing_key = SigningKey::from_bytes(&signing_seed);
        Self {
            dh_private: dh_secret.to_bytes(),
            signing_private: signing_key.to_bytes(),
        }
    }

    /// Export private key bytes for encrypted storage (call only during save)
    pub fn export_private_bytes(&self) -> ([u8; 32], [u8; 32]) {
        (self.dh_private, self.signing_private)
    }

    /// Raw X25519 private key — consumed by the DM ratchet for the initial
    /// key agreement; the ratchet rotates to fresh keys immediately after.
    pub fn dh_private_bytes(&self) -> [u8; 32] {
        self.dh_private
    }

    /// Get the public identity (safe for IPC)
    pub fn public_identity(&self) -> VeilResult<DevicePublicIdentity> {
        let dh_secret = StaticSecret::from(self.dh_private);
        let dh_public = X25519PublicKey::from(&dh_secret);
        
        let signing_key = SigningKey::from_bytes(&self.signing_private);
        let verifying_key = signing_key.verifying_key();
        
        let dh_pub_bytes = dh_public.as_bytes();
        let sign_pub_bytes = verifying_key.as_bytes();
        
        // Fingerprint = hex(SHA-256(dh_pub || sign_pub))
        use ring::digest;
        let mut input = Vec::with_capacity(64);
        input.extend_from_slice(dh_pub_bytes);
        input.extend_from_slice(sign_pub_bytes);
        let hash = digest::digest(&digest::SHA256, &input);
        
        Ok(DevicePublicIdentity {
            dh_public_key: hex::encode(dh_pub_bytes),
            signing_public_key: hex::encode(sign_pub_bytes),
            fingerprint: hex::encode(hash.as_ref()),
        })
    }

    /// Perform X25519 DH with peer's public key — result is used for key derivation
    pub fn dh_agree(&self, peer_public_key_hex: &str) -> VeilResult<[u8; 32]> {
        let peer_bytes = hex::decode(peer_public_key_hex).map_err(|_| VeilError::CryptoError)?;
        if peer_bytes.len() != 32 {
            return Err(VeilError::CryptoError);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&peer_bytes);
        let peer_public = X25519PublicKey::from(arr);
        let dh_secret = StaticSecret::from(self.dh_private);
        let shared = dh_secret.diffie_hellman(&peer_public);
        Ok(*shared.as_bytes())
    }

    /// Sign a message — returns hex-encoded signature
    pub fn sign(&self, message: &[u8]) -> VeilResult<String> {
        let signing_key = SigningKey::from_bytes(&self.signing_private);
        let signature = signing_key.sign(message);
        Ok(hex::encode(signature.to_bytes()))
    }
}

/// SHA-256 fingerprint over both public keys (hex, grouped later by the UI).
pub fn fingerprint_for_keys(dh_public_hex: &str, signing_public_hex: &str) -> String {
    use ring::digest;
    let dh = hex::decode(dh_public_hex).unwrap_or_default();
    let sign = hex::decode(signing_public_hex).unwrap_or_default();
    let mut input = Vec::with_capacity(dh.len() + sign.len());
    input.extend_from_slice(&dh);
    input.extend_from_slice(&sign);
    hex::encode(digest::digest(&digest::SHA256, &input).as_ref())
}

/// Verify a signature from a known public key
pub fn verify_signature(
    verifying_key_hex: &str,
    message: &[u8],
    signature_hex: &str,
) -> VeilResult<bool> {
    let key_bytes = hex::decode(verifying_key_hex).map_err(|_| VeilError::SignatureError)?;
    let sig_bytes = hex::decode(signature_hex).map_err(|_| VeilError::SignatureError)?;
    
    if key_bytes.len() != 32 || sig_bytes.len() != 64 {
        return Err(VeilError::SignatureError);
    }
    
    let key_arr: [u8; 32] = key_bytes.try_into().unwrap();
    let verifying_key = VerifyingKey::from_bytes(&key_arr)
        .map_err(|_| VeilError::SignatureError)?;
    
    let sig_arr: [u8; 64] = sig_bytes.try_into().unwrap();
    let signature = Signature::from_bytes(&sig_arr);
    
    Ok(verifying_key.verify(message, &signature).is_ok())
}
