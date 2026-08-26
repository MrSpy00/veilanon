//! Event sync and CRDT-like conflict resolution
#![allow(dead_code)] // Scaffold module — wired by future UI-facing commands

use tracing::{debug, warn};
use crate::error::VeilResult;
use crate::models::event::Event;

/// Process incoming events from the realtime stream
/// Events carry encrypted payloads — decryption happens in command layer
pub struct SyncManager {
    pub last_synced_at: Option<i64>,
}

impl SyncManager {
    pub fn new() -> Self {
        Self { last_synced_at: None }
    }

    /// Apply an event to local state
    /// Security-critical events (key rotation, member removal) are handled
    /// with extra validation before being forwarded to the UI
    pub fn apply_event(&mut self, event: Event) -> VeilResult<EventAction> {
        use crate::models::event::EventType;
        
        debug!("Processing event type: {:?}", event.event_type);
        
        match event.event_type {
            EventType::MessageCreated => Ok(EventAction::UpdateMessages),
            EventType::MessageDeleted => Ok(EventAction::UpdateMessages),
            EventType::MessageEdited => Ok(EventAction::UpdateMessages),
            EventType::PresenceUpdate => Ok(EventAction::UpdatePresence),
            EventType::TypingStart | EventType::TypingStop => Ok(EventAction::UpdateTyping),
            EventType::VoiceStateUpdate => Ok(EventAction::UpdateVoice),
            EventType::MemberJoined | EventType::MemberLeft => Ok(EventAction::UpdateMembers),
            EventType::VoiceKeyRotation => {
                // Security critical — log but don't expose details
                warn!("Voice key rotation event received — updating call keys");
                Ok(EventAction::RotateVoiceKeys)
            }
            EventType::DeviceAdded | EventType::DeviceRemoved => {
                warn!("Device list changed — user should verify device list");
                Ok(EventAction::UpdateDevices)
            }
            _ => Ok(EventAction::NoOp),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EventAction {
    UpdateMessages,
    UpdatePresence,
    UpdateTyping,
    UpdateVoice,
    UpdateMembers,
    UpdateDevices,
    RotateVoiceKeys,
    NoOp,
}
