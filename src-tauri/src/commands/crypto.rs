//! Crypto IPC commands — public key operations only
//! Private keys NEVER cross the IPC boundary.

use tauri::State;
use serde::{Deserialize, Serialize};
use crate::state::AppState;
use crate::error::VeilError;
use crate::crypto::identity;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyPairResponse {
    pub public_key: String,
    pub key_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignMessageInput {
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifySignatureInput {
    pub message: String,
    pub signature: String,
    pub public_key: String,
}

/// Generate a new ephemeral keypair (for protocol use — NOT stored)
#[tauri::command]
pub async fn generate_keypair(key_type: Option<String>) -> Result<KeyPairResponse, VeilError> {
    let key_type = key_type.unwrap_or_else(|| "x25519".to_string());
    match key_type.as_str() {
        "x25519" => {
            let id = identity::DeviceIdentity::generate()?;
            let public = id.public_identity()?;
            Ok(KeyPairResponse {
                public_key: public.dh_public_key,
                key_type: "x25519".into(),
            })
        }
        _ => Err(VeilError::InvalidInput("Unsupported key type".into())),
    }
}

/// Sign a message with the device signing key (hex-encoded message in,
/// hex-encoded Ed25519 signature out).
#[tauri::command]
pub async fn sign_message(
    input: SignMessageInput,
    state: State<'_, AppState>,
) -> Result<String, VeilError> {
    let msg_bytes = hex::decode(&input.message).map_err(|_| VeilError::InvalidInput("Invalid hex".into()))?;

    let device = state.device_identity.read().await;
    let device = device.as_ref().ok_or(VeilError::Unauthenticated)?;
    device.sign(&msg_bytes)
}

/// Verify a signature from a known public key
#[tauri::command]
pub async fn verify_signature(
    input: VerifySignatureInput,
) -> Result<bool, VeilError> {
    let msg_bytes = hex::decode(&input.message).map_err(|_| VeilError::InvalidInput("Invalid hex".into()))?;
    identity::verify_signature(&input.public_key, &msg_bytes, &input.signature)
}

/// Get the current device's public key
#[tauri::command]
pub async fn get_public_key(state: State<'_, AppState>) -> Result<String, VeilError> {
    let identity = state.get_or_restore_identity().await;
    let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
    Ok(identity.identity_key_public.clone())
}

/// Get fingerprint for verification UI
#[tauri::command]
pub async fn fingerprint(key_hex: Option<String>, state: State<'_, AppState>) -> Result<String, VeilError> {
    let key_hex = match key_hex {
        Some(key) => key,
        None => {
            let identity = state.get_or_restore_identity().await;
            let identity = identity.as_ref().ok_or(VeilError::Unauthenticated)?;
            identity.identity_key_public.clone()
        }
    };
    let key_bytes = hex::decode(&key_hex).map_err(|_| VeilError::InvalidInput("Invalid key".into()))?;
    use ring::digest;
    let hash = digest::digest(&digest::SHA256, &key_bytes);
    // Format as groups of 8 hex chars for readability
    let hex_str = hex::encode(hash.as_ref());
    let formatted = hex_str
        .chars()
        .collect::<Vec<char>>()
        .chunks(8)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(" ");
    Ok(formatted)
}
