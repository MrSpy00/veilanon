//! Message models
//! CRITICAL: The `ciphertext` field is the ONLY form in which message content
//! is stored server-side or transmitted. `plaintext_content` is ephemeral,
//! only exists in memory after successful decryption, and MUST NOT be logged.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message as stored in the local encrypted database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub sender_id: Uuid,
    pub sender_device_id: Uuid,
    /// Ephemeral — only present after local decryption, never logged
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// AES-256-GCM ciphertext (base64) — safe to store/transmit
    pub ciphertext: String,
    /// Nonce/IV (base64)
    pub iv: String,
    /// Per-message crypto metadata (JSON Double-Ratchet header for 1:1 DMs).
    /// NULL for deterministic-key messages. Never contains keys or plaintext.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub crypto_meta: Option<String>,
    pub message_type: MessageType,
    pub status: MessageStatus,
    pub reply_to_id: Option<Uuid>,
    pub pinned: bool,
    pub reactions: Vec<Reaction>,
    pub attachments: Vec<AttachmentRef>,
    pub edited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub disappears_at: Option<DateTime<Utc>>,
    pub schema_version: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text,
    File,
    Image,
    Video,
    Audio,
    System,
    Call,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageStatus {
    Sending,
    Sent,
    Delivered,
    Read,
    Failed,
    Queued, // offline queue
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reaction {
    pub emoji: String,
    pub user_ids: Vec<Uuid>,
    pub count: u32,
}

/// Reference to an encrypted file attachment
/// The actual content key is encrypted with the message key
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentRef {
    pub file_id: Uuid,
    pub r2_key: String,
    pub size_bytes: u64,
    /// Encrypted content key (base64) — required to decrypt the file
    pub content_key_ciphertext: String,
    pub mime_type_hint: Option<String>,
    /// Original plaintext filename, preserved end-to-end for download UX
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_attachment() -> AttachmentRef {
        AttachmentRef {
            file_id: Uuid::parse_str("123e4567-e89b-12d3-a456-426614174000").unwrap(),
            r2_key: "files/genel/abc.bin".to_string(),
            size_bytes: 1024,
            content_key_ciphertext: "a2V5Om5vbmNl".to_string(),
            mime_type_hint: Some("image/png".to_string()),
            file_name: None,
        }
    }

    #[test]
    fn round_trip_preserves_file_name_when_present() {
        // Given an attachment carrying its original filename
        let mut att = sample_attachment();
        att.file_name = Some("gorsel.png".to_string());
        // When it is serialized and deserialized back
        let json = serde_json::to_string(&att).unwrap();
        let parsed: AttachmentRef = serde_json::from_str(&json).unwrap();
        // Then the filename survives the round-trip
        assert_eq!(parsed.file_name.as_deref(), Some("gorsel.png"));
    }

    #[test]
    fn legacy_json_without_file_name_deserializes_to_none() {
        // Given legacy JSON produced before the field existed
        let legacy = serde_json::json!({
            "fileId": "123e4567-e89b-12d3-a456-426614174000",
            "r2Key": "files/genel/abc.bin",
            "sizeBytes": 1024,
            "contentKeyCiphertext": "a2V5Om5vbmNl",
            "mimeTypeHint": "image/png"
        });
        // When it is deserialized
        let parsed: AttachmentRef = serde_json::from_value(legacy).unwrap();
        // Then file_name defaults to None (backward compatible)
        assert_eq!(parsed.file_name, None);
    }
}

/// Offline queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)] // offline queue UI lands next
pub struct QueuedMessage {
    pub id: Uuid,
    pub message: Message,
    pub retry_count: u8,
    pub queued_at: DateTime<Utc>,
    pub next_retry_at: Option<DateTime<Utc>>,
}
