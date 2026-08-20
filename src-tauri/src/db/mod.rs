//! Local encrypted database
//! 
//! Uses bundled SQLite with application-layer AES-256-GCM encryption over
//! sensitive columns (see `db::cipher`). The encryption key is derived from
//! the user's master key stored in the OS keychain.
//! 
//! The connection is guarded by a std Mutex so `Database` is Send + Sync and
//! can be shared across Tauri command handlers. All queries go through the
//! helper methods below — never touch `Connection` directly outside this module.

pub mod channels;
pub mod cipher;
pub mod messages;
pub mod migrations;
pub mod queue;

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use tracing::{error, info};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use crate::error::{VeilError, VeilResult};
use uuid::Uuid;

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open the (plain SQLite) database and apply pragmas.
    /// At-rest protection of sensitive columns is applied at the data layer.
    pub fn open(path: &Path) -> VeilResult<Self> {
        let conn = Connection::open(path)?;

        // Enable WAL mode for better concurrent performance
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "cache_size", -32000i32)?; // 32MB cache

        info!("Database opened at {:?}", path);
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Drop the on-disk SQLite connection and swap in an in-memory placeholder.
    /// Releases the file handle (required on Windows before deleting DB files).
    /// Existing helpers keep working against the empty in-memory DB afterwards.
    pub fn force_close(&mut self) -> VeilResult<()> {
        self.conn = Mutex::new(Connection::open_in_memory()?);
        Ok(())
    }

    /// Legacy hook for SQLCipher-style key setting — kept for API stability.
    /// No-op under plain SQLite; sensitive columns are encrypted per-value.
    #[allow(dead_code)] // SQLCipher legacy hook
    pub fn set_key(&mut self, _key_hex: &str) -> VeilResult<()> {
        Ok(())
    }

    /// Legacy re-key hook — no-op under plain SQLite.
    #[allow(dead_code)] // SQLCipher legacy hook
    pub fn rekey(&mut self, _new_key_hex: &str) -> VeilResult<()> {
        Ok(())
    }

    /// Run all pending migrations
    pub fn run_migrations(&self) -> VeilResult<()> {
        let mut conn = self.lock()?;
        migrations::run(&mut conn)
    }

    /// Locked access to the underlying connection for advanced callers.
    /// Prefer the helpers (`execute`, `query_row`, ...) when possible.
    #[allow(dead_code)] // advanced callers
    pub fn conn(&self) -> VeilResult<MutexGuard<'_, Connection>> {
        self.lock()
    }

    /// Execute a statement with no return value; returns affected row count
    pub fn execute(&self, sql: &str, sql_params: impl rusqlite::Params) -> VeilResult<usize> {
        let conn = self.lock()?;
        Ok(conn.execute(sql, sql_params)?)
    }

    /// Update the local identity's profile metadata (display name / avatar hash).
    /// No key material is touched — only non-sensitive profile fields.
    pub fn update_local_identity(
        &self,
        id: &Uuid,
        display_name: &str,
        avatar_hash: Option<&str>,
    ) -> VeilResult<()> {
        if display_name.trim().is_empty() {
            return Err(VeilError::InvalidInput("Display name cannot be empty".into()));
        }
        let updated = self.execute(
            "UPDATE local_identity SET display_name = ?1, avatar_hash = ?2 WHERE id = ?3",
            rusqlite::params![display_name, avatar_hash, id.to_string()],
        )?;
        if updated == 0 {
            let _ = self.execute(
                "UPDATE local_identity SET id = ?1, display_name = ?2, avatar_hash = ?3",
                rusqlite::params![id.to_string(), display_name, avatar_hash],
            );
        }
        Ok(())
    }

    /// Update the local identity's username.
    pub fn set_local_identity_username(&self, id: &Uuid, username: &str) -> VeilResult<()> {
        let trimmed = username.trim();
        if trimmed.is_empty() {
            return Err(VeilError::InvalidInput("Username cannot be empty".into()));
        }
        let updated = self.execute(
            "UPDATE local_identity SET username = ?1 WHERE id = ?2",
            rusqlite::params![trimmed, id.to_string()],
        )?;
        if updated == 0 {
            let _ = self.execute(
                "UPDATE local_identity SET id = ?1, username = ?2",
                rusqlite::params![id.to_string(), trimmed],
            );
        }
        Ok(())
    }

    /// Update only the local identity banner (on-device only, never synced).
    pub fn set_local_identity_banner(&self, id: &Uuid, banner_hash: Option<&str>) -> VeilResult<()> {
        let updated = self.execute(
            "UPDATE local_identity SET banner_hash = ?1 WHERE id = ?2",
            rusqlite::params![banner_hash, id.to_string()],
        )?;
        if updated == 0 {
            let _ = self.execute(
                "UPDATE local_identity SET id = ?1, banner_hash = ?2",
                rusqlite::params![id.to_string(), banner_hash],
            );
        }
        Ok(())
    }

    /// Persist the Supabase refresh token so the SAME anonymous control-plane
    /// user is reused across app restarts (no MAU bloat / orphaned rows).
    pub fn save_supabase_refresh_token(&self, id: &Uuid, refresh_token: &str) -> VeilResult<()> {
        let updated = self.execute(
            "UPDATE local_identity SET supabase_refresh_token = ?1 WHERE id = ?2",
            rusqlite::params![refresh_token, id.to_string()],
        )?;
        if updated == 0 {
            let _ = self.execute(
                "UPDATE local_identity SET id = ?1, supabase_refresh_token = ?2",
                rusqlite::params![id.to_string(), refresh_token],
            );
        }
        Ok(())
    }

    /// Read the persisted Supabase refresh token for this identity.
    pub fn supabase_refresh_token(&self, id: &Uuid) -> VeilResult<Option<String>> {
        match self.query_row(
            "SELECT supabase_refresh_token FROM local_identity WHERE id = ?1",
            rusqlite::params![id.to_string()],
            |row| row.get::<_, Option<String>>(0),
        ) {
            Ok(token) => Ok(token),
            // Missing column on pre-0007 databases: treat as no token.
            Err(VeilError::DatabaseError(rusqlite::Error::SqliteFailure(_, _))) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Execute a statement expecting exactly one result row
    pub fn query_row<T, P, F>(&self, sql: &str, sql_params: P, f: F) -> VeilResult<T>
    where
        P: rusqlite::Params,
        F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    {
        let conn = self.lock()?;
        Ok(conn.query_row(sql, sql_params, f)?)
    }

    /// Execute a statement and map every result row
    pub fn query_map<T, P, F>(&self, sql: &str, sql_params: P, mut f: F) -> VeilResult<Vec<T>>
    where
        P: rusqlite::Params,
        F: FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(sql_params, |row| f(row))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Run a closure inside a single transaction (committed on Ok, rolled back on Err)
    #[allow(dead_code)] // multi-statement flows land with sync work
    pub fn transaction<T>(
        &self,
        f: impl FnOnce(&rusqlite::Transaction<'_>) -> VeilResult<T>,
    ) -> VeilResult<T> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        let result = f(&tx)?;
        tx.commit()?;
        Ok(result)
    }

    /// Check database integrity
    #[allow(dead_code)] // diagnostics UI hook
    pub fn integrity_check(&self) -> VeilResult<bool> {
        let result: String = self.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
        Ok(result == "ok")
    }

    /// Get database page count (for size estimation)
    #[allow(dead_code)] // diagnostics UI hook
    pub fn page_count(&self) -> VeilResult<u64> {
        let count: i64 = self.query_row("PRAGMA page_count", [], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Dump every user table as `{ "<table>": [ { "<col>": value, ... }, ... ] }`.
    /// Used by the encrypted data-export archive. Raw ciphertext is exported
    /// as-is (it is already AES-256-GCM at the column layer) — never plaintext.
    pub fn export_all_rows(&self) -> VeilResult<serde_json::Value> {
        let conn = self.lock()?;
        let mut out = serde_json::Map::new();

        for table in EXPORT_TABLES {
            let cols = table_columns(&conn, table)?;
            if cols.is_empty() {
                continue;
            }
            let col_list = cols.iter().map(|c| quote_ident(c)).collect::<Vec<_>>().join(", ");
            let sql = format!("SELECT {col_list} FROM {}", quote_ident(table));

            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map([], |row| {
                let mut obj = serde_json::Map::new();
                for (i, col) in cols.iter().enumerate() {
                    obj.insert(col.clone(), value_ref_to_json(&row.get_ref(i)?)?);
                }
                Ok(serde_json::Value::Object(obj))
            })?;

            let mut arr = Vec::new();
            for row in rows {
                arr.push(row?);
            }
            out.insert((*table).to_string(), serde_json::Value::Array(arr));
        }

        Ok(serde_json::Value::Object(out))
    }

    /// Restore rows from a `{ "<table>": [ {...}, ... ] }` archive object.
    /// Runs inside a single transaction with FK enforcement suspended so tables
    /// can be restored in any order (e.g. self-referential `messages.reply_to_id`).
    /// Unknown tables are ignored; malformed rows are skipped.
    pub fn import_rows(&self, data: &serde_json::Value) -> VeilResult<()> {
        let tables_obj = data
            .as_object()
            .ok_or_else(|| VeilError::InvalidInput("archive tables must be an object".into()))?;

        let mut conn = self.lock()?;
        // Must be toggled outside any transaction.
        conn.execute_batch("PRAGMA foreign_keys = OFF")?;

        let result = (|| -> VeilResult<()> {
            let tx = conn.transaction()?;
            for (table, rows) in tables_obj.iter() {
                if !is_exported_table(table) {
                    continue;
                }
                let Some(rows) = rows.as_array() else { continue; };
                if rows.is_empty() {
                    continue;
                }
                let cols = table_columns(&tx, table)?;
                if cols.is_empty() {
                    continue;
                }
                let col_list = cols.iter().map(|c| quote_ident(c)).collect::<Vec<_>>().join(", ");
                let placeholders = vec!["?"; cols.len()].join(", ");
                let sql = format!(
                    "INSERT OR REPLACE INTO {} ({col_list}) VALUES ({placeholders})",
                    quote_ident(table)
                );
                for row in rows {
                    let Some(obj) = row.as_object() else { continue; };
                    let params: Vec<rusqlite::types::Value> = cols
                        .iter()
                        .map(|c| json_to_sql_value(obj.get(c).unwrap_or(&serde_json::Value::Null)))
                        .collect();
                    tx.execute(&sql, rusqlite::params_from_iter(params.iter()))?;
                }
            }
            tx.commit()?;
            Ok(())
        })();

        // Always re-enable FK enforcement.
        conn.execute_batch("PRAGMA foreign_keys = ON")?;
        result
    }

    fn lock(&self) -> VeilResult<MutexGuard<'_, Connection>> {
        self.conn.lock().map_err(|_| {
            error!("database mutex poisoned");
            VeilError::Internal(anyhow::anyhow!("database lock poisoned"))
        })
    }
}

// ── Data-portability archive helpers ─────────────────────────────────────────

/// User tables exported/imported by the data-portability archive and wiped by
/// `reset_identity`. `_schema_migrations` is intentionally excluded.
const EXPORT_TABLES: &[&str] = &[
    "local_identity",
    "user_profiles",
    "spaces",
    "channels",
    "roles",
    "invites",
    "space_members",
    "messages",
    "offline_queue",
    "dm_sessions",
    "friends",
    "voice_state_cache",
    "app_settings",
];

/// Every user-data table — used by the identity reset command to wipe the
/// device before onboarding starts fresh.
pub const USER_TABLES: &[&str] = EXPORT_TABLES;

fn is_exported_table(table: &str) -> bool {
    EXPORT_TABLES.contains(&table)
}

/// Quote a SQLite identifier (defense in depth; names come from the schema).
fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

/// Read column names for a table via `PRAGMA table_info`.
fn table_columns(conn: &Connection, table: &str) -> VeilResult<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({})", quote_ident(table)))?;
    let cols = stmt.query_map([], |row| row.get::<_, String>(1))?;
    Ok(cols.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Convert a borrowed SQLite value into a JSON value (BLOBs become base64).
fn value_ref_to_json(v: &rusqlite::types::ValueRef<'_>) -> rusqlite::Result<serde_json::Value> {
    use rusqlite::types::ValueRef;
    Ok(match v {
        ValueRef::Null => serde_json::Value::Null,
        ValueRef::Integer(i) => serde_json::Value::from(*i),
        ValueRef::Real(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        ValueRef::Text(s) => serde_json::Value::String(String::from_utf8_lossy(s).into_owned()),
        ValueRef::Blob(b) => serde_json::Value::String(B64.encode(b)),
    })
}

/// Convert a JSON value into an owned SQLite value.
fn json_to_sql_value(v: &serde_json::Value) -> rusqlite::types::Value {
    match v {
        serde_json::Value::Null => rusqlite::types::Value::Null,
        serde_json::Value::Bool(b) => rusqlite::types::Value::Integer(*b as i64),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                rusqlite::types::Value::Real(f)
            } else {
                rusqlite::types::Value::Text(n.to_string())
            }
        }
        serde_json::Value::String(s) => rusqlite::types::Value::Text(s.clone()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            rusqlite::types::Value::Text(serde_json::to_string(v).unwrap_or_default())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_db() -> Database {
        let conn = Connection::open_in_memory().unwrap();
        let db = Database {
            conn: Mutex::new(conn),
        };
        db.run_migrations().unwrap();
        db
    }

    fn insert_identity(db: &Database, id: &Uuid) {
        db.execute(
            r#"INSERT INTO local_identity
               (id, username, display_name, dh_public_key, signing_public_key, device_id)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
            rusqlite::params![
                id.to_string(),
                "testuser",
                "Old Name",
                "dh_pub",
                "sign_pub",
                Uuid::new_v4().to_string(),
            ],
        )
        .unwrap();
    }

    #[test]
    fn update_local_identity_updates_display_name_and_avatar() {
        let db = test_db();
        let id = Uuid::new_v4();
        insert_identity(&db, &id);

        db.update_local_identity(&id, "New Name", Some("hash123")).unwrap();

        let (display_name, avatar_hash): (String, Option<String>) = db
            .query_row(
                "SELECT display_name, avatar_hash FROM local_identity WHERE id = ?1",
                rusqlite::params![id.to_string()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(display_name, "New Name");
        assert_eq!(avatar_hash.as_deref(), Some("hash123"));
    }

    #[test]
    fn update_local_identity_writes_null_avatar_hash() {
        let db = test_db();
        let id = Uuid::new_v4();
        insert_identity(&db, &id);

        db.update_local_identity(&id, "New Name", None).unwrap();

        let avatar_hash: Option<String> = db
            .query_row(
                "SELECT avatar_hash FROM local_identity WHERE id = ?1",
                rusqlite::params![id.to_string()],
                |row| row.get(0),
            )
            .unwrap();
        assert!(avatar_hash.is_none());
    }

    #[test]
    fn update_local_identity_rejects_empty_display_name() {
        let db = test_db();
        let id = Uuid::new_v4();
        insert_identity(&db, &id);

        assert!(db.update_local_identity(&id, "   ", None).is_err());
    }

    fn seed_archive_rows(db: &Database) {
        db.execute(
            r#"INSERT INTO channels (id, name) VALUES ('ch1','general')"#,
            [],
        )
        .unwrap();
        db.execute(
            r#"INSERT INTO messages (id, channel_id, sender_id, sender_device_id, ciphertext, iv, created_at)
               VALUES ('m1','ch1','alice','dev1','Y3Q=','aXY=', 123)"#,
            [],
        )
        .unwrap();
    }

    #[test]
    fn export_import_round_trip() {
        let src = test_db();
        let id = Uuid::new_v4();
        insert_identity(&src, &id);
        seed_archive_rows(&src);

        let dump = src.export_all_rows().unwrap();
        assert_eq!(dump["local_identity"].as_array().unwrap().len(), 1);
        assert_eq!(dump["messages"].as_array().unwrap().len(), 1);

        let dst = test_db();
        dst.import_rows(&dump).unwrap();

        let count: i64 = dst
            .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let name: String = dst
            .query_row("SELECT username FROM local_identity LIMIT 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "testuser");
    }

    #[test]
    fn import_rejects_non_object() {
        let db = test_db();
        let bad = serde_json::json!(42);
        assert!(db.import_rows(&bad).is_err());
    }

    #[test]
    fn import_ignores_unknown_tables() {
        let db = test_db();
        let dump = serde_json::json!({
            "messages": [
                {"id":"m9","channel_id":"ch1","sender_id":"s","sender_device_id":"sd",
                 "ciphertext":"c","iv":"i","created_at":1}
            ],
            "evil_table": [ {"id": 1} ]
        });
        // FK is suspended during import, so a dangling channel_id is allowed;
        // unknown tables are skipped silently.
        db.import_rows(&dump).unwrap();
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
