//! User and identity models
//! SECURITY: identity_key_public is the only key ever serialized for IPC.
//! Private keys NEVER appear in model structs that cross the IPC boundary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeroize::Zeroize;

/// User identity — stored locally, minimal server-side footprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_hash: Option<String>,
    pub banner_hash: Option<String>,
    /// X25519 public key (hex-encoded) - safe for IPC
    pub identity_key_public: String,
    /// Ed25519 signing public key (hex-encoded) — safe for IPC
    pub signing_key_public: String,
    pub created_at: DateTime<Utc>,
    pub device_id: Uuid,
}

/// Device registration record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // session list UI lands next
pub struct Device {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub platform: String,
    pub public_key: String,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub is_current: bool,
}

/// Public user profile (what others see)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // profile UI lands next
pub struct UserProfile {
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_hash: Option<String>,
    pub identity_key_public: String,
    pub bio: Option<String>,
    /// Presence visibility is controlled by privacy settings
    pub online_status: OnlineStatus,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)] // presence UI lands next
pub enum OnlineStatus {
    /// Only visible to contacts per privacy setting
    Online,
    Away,
    DoNotDisturb,
    Offline,
    /// Privacy mode — appears offline to everyone
    Invisible,
}

/// Sensitive key material — NEVER crosses IPC boundary
/// Always zeroized on drop
#[derive(Zeroize)]
#[zeroize(drop)]
#[allow(dead_code)] // session key holder lands next
pub struct KeyMaterial {
    pub identity_key_private: Vec<u8>,
    pub signing_key_private: Vec<u8>,
    pub db_encryption_key: Vec<u8>,
}
