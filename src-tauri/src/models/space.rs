//! Space (community/server) and Role models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A community space (analogous to a Discord server)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // full model used once sync lands
pub struct Space {
    pub id: Uuid,
    pub name: String,
    pub icon_hash: Option<String>,
    pub owner_id: Uuid,
    pub description: Option<String>,
    pub member_count: u32,
    pub roles: Vec<Role>,
    pub created_at: DateTime<Utc>,
    pub is_owner: bool,
    pub my_roles: Vec<Uuid>,
}

/// Role within a space
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // full model used once sync lands
pub struct Role {
    pub id: Uuid,
    pub space_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub permissions: Permissions,
    pub position: i32,
    pub is_default: bool,
}

/// Permission bitfield — defines what a role can do
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    // Text permissions
    pub send_messages: bool,
    pub read_messages: bool,
    pub manage_messages: bool,
    pub embed_links: bool,
    pub attach_files: bool,
    pub add_reactions: bool,
    pub use_slash_commands: bool,
    pub mention_everyone: bool,
    pub pin_messages: bool,
    // Voice permissions
    pub connect_voice: bool,
    pub speak: bool,
    pub mute_members: bool,
    pub deafen_members: bool,
    pub move_members: bool,
    pub use_voice_activity: bool,
    pub stream_video: bool,
    pub share_screen: bool,
    pub priority_speaker: bool,
    // Channel management
    pub manage_channels: bool,
    pub manage_roles: bool,
    pub manage_webhooks: bool,
    pub manage_invites: bool,
    // Member management
    pub kick_members: bool,
    pub ban_members: bool,
    pub timeout_members: bool,
    pub view_audit_log: bool,
    // Space management
    pub manage_space: bool,
    pub administrator: bool, // Overrides all
}

const PERMISSION_IDS: [&str; 28] = [
    "administrator",
    "manage_space",
    "view_audit_log",
    "manage_roles",
    "manage_channels",
    "kick_members",
    "ban_members",
    "timeout_members",
    "manage_invites",
    "manage_webhooks",
    "send_messages",
    "read_messages",
    "manage_messages",
    "embed_links",
    "attach_files",
    "add_reactions",
    "use_slash_commands",
    "mention_everyone",
    "pin_messages",
    "connect_voice",
    "speak",
    "stream_video",
    "share_screen",
    "mute_members",
    "deafen_members",
    "move_members",
    "use_voice_activity",
    "priority_speaker",
];

impl Permissions {
    /// Wire format: sorted list of enabled permission ids (UI contract).
    pub fn enabled_ids(&self) -> Vec<String> {
        PERMISSION_IDS
            .iter()
            .filter(|id| self.has(id))
            .map(|id| id.to_string())
            .collect()
    }

    pub fn has(&self, id: &str) -> bool {
        if self.administrator {
            return true;
        }
        match id {
            "administrator" => self.administrator,
            "manage_space" => self.manage_space,
            "view_audit_log" => self.view_audit_log,
            "manage_roles" => self.manage_roles,
            "manage_channels" => self.manage_channels,
            "kick_members" => self.kick_members,
            "ban_members" => self.ban_members,
            "timeout_members" => self.timeout_members,
            "manage_invites" => self.manage_invites,
            "manage_webhooks" => self.manage_webhooks,
            "send_messages" => self.send_messages,
            "read_messages" => self.read_messages,
            "manage_messages" => self.manage_messages,
            "embed_links" => self.embed_links,
            "attach_files" => self.attach_files,
            "add_reactions" => self.add_reactions,
            "use_slash_commands" => self.use_slash_commands,
            "mention_everyone" => self.mention_everyone,
            "pin_messages" => self.pin_messages,
            "connect_voice" => self.connect_voice,
            "speak" => self.speak,
            "stream_video" => self.stream_video,
            "share_screen" => self.share_screen,
            "mute_members" => self.mute_members,
            "deafen_members" => self.deafen_members,
            "move_members" => self.move_members,
            "use_voice_activity" => self.use_voice_activity,
            "priority_speaker" => self.priority_speaker,
            _ => false,
        }
    }

    pub fn apply_ids(&mut self, ids: &[String]) {
        for id in ids {
            match id.as_str() {
                "administrator" => self.administrator = true,
                "manage_space" => self.manage_space = true,
                "view_audit_log" => self.view_audit_log = true,
                "manage_roles" => self.manage_roles = true,
                "manage_channels" => self.manage_channels = true,
                "kick_members" => self.kick_members = true,
                "ban_members" => self.ban_members = true,
                "timeout_members" => self.timeout_members = true,
                "manage_invites" => self.manage_invites = true,
                "manage_webhooks" => self.manage_webhooks = true,
                "send_messages" => self.send_messages = true,
                "read_messages" => self.read_messages = true,
                "manage_messages" => self.manage_messages = true,
                "embed_links" => self.embed_links = true,
                "attach_files" => self.attach_files = true,
                "add_reactions" => self.add_reactions = true,
                "use_slash_commands" => self.use_slash_commands = true,
                "mention_everyone" => self.mention_everyone = true,
                "pin_messages" => self.pin_messages = true,
                "connect_voice" => self.connect_voice = true,
                "speak" => self.speak = true,
                "stream_video" => self.stream_video = true,
                "share_screen" => self.share_screen = true,
                "mute_members" => self.mute_members = true,
                "deafen_members" => self.deafen_members = true,
                "move_members" => self.move_members = true,
                "use_voice_activity" => self.use_voice_activity = true,
                "priority_speaker" => self.priority_speaker = true,
                _ => {}
            }
        }
    }
}

/// Invite link
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // full model used once sync lands
pub struct Invite {
    pub id: Uuid,
    pub code: String,
    pub space_id: Uuid,
    pub creator_id: Uuid,
    pub role_id: Option<Uuid>,
    pub max_uses: Option<u32>,
    pub used_count: u32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    /// Whether this invite has channel-scope restriction
    pub channel_scope: Option<Uuid>,
}
