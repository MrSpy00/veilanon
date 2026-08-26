//! Offline message queue
#![allow(dead_code)] // Scaffold module — wired by future UI-facing commands
//! Messages that couldn't be sent due to network unavailability
//! are queued here and retried with exponential backoff.

use rusqlite::params;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::error::{VeilError, VeilResult};
use crate::db::Database;
use crate::models::message::{Message, MessageStatus, MessageType, QueuedMessage};

const MAX_RETRIES: u8 = 10;

impl Database {
    pub fn enqueue_message(&self, msg: &QueuedMessage) -> VeilResult<()> {
        let attachments_json = serde_json::to_string(&msg.message.attachments)
            .map_err(|_| VeilError::SerializationError)?;

        self.execute(
            r#"INSERT INTO offline_queue
               (id, channel_id, ciphertext, iv, crypto_meta, message_type, reply_to_id,
                attachments, disappears_at, retry_count, queued_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)"#,
            params![
                msg.id.to_string(),
                msg.message.channel_id.to_string(),
                msg.message.ciphertext,
                msg.message.iv,
                msg.message.crypto_meta,
                format!("{:?}", msg.message.message_type).to_lowercase(),
                msg.message.reply_to_id.map(|id| id.to_string()),
                attachments_json,
                msg.message.disappears_at.map(|dt| dt.timestamp()),
                msg.retry_count as i32,
                msg.queued_at.timestamp(),
            ],
        )?;
        Ok(())
    }

    pub fn get_pending_queue(&self, limit: u32) -> VeilResult<Vec<QueuedMessage>> {
        let limit = limit.min(100) as i64;
        let now = Utc::now().timestamp();
        self.query_map(
            r#"SELECT id, channel_id, ciphertext, iv, crypto_meta, message_type,
                      reply_to_id, attachments, disappears_at, retry_count,
                      queued_at, next_retry_at, schema_version
               FROM offline_queue
               WHERE next_retry_at IS NULL OR next_retry_at <= ?1
               ORDER BY queued_at ASC
               LIMIT ?2"#,
            params![now, limit],
            row_to_queued,
        )
    }

