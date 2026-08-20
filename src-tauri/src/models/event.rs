//! Event models for real-time sync
#![allow(dead_code)] // Scaffold module — wired by future UI-facing commands
//! Events carry no plaintext content — only encrypted payloads and routing metadata.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_id: Uuid,
    pub event_type: EventType,
    pub space_id: Option<Uuid>,
    pub channel_id: Option<Uuid>,
    pub actor_device_id: Uuid,
    /// Encrypted payload (base64) — server cannot read content
    pub payload_ciphertext: Option<String>,
    pub payload_iv: Option<String>,
    pub server_received_at: DateTime<Utc>,
    pub client_created_at: DateTime<Utc>,
    pub schema_version: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Message events
    MessageCreated,
    MessageEdited,
    MessageDeleted,
    MessageReactionAdded,
    MessageReactionRemoved,
    MessagePinned,
    MessageUnpinned,
    // Typing events (ephemeral, not stored)
    TypingStart,
    TypingStop,
    // Presence events (ephemeral)
    PresenceUpdate,
    // Voice events
    VoiceStateUpdate,
    VoiceKeyRotation,
    // Space events
    SpaceUpdated,
    ChannelCreated,
    ChannelUpdated,
    ChannelDeleted,
    RoleCreated,
    RoleUpdated,
    RoleDeleted,
    MemberJoined,
    MemberLeft,
    MemberBanned,
    MemberUnbanned,
    MemberUpdated,
    // Device/key events
    DeviceAdded,
    DeviceRemoved,
    KeyBundlePublished,
    // System
    Heartbeat,
    Reconnect,
}
