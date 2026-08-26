//! Network subsystem
//! 
//! All outbound content is already encrypted before reaching this layer.
//! Network errors are logged without revealing payload content.

pub mod api;
pub mod realtime;
pub mod retry;
pub mod sync;

pub use api::ApiClient;
pub use realtime::RealtimeManager;

use tracing::{debug, info};
use uuid::Uuid;

use crate::db::Database;
use crate::models::message::{MessageStatus, QueuedMessage};

pub struct NetworkManager {
    pub api: ApiClient,
    pub realtime: RealtimeManager,
}

impl NetworkManager {
    pub fn new() -> Self {
        Self {
            api: ApiClient::new(),
            realtime: RealtimeManager::new(),
        }
    }

    #[allow(dead_code)] // surfaced by the connection-status UI in a later iteration
    pub fn is_connected(&self) -> bool {
        self.realtime.is_connected()
    }

    /// Push queued ciphertext messages to the control plane. Called by the
    /// background flush loop; returns how many messages were delivered.
    pub async fn flush_offline_queue(&self, db: &Database, sender_device_id: Uuid) -> usize {
        let pending: Vec<QueuedMessage> = match db.get_pending_queue(25) {
            Ok(p) => p,
            Err(e) => {
                debug!("Queue read failed: {}", e);
                return 0;
            }
        };

        let mut flushed = 0;
        for queued in pending {
            let msg = &queued.message;
            let payload = serde_json::json!({
                "id": msg.id.to_string(),
                "channel_id": msg.channel_id.to_string(),
                "sender_device_id": sender_device_id.to_string(),
                "sender_id": msg.sender_id.to_string(),
                "ciphertext": msg.ciphertext,
                "iv": msg.iv,
                "crypto_meta": msg.crypto_meta,
                "schema_version": 1,
                "client_created_at": msg.created_at.to_rfc3339(),
                "disappears_at": msg.disappears_at.map(|dt| dt.to_rfc3339()),
            });
            // Supabase columns are NOT NULL with defaults; JSON null from
            // Option<T> fields is rejected with 400. Drop nulls so the DB
            // default applies instead. Plain insert (not upsert) — PostgREST
            // on_conflict + RLS rejects rows whose conflict column differs
            // from auth.uid() with 403.
            let payload = match payload {
                serde_json::Value::Object(map) => serde_json::Value::Object(
                    map.into_iter().filter(|(_, v)| !v.is_null()).collect(),
                ),
                other => other,
            };

            if self.api.insert("messages", &payload).await.is_ok() {
                if db.update_message_status(&msg.id, &MessageStatus::Sent).is_ok() {
                    let _ = db.dequeue_message(&queued.id);
                }
                flushed += 1;
            } else {
                let _ = db.increment_retry_count(&queued.id);
            }
        }

        if flushed > 0 {
            info!("Flushed {} queued message(s)", flushed);
        }
        flushed
    }
}