    pub fn dequeue_message(&self, id: &Uuid) -> VeilResult<()> {
        self.execute(
            "DELETE FROM offline_queue WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }

    pub fn increment_retry_count(&self, id: &Uuid) -> VeilResult<()> {
        let now = Utc::now().timestamp();
        // Exponential backoff: 2^retry_count seconds, max 1 hour
        self.execute(
            r#"UPDATE offline_queue
               SET retry_count = retry_count + 1,
                   next_retry_at = ?1 + MIN(3600, (1 << retry_count))
               WHERE id = ?2"#,
            params![now, id.to_string()],
        )?;
        Ok(())
    }

    pub fn purge_failed_queue(&self) -> VeilResult<u64> {
        let count = self.execute(
            "DELETE FROM offline_queue WHERE retry_count >= ?1",
            params![MAX_RETRIES as i32],
        )?;
        Ok(count as u64)
    }
}

// ── Row mapping ──────────────────────────────────────────────────────────────

fn parse_uuid(s: &str) -> Uuid {
    Uuid::parse_str(s).unwrap_or_else(|_| Uuid::nil())
}

fn ts_to_dt(ts: Option<i64>) -> Option<DateTime<Utc>> {
    ts.and_then(|t| DateTime::from_timestamp(t, 0))
}

fn parse_message_type(s: &str) -> MessageType {
    serde_json::from_str::<MessageType>(&format!("\"{s}\"")).unwrap_or(MessageType::Text)
}

fn row_to_queued(row: &rusqlite::Row<'_>) -> rusqlite::Result<QueuedMessage> {
    let id_str: String = row.get(0)?;
    let channel_str: String = row.get(1)?;
    let ciphertext: String = row.get(2)?;
    let iv: String = row.get(3)?;
    let crypto_meta: Option<String> = row.get(4)?;
    let message_type: String = row.get(5)?;
    let reply_to_id: Option<String> = row.get(6)?;
    let attachments_json: String = row.get(7)?;
    let disappears_at: Option<i64> = row.get(8)?;
    let retry_count: i32 = row.get(9)?;
    let queued_at: i64 = row.get(10)?;
    let next_retry_at: Option<i64> = row.get(11)?;
    let schema_version: i32 = row.get(12)?;

    let id = parse_uuid(&id_str);
    let channel_id = parse_uuid(&channel_str);
    let queued_dt = ts_to_dt(Some(queued_at)).unwrap_or_else(Utc::now);

    let message = Message {
        id,
        channel_id,
        sender_id: Uuid::nil(),
        sender_device_id: Uuid::nil(),
        content: None,
        ciphertext,
        iv,
        crypto_meta,
        message_type: parse_message_type(&message_type),
        status: MessageStatus::Queued,
        reply_to_id: reply_to_id.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
        pinned: false,
        reactions: Vec::new(),
        attachments: serde_json::from_str(&attachments_json).unwrap_or_default(),
        edited_at: None,
        created_at: queued_dt,
        deleted_at: None,
        disappears_at: ts_to_dt(disappears_at),
        schema_version: schema_version.max(0) as u8,
    };

    Ok(QueuedMessage {
        id,
        message,
        retry_count: retry_count.max(0) as u8,
        queued_at: queued_dt,
        next_retry_at: ts_to_dt(next_retry_at),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use rusqlite::{params, Connection};
    use uuid::Uuid;
    use chrono::Utc;
    use crate::db::Database;
    use crate::models::message::{Message, MessageStatus, MessageType};

    fn in_memory_db() -> Database {
        let db = Database { conn: Mutex::new(Connection::open_in_memory().unwrap()) };
        db.run_migrations().unwrap();
        db
    }

    fn test_queued(id: Uuid, channel_id: Uuid) -> QueuedMessage {
        let now = Utc::now();
        QueuedMessage {
            id,
            message: Message {
                id,
                channel_id,
                sender_id: Uuid::nil(),
                sender_device_id: Uuid::nil(),
                content: None,
                ciphertext: "ciphertext-abc".to_string(),
                iv: "iv-def".to_string(),
                crypto_meta: None,
                message_type: MessageType::Text,
                status: MessageStatus::Queued,
                reply_to_id: None,
                pinned: false,
                reactions: Vec::new(),
                attachments: Vec::new(),
                edited_at: None,
                created_at: now,
                deleted_at: None,
                disappears_at: None,
                schema_version: 1,
            },
            retry_count: 0,
            queued_at: now,
            next_retry_at: None,
        }
    }

    #[test]
    fn enqueue_then_pending_returns_row() {
        let db = in_memory_db();
        let id = Uuid::new_v4();
        let channel = Uuid::new_v4();
        db.enqueue_message(&test_queued(id, channel)).unwrap();

        let pending = db.get_pending_queue(10).unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, id);
        assert_eq!(pending[0].message.channel_id, channel);
        assert_eq!(pending[0].message.ciphertext, "ciphertext-abc");
        assert_eq!(pending[0].message.status, MessageStatus::Queued);
    }

    #[test]
    fn future_next_retry_is_not_returned() {
        let db = in_memory_db();
        let id = Uuid::new_v4();
        let channel = Uuid::new_v4();
        db.enqueue_message(&test_queued(id, channel)).unwrap();

        let future = Utc::now().timestamp() + 300;
        db.execute(
            "UPDATE offline_queue SET next_retry_at = ?1 WHERE id = ?2",
            params![future, id.to_string()],
        ).unwrap();

        assert!(db.get_pending_queue(10).unwrap().is_empty());
    }

    #[test]
    fn increment_retry_bumps_count_and_sets_next_retry() {
        let db = in_memory_db();
        let id = Uuid::new_v4();
        let channel = Uuid::new_v4();
        db.enqueue_message(&test_queued(id, channel)).unwrap();

        db.increment_retry_count(&id).unwrap();

        let (retry_count, next_retry_at): (i32, Option<i64>) = db.query_row(
            "SELECT retry_count, next_retry_at FROM offline_queue WHERE id = ?1",
            params![id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();

        assert_eq!(retry_count, 1);
        assert!(next_retry_at.is_some());
    }

    #[test]
    fn purge_removes_exhausted_messages() {
        let db = in_memory_db();
        let id = Uuid::new_v4();
        let channel = Uuid::new_v4();
        db.enqueue_message(&test_queued(id, channel)).unwrap();

        db.execute(
            "UPDATE offline_queue SET retry_count = ?1 WHERE id = ?2",
            params![MAX_RETRIES as i32, id.to_string()],
        ).unwrap();

        let purged = db.purge_failed_queue().unwrap();
        assert_eq!(purged, 1);
        assert!(db.get_pending_queue(10).unwrap().is_empty());
    }

    #[test]
    fn dequeue_removes_single_message() {
        let db = in_memory_db();
        let id = Uuid::new_v4();
        let channel = Uuid::new_v4();
        db.enqueue_message(&test_queued(id, channel)).unwrap();

        db.dequeue_message(&id).unwrap();
        assert!(db.get_pending_queue(10).unwrap().is_empty());
    }
}
