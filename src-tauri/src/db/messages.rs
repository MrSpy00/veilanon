//! Message database operations
//! All queries return encrypted data — decryption happens in the command layer.

use rusqlite::params;
use uuid::Uuid;
use crate::error::{VeilError, VeilResult};
use crate::db::{cipher, Database};
use crate::models::message::{Message, MessageStatus, MessageType, Reaction, AttachmentRef};

impl Database {
    /// Insert a message (ciphertext only — never plaintext)
    pub fn insert_message(&self, msg: &Message) -> VeilResult<()> {
        let reactions_json = serde_json::to_string(&msg.reactions)
            .map_err(|_| VeilError::SerializationError)?;
        let attachments_json = serde_json::to_string(&msg.attachments)
            .map_err(|_| VeilError::SerializationError)?;

        self.execute(
            r#"INSERT OR REPLACE INTO messages
               (id, channel_id, sender_id, sender_device_id, ciphertext, iv,
                message_type, status, reply_to_id, pinned, reactions, attachments,
                edited_at, created_at, deleted_at, disappears_at, schema_version, crypto_meta)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)"#,
            params![
                msg.id.to_string(),
                msg.channel_id.to_string(),
                msg.sender_id.to_string(),
                msg.sender_device_id.to_string(),
                msg.ciphertext,
                msg.iv,
                format!("{:?}", msg.message_type).to_lowercase(),
                format!("{:?}", msg.status).to_lowercase(),
                msg.reply_to_id.map(|id| id.to_string()),
                msg.pinned as i32,
                reactions_json,
                attachments_json,
                msg.edited_at.map(|dt| dt.timestamp()),
                msg.created_at.timestamp(),
                msg.deleted_at.map(|dt| dt.timestamp()),
                msg.disappears_at.map(|dt| dt.timestamp()),
                msg.schema_version as i32,
                msg.crypto_meta,
            ],
        )?;
        Ok(())
    }

    /// Fetch messages for a channel (encrypted — caller decrypts)
    pub fn get_messages(
        &self,
        channel_id: &Uuid,
        before_id: Option<&Uuid>,
        limit: u32,
    ) -> VeilResult<Vec<Message>> {
        let limit = limit.min(100) as i64;
        let channel_str = channel_id.to_string();

        if let Some(before) = before_id {
            let cursor_created: Option<i64> = self.query_row(
                "SELECT created_at FROM messages WHERE id = ?1 AND channel_id = ?2",
                params![before.to_string(), channel_str],
                |r| r.get(0),
            ).ok();
            if let Some(ts) = cursor_created {
                return self.query_map(
                    r#"SELECT id, channel_id, sender_id, sender_device_id, ciphertext, iv,
                                  message_type, status, reply_to_id, pinned, reactions, attachments,
                                  edited_at, created_at, deleted_at, disappears_at, schema_version, crypto_meta
                           FROM messages
                           WHERE channel_id = ?1 AND deleted_at IS NULL AND (created_at < ?2 OR (created_at = ?2 AND id < ?3))
                           ORDER BY created_at DESC, id DESC LIMIT ?4"#,
                    params![channel_str, ts, before.to_string(), limit],
                    row_to_message,
                );
            }
            return self.query_map(
                r#"SELECT id, channel_id, sender_id, sender_device_id, ciphertext, iv,
                                 message_type, status, reply_to_id, pinned, reactions, attachments,
                                 edited_at, created_at, deleted_at, disappears_at, schema_version, crypto_meta
                          FROM messages
                          WHERE channel_id = ?1 AND deleted_at IS NULL AND id < ?2
                          ORDER BY created_at DESC LIMIT ?3"#,
                params![channel_str, before.to_string(), limit],
                row_to_message,
            );
        }

        self.query_map(
            r#"SELECT id, channel_id, sender_id, sender_device_id, ciphertext, iv,
                              message_type, status, reply_to_id, pinned, reactions, attachments,
                              edited_at, created_at, deleted_at, disappears_at, schema_version, crypto_meta
                       FROM messages
                       WHERE channel_id = ?1 AND deleted_at IS NULL
                       ORDER BY created_at DESC LIMIT ?2"#,
            params![channel_str, limit],
            row_to_message,
        )
    }

    /// Mark message as deleted (soft delete — cryptographic delete on server)
    pub fn soft_delete_message(&self, message_id: &Uuid) -> VeilResult<()> {
        use chrono::Utc;
        self.execute(
            "UPDATE messages SET deleted_at = ?1, ciphertext = '', iv = '' WHERE id = ?2",
            params![Utc::now().timestamp(), message_id.to_string()],
        )?;
        Ok(())
    }

    /// Mark all messages in a channel as deleted
    pub fn clear_channel_messages(&self, channel_id: &Uuid) -> VeilResult<()> {
        use chrono::Utc;
        self.execute(
            "UPDATE messages SET deleted_at = ?1, ciphertext = '', iv = '' WHERE channel_id = ?2",
            params![Utc::now().timestamp(), channel_id.to_string()],
        )?;
        Ok(())
    }

    /// Get pinned messages for a channel
    pub fn get_pinned_messages(&self, channel_id: &Uuid) -> VeilResult<Vec<Message>> {
        self.query_map(
            r#"SELECT id, channel_id, sender_id, sender_device_id, ciphertext, iv,
                      message_type, status, reply_to_id, pinned, reactions, attachments,
                      edited_at, created_at, deleted_at, disappears_at, schema_version, crypto_meta
               FROM messages
               WHERE channel_id = ?1 AND pinned = 1 AND deleted_at IS NULL
               ORDER BY created_at DESC"#,
            params![channel_id.to_string()],
            row_to_message,
        )
    }

    /// Fetch a single message by ID (encrypted — caller decrypts)
    pub fn get_message(&self, id: &Uuid) -> VeilResult<Option<Message>> {
        let rows = self.query_map(
            r#"SELECT id, channel_id, sender_id, sender_device_id, ciphertext, iv,
                      message_type, status, reply_to_id, pinned, reactions, attachments,
                      edited_at, created_at, deleted_at, disappears_at, schema_version, crypto_meta
               FROM messages WHERE id = ?1"#,
            params![id.to_string()],
            row_to_message,
        )?;
        Ok(rows.into_iter().next())
    }

    /// Re-store ciphertext + iv after an edit (re-encrypted with the same message key)
    pub fn update_message_ciphertext(
        &self,
        id: &Uuid,
        ciphertext: &str,
        iv: &str,
        edited_at: i64,
    ) -> VeilResult<()> {
        self.execute(
            "UPDATE messages SET ciphertext = ?1, iv = ?2, edited_at = ?3 WHERE id = ?4",
            params![ciphertext, iv, edited_at, id.to_string()],
        )?;
        Ok(())
    }

    /// Persist the full reactions list for a message
    pub fn update_message_reactions(&self, id: &Uuid, reactions: &[Reaction]) -> VeilResult<()> {
        let reactions_json = serde_json::to_string(reactions)
            .map_err(|_| VeilError::SerializationError)?;
        self.execute(
            "UPDATE messages SET reactions = ?1 WHERE id = ?2",
            params![reactions_json, id.to_string()],
        )?;
        Ok(())
    }

    /// Fetch all non-deleted messages (newest first) for local search.
    /// Caller decrypts and filters in memory — no plaintext search index.
    pub fn get_all_messages(&self, limit: u32) -> VeilResult<Vec<Message>> {
        let limit = limit.min(1000) as i64;
        self.query_map(
            r#"SELECT id, channel_id, sender_id, sender_device_id, ciphertext, iv,
                      message_type, status, reply_to_id, pinned, reactions, attachments,
                      edited_at, created_at, deleted_at, disappears_at, schema_version, crypto_meta
               FROM messages
               WHERE deleted_at IS NULL
               ORDER BY created_at DESC LIMIT ?1"#,
            params![limit],
            row_to_message,
        )
    }

    /// Update message status
    #[allow(dead_code)] // delivery receipts land next
    pub fn update_message_status(&self, message_id: &Uuid, status: &MessageStatus) -> VeilResult<()> {
        self.execute(
            "UPDATE messages SET status = ?1 WHERE id = ?2",
            params![
                format!("{:?}", status).to_lowercase(),
                message_id.to_string()
            ],
        )?;
        Ok(())
    }

    /// Hard-delete locally expired disappearing messages. Returns how many
    /// rows were removed. Expiry is client-side by design: the server only
    /// ever saw the ciphertext hint.
    pub fn purge_expired_messages(&self) -> VeilResult<usize> {
        let deleted = self.execute(
            "DELETE FROM messages WHERE disappears_at IS NOT NULL AND disappears_at <= unixepoch()",
            [],
        )?;
        Ok(deleted)
    }

    // ── DM message-key cache (ratchet re-read support) ──────────────────────

    /// Store a decrypted ratchet message key, wrapped with the DB key.
    pub fn save_message_key(
        &self,
        message_id: &Uuid,
        key: &[u8; 32],
        db_key: Option<&[u8; 32]>,
    ) -> VeilResult<()> {
        let wrap_key = db_key.ok_or(VeilError::Unauthenticated)?;
        let (ct, nonce) = cipher::encrypt(wrap_key, key)?;
        self.execute(
            "INSERT OR REPLACE INTO message_keys (message_id, key_cipher, key_iv) VALUES (?1, ?2, ?3)",
            params![message_id.to_string(), ct, nonce],
        )?;
        Ok(())
    }

    /// Fetch a cached ratchet message key (None when not yet decrypted).
    pub fn get_message_key(
        &self,
        message_id: &Uuid,
        db_key: Option<&[u8; 32]>,
    ) -> VeilResult<Option<[u8; 32]>> {
        let wrap_key = db_key.ok_or(VeilError::Unauthenticated)?;
        let rows = self.query_map(
            "SELECT key_cipher, key_iv FROM message_keys WHERE message_id = ?1",
            params![message_id.to_string()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?;
        let Some((ct, nonce)) = rows.into_iter().next() else {
            return Ok(None);
        };
        let plaintext = cipher::decrypt(wrap_key, &ct, &nonce)?;
        let key: [u8; 32] = plaintext.try_into().map_err(|_| VeilError::DecryptionError)?;
        Ok(Some(key))
    }

    /// Drop a cached key (message edited → old ciphertext replaced).
    pub fn delete_message_key(&self, message_id: &Uuid) -> VeilResult<()> {
        self.execute(
            "DELETE FROM message_keys WHERE message_id = ?1",
            params![message_id.to_string()],
        )?;
        Ok(())
    }

    // ── DM sessions (ratchet state — encrypted at rest) ──────────────────────

    /// Persist a DM ratchet state, encrypted with the DB key.
    /// Requires an authenticated session (db_key present).
    #[allow(dead_code)] // DM ratchet persistence wired by session commands
    pub fn save_dm_session(
        &self,
        channel_id: &Uuid,
        peer_id: &Uuid,
        ratchet_json: &str,
        db_key: Option<&[u8; 32]>,
    ) -> VeilResult<()> {
        let key = db_key.ok_or(VeilError::Unauthenticated)?;
        let stored = seal_ratchet(key, ratchet_json)?;
        self.execute(
            r#"INSERT OR REPLACE INTO dm_sessions (id, peer_id, ratchet_state, updated_at)
               VALUES (?1, ?2, ?3, unixepoch())"#,
            params![channel_id.to_string(), peer_id.to_string(), stored],
        )?;
        Ok(())
    }

    /// Load and decrypt a DM ratchet state.
    /// Returns `(peer_id, ratchet_json)`.
    #[allow(dead_code)] // DM ratchet persistence wired by session commands
    pub fn load_dm_session(
        &self,
        channel_id: &Uuid,
        db_key: Option<&[u8; 32]>,
    ) -> VeilResult<Option<(Uuid, String)>> {
        let key = db_key.ok_or(VeilError::Unauthenticated)?;
        let row = self.query_map(
            "SELECT peer_id, ratchet_state FROM dm_sessions WHERE id = ?1",
            params![channel_id.to_string()],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?;

        match row.into_iter().next() {
            Some((peer_id, stored)) => {
                let peer = Uuid::parse_str(&peer_id).map_err(|_| VeilError::SerializationError)?;
                let ratchet_json = open_ratchet(key, &stored)?;
                Ok(Some((peer, ratchet_json)))
            }
            None => Ok(None),
        }
    }

    /// Delete a DM session (e.g. peer identity key changed)
    #[allow(dead_code)] // DM ratchet persistence wired by session commands
    pub fn delete_dm_session(&self, channel_id: &Uuid) -> VeilResult<()> {
        self.execute(
            "DELETE FROM dm_sessions WHERE id = ?1",
            params![channel_id.to_string()],
        )?;
        Ok(())
    }
}

// ── Row mapping ──────────────────────────────────────────────────────────────

fn row_to_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    use chrono::{DateTime, Utc};

    fn ts_to_dt(ts: Option<i64>) -> Option<DateTime<Utc>> {
        ts.and_then(|t| DateTime::from_timestamp(t, 0))
    }

    let parse_uuid = |s: &str| Uuid::parse_str(s);
    let reactions_json: String = row.get(10)?;
    let attachments_json: String = row.get(11)?;

    Ok(Message {
        id: parse_uuid(&row.get::<_, String>(0)?).unwrap_or_else(|_| Uuid::nil()),
        channel_id: parse_uuid(&row.get::<_, String>(1)?).unwrap_or_else(|_| Uuid::nil()),
        sender_id: parse_uuid(&row.get::<_, String>(2)?).unwrap_or_else(|_| Uuid::nil()),
        sender_device_id: parse_uuid(&row.get::<_, String>(3)?).unwrap_or_else(|_| Uuid::nil()),
        content: None,
        ciphertext: row.get(4)?,
        iv: row.get(5)?,
        crypto_meta: row.get(17)?,
        message_type: parse_message_type(&row.get::<_, String>(6)?),
        status: parse_message_status(&row.get::<_, String>(7)?),
        reply_to_id: row.get::<_, Option<String>>(8)?.as_deref().and_then(|s| parse_uuid(s).ok()),
        pinned: row.get::<_, i32>(9)? != 0,
        reactions: serde_json::from_str(&reactions_json).unwrap_or_default(),
        attachments: serde_json::from_str(&attachments_json).unwrap_or_default(),
        edited_at: ts_to_dt(row.get(12)?),
        created_at: ts_to_dt(Some(row.get(13)?)).unwrap_or_else(Utc::now),
        deleted_at: ts_to_dt(row.get(14)?),
        disappears_at: ts_to_dt(row.get(15)?),
        schema_version: row.get::<_, i32>(16)?.max(0) as u8,
    })
}

fn parse_message_type(s: &str) -> MessageType {
    serde_json::from_str::<MessageType>(&format!("\"{s}\""))
        .unwrap_or(MessageType::Text)
}

fn parse_message_status(s: &str) -> MessageStatus {
    serde_json::from_str::<MessageStatus>(&format!("\"{s}\""))
        .unwrap_or(MessageStatus::Sent)
}

// ── Ratchet at-rest envelope ─────────────────────────────────────────────────

#[allow(dead_code)] // ratchet envelope used by session commands
fn seal_ratchet(key: &[u8; 32], ratchet_json: &str) -> VeilResult<String> {
    let (ciphertext, nonce) = cipher::encrypt(key, ratchet_json.as_bytes())?;
    Ok(format!("{nonce}.{ciphertext}"))
}

#[allow(dead_code)] // ratchet envelope used by session commands
fn open_ratchet(key: &[u8; 32], stored: &str) -> VeilResult<String> {
    let (nonce, ciphertext) = stored.split_once('.').ok_or(VeilError::DecryptionError)?;
    let plaintext = cipher::decrypt(key, ciphertext, nonce)?;
    String::from_utf8(plaintext).map_err(|_| VeilError::DecryptionError)
}

// ── Pending DM messages (peer key missing) ───────────────────────────────

impl Database {
    #[allow(dead_code)]
    pub fn insert_pending_dm(
        &self,
        id: &Uuid,
        channel_id: &Uuid,
        peer_id: &Uuid,
        content: &str,
        message_type: &str,
        reply_to_id: Option<&Uuid>,
        attachments: &[AttachmentRef],
        disappears_at: Option<i64>,
    ) -> VeilResult<()> {
        self.insert_pending_dm_encrypted(id, channel_id, peer_id, content, message_type, reply_to_id, attachments, disappears_at, None)
    }

    pub fn insert_pending_dm_encrypted(
        &self,
        id: &Uuid,
        channel_id: &Uuid,
        peer_id: &Uuid,
        content: &str,
        message_type: &str,
        reply_to_id: Option<&Uuid>,
        attachments: &[AttachmentRef],
        disappears_at: Option<i64>,
        db_key: Option<&[u8; 32]>,
    ) -> VeilResult<()> {
        let attachments_json = serde_json::to_string(attachments)
            .map_err(|_| VeilError::SerializationError)?;
        let (store_content, cipher, nonce) = if let Some(k) = db_key {
            let (ct, n) = cipher::encrypt(k, content.as_bytes())?;
            (String::new(), Some(ct), Some(n))
        } else {
            (content.to_string(), None, None)
        };
        let c_cipher: Option<String> = cipher;
        let c_nonce: Option<String> = nonce;
        self.execute(
            r#"INSERT INTO pending_dm_messages
               (id, channel_id, peer_id, content, content_cipher, content_nonce, message_type, reply_to_id,
                attachments, disappears_at, created_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)"#,
            params![
                id.to_string(),
                channel_id.to_string(),
                peer_id.to_string(),
                store_content,
                c_cipher,
                c_nonce,
                message_type,
                reply_to_id.map(|id| id.to_string()),
                attachments_json,
                disappears_at,
                chrono::Utc::now().timestamp(),
            ],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_pending_dms_by_peer(&self, peer_id: &Uuid) -> VeilResult<Vec<(Uuid, Uuid, String, String, Option<String>, Vec<AttachmentRef>, Option<i64>)>> {
        self.get_pending_dms_by_peer_decrypted(peer_id, None)
    }

    pub fn get_pending_dms_by_peer_decrypted(
        &self,
        peer_id: &Uuid,
        db_key: Option<&[u8; 32]>,
    ) -> VeilResult<Vec<(Uuid, Uuid, String, String, Option<String>, Vec<AttachmentRef>, Option<i64>)>> {
        let rows: Vec<(String, String, String, Option<String>, Option<String>, String, Option<String>, String, Option<i64>)> = self.query_map(
            "SELECT id, channel_id, content, content_cipher, content_nonce, message_type, reply_to_id, attachments, disappears_at FROM pending_dm_messages WHERE peer_id = ?1 ORDER BY created_at ASC",
            params![peer_id.to_string()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?)),
        )?;
        let mut out = Vec::with_capacity(rows.len());
        for (id_s, ch_s, content_plain, c_cipher, c_nonce, mtype, reply, att_json, dis) in rows {
            let is_encrypted = c_cipher.is_some();
            let content = if let (Some(ct), Some(nc)) = (c_cipher, c_nonce) {
                if let Some(k) = db_key {
                    cipher::decrypt(k, &ct, &nc).ok().and_then(|b| String::from_utf8(b).ok()).unwrap_or_default()
                } else {
                    String::new()
                }
            } else {
                content_plain
            };
            if content.is_empty() && is_encrypted { continue; }
            let attachments: Vec<AttachmentRef> = serde_json::from_str(&att_json).unwrap_or_default();
            out.push((
                Uuid::parse_str(&id_s).unwrap_or_default(),
                Uuid::parse_str(&ch_s).unwrap_or_default(),
                content,
                mtype,
                reply,
                attachments,
                dis,
            ));
        }
        Ok(out)
    }

    pub fn delete_pending_dm(&self, id: &Uuid) -> VeilResult<()> {
        self.execute(
            "DELETE FROM pending_dm_messages WHERE id = ?1",
            params![id.to_string()],
        )?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete_all_pending_dms_for_peer(&self, peer_id: &Uuid) -> VeilResult<()> {
        self.execute(
            "DELETE FROM pending_dm_messages WHERE peer_id = ?1",
            params![peer_id.to_string()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use rusqlite::Connection;
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    use crate::crypto::{decrypt_aes_gcm, derive_message_key, encrypt_aes_gcm};

    const DB_KEY: [u8; 32] = [0x42; 32];

    fn test_db() -> Database {
        let db = Database {
            conn: Mutex::new(Connection::open_in_memory().unwrap()),
        };
        db.run_migrations().unwrap();
        db
    }

    fn seed_channel(db: &Database, channel_id: &Uuid) {
        db.execute(
            "INSERT INTO channels (id, name) VALUES (?1, 'test')",
            rusqlite::params![channel_id.to_string()],
        )
        .unwrap();
    }

    fn encrypted_message(db_key: &[u8; 32], id: Uuid, channel_id: Uuid, content: &str) -> Message {
        let msg_key = derive_message_key(db_key, &id).unwrap();
        let (ct, nonce) = encrypt_aes_gcm(&msg_key, content.as_bytes()).unwrap();
        Message {
            id,
            channel_id,
            sender_id: Uuid::new_v4(),
            sender_device_id: Uuid::new_v4(),
            content: None,
            ciphertext: B64.encode(&ct),
            iv: B64.encode(&nonce),
            crypto_meta: None,
            message_type: MessageType::Text,
            status: MessageStatus::Sent,
            reply_to_id: None,
            pinned: false,
            reactions: Vec::new(),
            attachments: Vec::new(),
            edited_at: None,
            created_at: chrono::Utc::now(),
            deleted_at: None,
            disappears_at: None,
            schema_version: 1,
        }
    }

    fn decrypt(db_key: &[u8; 32], msg: &Message) -> Option<String> {
        let key = derive_message_key(db_key, &msg.id).ok()?;
        let ct = B64.decode(&msg.ciphertext).ok()?;
        let nonce = B64.decode(&msg.iv).ok()?;
        let plain = decrypt_aes_gcm(&key, &ct, &nonce).ok()?;
        String::from_utf8(plain).ok()
    }

    #[test]
    fn edit_message_reencrypts_content() {
        let db = test_db();
        let channel = Uuid::new_v4();
        seed_channel(&db, &channel);

        let msg_id = Uuid::new_v4();
        let msg = encrypted_message(&DB_KEY, msg_id, channel, "hello");
        db.insert_message(&msg).unwrap();

        let key = derive_message_key(&DB_KEY, &msg_id).unwrap();
        let (ct, nonce) = encrypt_aes_gcm(&key, b"edited content").unwrap();
        let now = chrono::Utc::now().timestamp();
        db.update_message_ciphertext(&msg_id, &B64.encode(&ct), &B64.encode(&nonce), now)
            .unwrap();

        let fetched = db.get_message(&msg_id).unwrap().unwrap();
        assert_eq!(decrypt(&DB_KEY, &fetched).unwrap(), "edited content");
        assert!(fetched.edited_at.is_some());
    }

    #[test]
    fn search_all_messages_is_case_insensitive() {
        let db = test_db();
        let channel = Uuid::new_v4();
        seed_channel(&db, &channel);

        let msg1 = encrypted_message(&DB_KEY, Uuid::new_v4(), channel, "selam");
        let msg2 = encrypted_message(&DB_KEY, Uuid::new_v4(), channel, "merhaba");
        db.insert_message(&msg1).unwrap();
        db.insert_message(&msg2).unwrap();

        let all = db.get_all_messages(1000).unwrap();
        assert_eq!(all.len(), 2);

        let needle = "SELAM".to_lowercase();
        let matches: Vec<String> = all
            .iter()
            .filter_map(|m| decrypt(&DB_KEY, m))
            .filter(|c| c.to_lowercase().contains(&needle))
            .collect();
        assert_eq!(matches, vec!["selam"]);
    }

    #[test]
    fn pinned_messages_are_decryptable() {
        let db = test_db();
        let channel = Uuid::new_v4();
        seed_channel(&db, &channel);

        let msg = encrypted_message(&DB_KEY, Uuid::new_v4(), channel, "pin me");
        db.insert_message(&msg).unwrap();
        db.execute(
            "UPDATE messages SET pinned = 1 WHERE id = ?1",
            rusqlite::params![msg.id.to_string()],
        )
        .unwrap();

        let pinned = db.get_pinned_messages(&channel).unwrap();
        assert_eq!(pinned.len(), 1);
        assert_eq!(decrypt(&DB_KEY, &pinned[0]).unwrap(), "pin me");
    }

    #[test]
    fn reactions_json_round_trips() {
        let db = test_db();
        let channel = Uuid::new_v4();
        seed_channel(&db, &channel);

        let msg = encrypted_message(&DB_KEY, Uuid::new_v4(), channel, "hi");
        db.insert_message(&msg).unwrap();

        let actor = Uuid::new_v4();
        let reactions = vec![Reaction {
            emoji: "👍".to_string(),
            user_ids: vec![actor],
            count: 1,
        }];
        db.update_message_reactions(&msg.id, &reactions).unwrap();

        let fetched = db.get_message(&msg.id).unwrap().unwrap();
        assert_eq!(fetched.reactions.len(), 1);
        assert_eq!(fetched.reactions[0].emoji, "👍");
        assert_eq!(fetched.reactions[0].user_ids, vec![actor]);
        assert_eq!(fetched.reactions[0].count, 1);
    }
}
