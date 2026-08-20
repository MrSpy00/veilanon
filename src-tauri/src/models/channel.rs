//! Channel models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::space::Permissions;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub id: Uuid,
    pub space_id: Option<Uuid>, // None for DM channels
    pub name: String,
    pub channel_type: ChannelType,
    pub position: i32,
    pub topic: Option<String>,
    pub is_nsfw: bool,
    pub is_e2ee: bool,
    pub slow_mode_seconds: u32,
    pub permission_overrides: Vec<PermissionOverride>,
    pub created_at: DateTime<Utc>,
    pub last_message_id: Option<Uuid>,
    pub unread_count: u32,
    pub mentioned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelType {
    Text,
    Voice,
    Announcement,
    Forum,
    DirectMessage,
    GroupDirectMessage,
    Category,
}

impl ChannelType {
    pub fn to_db_string(&self) -> String {
        match self {
            ChannelType::Text => "text".into(),
            ChannelType::Voice => "voice".into(),
            ChannelType::Announcement => "announcement".into(),
            ChannelType::Forum => "forum".into(),
            ChannelType::DirectMessage => "dm".into(),
            ChannelType::GroupDirectMessage => "group_dm".into(),
            ChannelType::Category => "category".into(),
        }
    }

    pub fn from_db_string(s: &str) -> Self {
        match s {
            "voice" => ChannelType::Voice,
            "announcement" => ChannelType::Announcement,
            "forum" => ChannelType::Forum,
            "dm" | "direct_message" => ChannelType::DirectMessage,
            "group_dm" | "group_direct_message" => ChannelType::GroupDirectMessage,
            "category" => ChannelType::Category,
            _ => ChannelType::Text,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOverride {
    pub target_id: Uuid,
    pub target_type: OverrideTarget,
    pub allow: Permissions,
    pub deny: Permissions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OverrideTarget {
    Role,
    Member,
}

/// Voice channel state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(dead_code)] // voice state UI lands next
pub struct VoiceState {
    pub channel_id: Uuid,
    pub participants: Vec<VoiceParticipant>,
    pub is_e2ee: bool,
    pub e2ee_key_epoch: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // voice state UI lands next
pub struct VoiceParticipant {
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub display_name: String,
    pub avatar_hash: Option<String>,
    pub is_muted: bool,
    pub is_deafened: bool,
    pub is_video_on: bool,
    pub is_screen_sharing: bool,
    pub is_speaking: bool,
    pub joined_at: DateTime<Utc>,
}
