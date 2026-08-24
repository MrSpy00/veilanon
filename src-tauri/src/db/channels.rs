//! Channel, space, role, invite, member and friend database operations

use chrono::{DateTime, Utc};
use rusqlite::params;
use uuid::Uuid;

use crate::db::{cipher, Database};
use crate::error::{VeilError, VeilResult};
use crate::models::channel::{Channel, ChannelType};
use crate::models::space::Permissions;

type ProfileRow = (Uuid, String, String, Option<String>);

// ── Lightweight row views (IPC-shaped, built by the command layer) ──────────

pub struct SpaceRow {
    pub id: Uuid,
    pub name: String,
    pub icon_hash: Option<String>,
    pub owner_id: Uuid,
    pub member_count: u32,
    pub is_owner: bool,
    pub my_roles: Vec<Uuid>,
    pub banner_hash: Option<String>,
    pub description: Option<String>,
    pub custom_link: Option<String>,
}

pub struct ChannelRow {
    pub id: Uuid,
    pub space_id: Option<Uuid>,
    pub name: String,
    pub channel_type: String,
    pub position: i32,
    pub is_nsfw: bool,
    pub is_e2ee: bool,
    pub unread_count: u32,
    pub mentioned: bool,
    pub last_message_id: Option<Uuid>,
}

pub struct RoleRow {
    pub id: Uuid,
    pub space_id: Uuid,
    pub name: String,
    pub color: Option<String>,
    pub permissions: Permissions,
    pub position: i32,
    pub is_default: bool,
}

pub struct InviteRow {
    pub id: Uuid,
    pub code: String,
    pub space_id: Uuid,
    pub max_uses: Option<u32>,
    pub used_count: u32,
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct MemberRow {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_hash: Option<String>,
    pub role_ids: Vec<Uuid>,
    pub online_status: String,
}

pub struct BanRow {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub banned_by: Uuid,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub struct FriendRow {
    pub user_id: Uuid,
    pub username: String,
    pub display_name: String,
    pub avatar_hash: Option<String>,
    pub status: String,
    pub online_status: String,
}

fn parse_uuids(json: &str) -> Vec<Uuid> {
    serde_json::from_str::<Vec<String>>(json)
        .unwrap_or_default()
        .iter()
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect()
}

impl Database {
    // ── Channels ────────────────────────────────────────────────────────────

    pub fn upsert_channel(&self, channel: &Channel, db_key: Option<&[u8; 32]>) -> VeilResult<()> {
        let overrides_json = serde_json::to_string(&channel.permission_overrides)
            .map_err(|_| VeilError::SerializationError)?;

        let topic_ciphertext = match (&channel.topic, db_key) {
            (Some(topic), Some(key)) => {
                let (ct, nonce) = cipher::encrypt(key, topic.as_bytes())?;
                Some(format!("{nonce}.{ct}"))
            }
            _ => None,
        };

        self.execute(
            r#"INSERT OR REPLACE INTO channels
               (id, space_id, name, channel_type, position, is_nsfw, is_e2ee,
                slow_mode_seconds, permission_overrides, last_message_id,
                unread_count, mentioned, topic_ciphertext, created_at, updated_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,unixepoch())"#,
            params![
                channel.id.to_string(),
                channel.space_id.map(|id| id.to_string()),
                channel.name,
                channel.channel_type.to_db_string(),
                channel.position,
                channel.is_nsfw as i32,
                channel.is_e2ee as i32,
                channel.slow_mode_seconds as i64,
                overrides_json,
                channel.last_message_id.map(|id| id.to_string()),
                channel.unread_count as i64,
                channel.mentioned as i32,
                topic_ciphertext,
                channel.created_at.timestamp(),
            ],
        )?;
        Ok(())
    }

    pub fn get_channel(&self, id: &Uuid, db_key: Option<&[u8; 32]>) -> VeilResult<Option<Channel>> {
        let rows = self.query_map(
            r#"SELECT id, space_id, name, channel_type, position, topic_ciphertext,
                      is_nsfw, is_e2ee, slow_mode_seconds, permission_overrides,
                      created_at, last_message_id, unread_count, mentioned
               FROM channels WHERE id = ?1"#,
            params![id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i32>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i32>(6)?,
                    row.get::<_, i32>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, Option<String>>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, i32>(13)?,
                ))
            },
        )?;

        let Some(row) = rows.into_iter().next() else {
            return Ok(None);
        };

        let topic = match (&row.5, db_key) {
            (Some(stored), Some(key)) => {
                let (nonce, ct) = stored.split_once('.').ok_or(VeilError::DecryptionError)?;
                let plain = cipher::decrypt(key, ct, nonce)?;
                Some(String::from_utf8(plain).map_err(|_| VeilError::DecryptionError)?)
            }
            _ => None,
        };

        Ok(Some(Channel {
            id: Uuid::parse_str(&row.0).unwrap_or_else(|_| Uuid::nil()),
            space_id: row.1.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
            name: row.2,
            channel_type: ChannelType::from_db_string(&row.3),
            position: row.4,
            topic,
            is_nsfw: row.6 != 0,
            is_e2ee: row.7 != 0,
            slow_mode_seconds: row.8.max(0) as u32,
            permission_overrides: serde_json::from_str(&row.9).unwrap_or_default(),
            created_at: DateTime::from_timestamp(row.10, 0).unwrap_or_else(Utc::now),
            last_message_id: row.11.as_deref().and_then(|s| Uuid::parse_str(s).ok()),
            unread_count: row.12.max(0) as u32,
            mentioned: row.13 != 0,
        }))
    }

    pub fn get_channels_for_space(&self, space_id: &Uuid) -> VeilResult<Vec<ChannelRow>> {
        self.query_map(
            r#"SELECT id, space_id, name, channel_type, position, is_nsfw, is_e2ee,
                      unread_count, mentioned, last_message_id
               FROM channels WHERE space_id = ?1 ORDER BY position ASC"#,
            params![space_id.to_string()],
            row_to_channel_row,
        )
    }

    pub fn list_dm_channels(&self) -> VeilResult<Vec<ChannelRow>> {
        self.query_map(
            r#"SELECT id, space_id, name, channel_type, position, is_nsfw, is_e2ee,
                      unread_count, mentioned, last_message_id
               FROM channels WHERE space_id IS NULL ORDER BY position ASC"#,
            [],
            row_to_channel_row,
        )
    }

    pub fn delete_channel(&self, id: &Uuid) -> VeilResult<()> {
        self.execute("DELETE FROM channels WHERE id = ?1", params![id.to_string()])?;
        Ok(())
    }

    pub fn next_channel_position(&self, space_id: Option<&Uuid>) -> VeilResult<i32> {
        let (sql, param) = match space_id {
            Some(id) => (
                "SELECT COALESCE(MAX(position), -1) + 1 FROM channels WHERE space_id = ?1",
                id.to_string(),
            ),
            None => (
                "SELECT COALESCE(MAX(position), -1) + 1 FROM channels WHERE space_id IS NULL",
                String::new(),
            ),
        };
        let pos: i64 = if space_id.is_some() {
            self.query_row(sql, params![param], |row| row.get(0))?
        } else {
            self.query_row(sql, [], |row| row.get(0))?
        };
        Ok(pos as i32)
    }

    pub fn mark_channel_read(&self, channel_id: &Uuid) -> VeilResult<()> {
        self.execute(
            "UPDATE channels SET unread_count = 0, mentioned = 0 WHERE id = ?1",
            params![channel_id.to_string()],
        )?;
        Ok(())
    }

    #[allow(dead_code)] // realtime ingest hook
    pub fn increment_unread(&self, channel_id: &Uuid, mentioned: bool) -> VeilResult<()> {
        self.execute(
            "UPDATE channels SET unread_count = unread_count + 1, mentioned = MAX(mentioned, ?1) WHERE id = ?2",
            params![mentioned as i32, channel_id.to_string()],
        )?;
        Ok(())
    }

    // ── Spaces ──────────────────────────────────────────────────────────────

    pub fn insert_space(
        &self,
        id: &Uuid,
        name: &str,
        icon_hash: Option<&str>,
        owner_id: &Uuid,
    ) -> VeilResult<()> {
        self.insert_space_full(id, name, icon_hash, owner_id, None, None, None)
    }

    pub fn insert_space_full(
        &self,
        id: &Uuid,
        name: &str,
        icon_hash: Option<&str>,
        owner_id: &Uuid,
        banner_hash: Option<&str>,
        description: Option<&str>,
        custom_link: Option<&str>,
    ) -> VeilResult<()> {
        let icon_val: Option<&str> = icon_hash.filter(|s| !s.is_empty());
        let banner_val: Option<&str> = banner_hash.filter(|s| !s.is_empty());
        let desc_val: Option<&str> = description.filter(|s| !s.is_empty());
        let link_val: Option<&str> = custom_link.filter(|s| !s.is_empty());
        self.execute(
            r#"INSERT INTO spaces (id, name, icon_hash, owner_id, member_count, is_owner, banner_hash, description, custom_link)
               VALUES (?1, ?2, ?3, ?4, 1, 0, ?5, ?6, ?7)
               ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 icon_hash = coalesce(nullif(excluded.icon_hash, ''), spaces.icon_hash),
                 owner_id = excluded.owner_id,
                 banner_hash = coalesce(nullif(excluded.banner_hash, ''), spaces.banner_hash),
                 description = coalesce(nullif(excluded.description, ''), spaces.description),
                 custom_link = coalesce(nullif(excluded.custom_link, ''), spaces.custom_link),
                 updated_at = unixepoch()"#,
            params![
                id.to_string(),
                name,
                icon_val,
                owner_id.to_string(),
                banner_val,
                desc_val,
                link_val,
            ],
        )?;
        self.add_space_member(id, owner_id)?;
        Ok(())
    }

    pub fn set_space_owner(&self, id: &Uuid, user_id: &Uuid, _is_owner: bool) -> VeilResult<()> {
        let space = self.get_space(id)?;
        let actual_owner = space.as_ref().map(|s| s.owner_id == *user_id).unwrap_or(false);
        self.execute(
            "UPDATE spaces SET is_owner = ?1 WHERE id = ?2",
            params![actual_owner as i32, id.to_string()],
        )?;
        Ok(())
    }

    pub fn update_space(
        &self,
        id: &Uuid,
        name: Option<&str>,
        icon_hash: Option<Option<&str>>,
        banner_hash: Option<Option<&str>>,
        description: Option<Option<&str>>,
    ) -> VeilResult<()> {
        if let Some(n) = name {
            self.execute(
                "UPDATE spaces SET name = ?1, updated_at = unixepoch() WHERE id = ?2",
                params![n, id.to_string()],
            )?;
        }
        if let Some(i) = icon_hash {
            self.execute(
                "UPDATE spaces SET icon_hash = ?1, updated_at = unixepoch() WHERE id = ?2",
                params![i, id.to_string()],
            )?;
        }
        if let Some(b) = banner_hash {
            self.execute(
                "UPDATE spaces SET banner_hash = ?1, updated_at = unixepoch() WHERE id = ?2",
                params![b, id.to_string()],
            )?;
        }
        if let Some(d) = description {
            self.execute(
                "UPDATE spaces SET description = ?1, updated_at = unixepoch() WHERE id = ?2",
                params![d, id.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn transfer_space_ownership(&self, space_id: &Uuid, new_owner_id: &Uuid) -> VeilResult<()> {
        self.execute(
            "UPDATE spaces SET owner_id = ?1, is_owner = 0, updated_at = unixepoch() WHERE id = ?2",
            params![new_owner_id.to_string(), space_id.to_string()],
        )?;
        Ok(())
    }

    pub fn get_space(&self, id: &Uuid) -> VeilResult<Option<SpaceRow>> {
        let rows = self.query_map(
            r#"SELECT id, name, icon_hash, owner_id, member_count, is_owner, my_roles, banner_hash, description, custom_link
               FROM spaces WHERE id = ?1"#,
            params![id.to_string()],
            row_to_space_row,
        )?;
        Ok(rows.into_iter().next())
    }

    /// Topluluk kısa bağlantısı (custom_link) ile topluluk bul.
    pub fn get_space_by_custom_link(&self, link: &str) -> VeilResult<Option<SpaceRow>> {
        let rows = self.query_map(
            r#"SELECT id, name, icon_hash, owner_id, member_count, is_owner, my_roles, banner_hash, description, custom_link
               FROM spaces WHERE custom_link = ?1"#,
            params![link],
            row_to_space_row,
        )?;
        Ok(rows.into_iter().next())
    }

    /// Özel bağlantıyı ayarla. Zaten doluysa değiştirilemez (bir kez alınır).
    /// Boş değer bağlantıyı hiç oluşturulmamış gibi bırakır — iptal yoktur.
    pub fn set_custom_link(&self, id: &Uuid, link: &str) -> VeilResult<bool> {
        let current: Option<String> = self
            .query_row(
                "SELECT custom_link FROM spaces WHERE id = ?1",
                params![id.to_string()],
                |row| row.get(0),
            )
            .unwrap_or(None);
        if current.as_deref().map(|c| !c.is_empty()).unwrap_or(false) {
            return Ok(false);
        }
        self.execute(
            "UPDATE spaces SET custom_link = ?1, updated_at = unixepoch() WHERE id = ?2",
            params![link, id.to_string()],
        )?;
        Ok(true)
    }

    pub fn list_spaces(&self) -> VeilResult<Vec<SpaceRow>> {
        self.query_map(
            r#"SELECT id, name, icon_hash, owner_id, member_count, is_owner, my_roles, banner_hash, description, custom_link
               FROM spaces ORDER BY name ASC"#,
            [],
            row_to_space_row,
        )
    }

    pub fn delete_space(&self, id: &Uuid) -> VeilResult<()> {
        let sid = id.to_string();
        let _ = self.execute(
            "DELETE FROM messages WHERE channel_id IN (SELECT id FROM channels WHERE space_id = ?1)",
            params![sid],
        );
        let _ = self.execute(
            "DELETE FROM channel_members WHERE channel_id IN (SELECT id FROM channels WHERE space_id = ?1)",
            params![sid],
        );
        let _ = self.execute("DELETE FROM channels WHERE space_id = ?1", params![sid]);
        let _ = self.execute(
            "DELETE FROM role_members WHERE role_id IN (SELECT id FROM roles WHERE space_id = ?1)",
            params![sid],
        );
        let _ = self.execute("DELETE FROM roles WHERE space_id = ?1", params![sid]);
        let _ = self.execute("DELETE FROM space_members WHERE space_id = ?1", params![sid]);
        let _ = self.execute("DELETE FROM bans WHERE space_id = ?1", params![sid]);
        let _ = self.execute("DELETE FROM invites WHERE space_id = ?1", params![sid]);
        let _ = self.execute("DELETE FROM spaces WHERE id = ?1", params![sid]);
        Ok(())
    }

    // ── Members ─────────────────────────────────────────────────────────────

    pub fn add_space_member(&self, space_id: &Uuid, user_id: &Uuid) -> VeilResult<()> {
        self.execute(
            "INSERT OR IGNORE INTO space_members (space_id, user_id) VALUES (?1, ?2)",
            params![space_id.to_string(), user_id.to_string()],
        )?;
        self.execute(
            r#"UPDATE spaces SET member_count = (
                   SELECT COUNT(*) FROM space_members WHERE space_id = ?1
               ), updated_at = unixepoch() WHERE id = ?1"#,
            params![space_id.to_string()],
        )?;
        Ok(())
    }

    pub fn remove_space_member(&self, space_id: &Uuid, user_id: &Uuid) -> VeilResult<()> {
        self.execute(
            "DELETE FROM space_members WHERE space_id = ?1 AND user_id = ?2",
            params![space_id.to_string(), user_id.to_string()],
        )?;
        self.execute(
            r#"UPDATE spaces SET member_count = (
                   SELECT COUNT(*) FROM space_members WHERE space_id = ?1
               ), updated_at = unixepoch() WHERE id = ?1"#,
            params![space_id.to_string()],
        )?;
        Ok(())
    }

    pub fn list_space_members(&self, space_id: &Uuid) -> VeilResult<Vec<MemberRow>> {
        self.query_map(
            r#"SELECT m.user_id,
                      COALESCE(p.username, i.username, 'kullanici-' || substr(m.user_id, 1, 8)) AS username,
                      COALESCE(p.display_name, i.display_name, p.username, i.username, 'Kullanıcı') AS display_name,
                      COALESCE(p.avatar_hash, i.avatar_hash) AS avatar_hash,
                      m.role_ids,
                      COALESCE(p.online_status, 'offline') AS online_status
               FROM space_members m
               LEFT JOIN user_profiles p ON p.id = m.user_id
               LEFT JOIN local_identity i ON i.id = m.user_id
               WHERE m.space_id = ?1 ORDER BY display_name ASC"#,
            params![space_id.to_string()],
            row_to_member_row,
        )
    }

    pub fn update_space_member_roles(
        &self,
        space_id: &Uuid,
        user_id: &Uuid,
        role_ids: &[Uuid],
    ) -> VeilResult<()> {
        let _ = self.add_space_member(space_id, user_id);
        let ids: Vec<String> = role_ids.iter().map(|id| id.to_string()).collect();
        let json = serde_json::to_string(&ids).map_err(|_| VeilError::SerializationError)?;
        self.execute(
            "UPDATE space_members SET role_ids = ?1 WHERE space_id = ?2 AND user_id = ?3",
            params![json, space_id.to_string(), user_id.to_string()],
        )?;
        Ok(())
    }

    #[allow(dead_code)] // permission checks land next
    pub fn is_space_member(&self, space_id: &Uuid, user_id: &Uuid) -> VeilResult<bool> {
        let exists: i64 = self.query_row(
            "SELECT EXISTS(SELECT 1 FROM space_members WHERE space_id = ?1 AND user_id = ?2)",
            params![space_id.to_string(), user_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(exists > 0)
    }

    // ── Moderation (kick / ban / timeout) ────────────────────────────────────

    pub fn set_member_timeout(&self, space_id: &Uuid, user_id: &Uuid, until: Option<i64>) -> VeilResult<()> {
        self.execute(
            "UPDATE space_members SET timeout_until = ?1 WHERE space_id = ?2 AND user_id = ?3",
            params![until, space_id.to_string(), user_id.to_string()],
        )?;
        Ok(())
    }

    pub fn get_member_timeout(&self, space_id: &Uuid, user_id: &Uuid) -> VeilResult<Option<i64>> {
        let row = self.query_row(
            "SELECT timeout_until FROM space_members WHERE space_id = ?1 AND user_id = ?2",
            params![space_id.to_string(), user_id.to_string()],
            |row| row.get::<_, Option<i64>>(0),
        );
        Ok(row.unwrap_or(None))
    }

    pub fn ban_member(&self, space_id: &Uuid, user_id: &Uuid, banned_by: &Uuid, reason: Option<&str>) -> VeilResult<()> {
        self.execute(
            "INSERT OR REPLACE INTO banned_members (space_id, user_id, banned_by, reason, created_at)
             VALUES (?1, ?2, ?3, ?4, unixepoch())",
            params![space_id.to_string(), user_id.to_string(), banned_by.to_string(), reason],
        )?;
        Ok(())
    }

    pub fn unban_member(&self, space_id: &Uuid, user_id: &Uuid) -> VeilResult<()> {
        self.execute(
            "DELETE FROM banned_members WHERE space_id = ?1 AND user_id = ?2",
            params![space_id.to_string(), user_id.to_string()],
        )?;
        Ok(())
    }

    pub fn is_banned(&self, space_id: &Uuid, user_id: &Uuid) -> VeilResult<bool> {
        let exists: i64 = self.query_row(
            "SELECT EXISTS(SELECT 1 FROM banned_members WHERE space_id = ?1 AND user_id = ?2)",
            params![space_id.to_string(), user_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(exists > 0)
    }

    pub fn list_bans(&self, space_id: &Uuid) -> VeilResult<Vec<BanRow>> {
        self.query_map(
            r#"SELECT b.user_id, p.username, p.display_name, b.banned_by, b.reason, b.created_at
               FROM banned_members b
               LEFT JOIN user_profiles p ON p.id = b.user_id
               WHERE b.space_id = ?1 ORDER BY b.created_at DESC"#,
            params![space_id.to_string()],
            |row| {
                let banned_by: String = row.get(3)?;
                let created_raw: i64 = row.get(5)?;
                Ok(BanRow {
                    user_id: Uuid::parse_str(&row.get::<_, String>(0)?)
                        .unwrap_or_else(|_| Uuid::nil()),
                    username: row.get(1)?,
                    display_name: row.get(2)?,
                    banned_by: Uuid::parse_str(&banned_by).unwrap_or_else(|_| Uuid::nil()),
                    reason: row.get(4)?,
                    created_at: DateTime::from_timestamp(created_raw, 0).unwrap_or_else(Utc::now),
                })
            },
        )
    }

    // ── Roles ───────────────────────────────────────────────────────────────

    pub fn insert_role(
        &self,
        id: &Uuid,
        space_id: &Uuid,
        name: &str,
        color: Option<&str>,
        permissions: &Permissions,
        position: i32,
    ) -> VeilResult<()> {
        let perms_json = serde_json::to_string(permissions).map_err(|_| VeilError::SerializationError)?;
        self.execute(
            r#"INSERT INTO roles (id, space_id, name, color, permissions, position, is_default)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)"#,
            params![id.to_string(), space_id.to_string(), name, color, perms_json, position],
        )?;
        Ok(())
    }

    pub fn upsert_role(
        &self,
        id: &Uuid,
        space_id: &Uuid,
        name: &str,
        color: Option<&str>,
        permissions: &Permissions,
        position: i32,
    ) -> VeilResult<()> {
        let perms_json = serde_json::to_string(permissions).map_err(|_| VeilError::SerializationError)?;
        self.execute(
            r#"INSERT INTO roles (id, space_id, name, color, permissions, position, is_default)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)
               ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 color = excluded.color,
                 permissions = excluded.permissions,
                 position = excluded.position"#,
            params![id.to_string(), space_id.to_string(), name, color, perms_json, position],
        )?;
        Ok(())
    }

    pub fn update_role(
        &self,
        id: &Uuid,
        name: Option<&str>,
        color: Option<Option<&str>>,
        permissions: Option<&Permissions>,
        position: Option<i32>,
    ) -> VeilResult<()> {
        if let Some(n) = name {
            self.execute(
                "UPDATE roles SET name = ?1 WHERE id = ?2",
                params![n, id.to_string()],
            )?;
        }
        if let Some(c) = color {
            self.execute(
                "UPDATE roles SET color = ?1 WHERE id = ?2",
                params![c, id.to_string()],
            )?;
        }
        if let Some(p) = permissions {
            let perms_json = serde_json::to_string(p).map_err(|_| VeilError::SerializationError)?;
            self.execute(
                "UPDATE roles SET permissions = ?1 WHERE id = ?2",
                params![perms_json, id.to_string()],
            )?;
        }
        if let Some(pos) = position {
            self.execute(
                "UPDATE roles SET position = ?1 WHERE id = ?2",
                params![pos, id.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn reorder_roles(&self, space_id: &Uuid, ordered_role_ids: &[Uuid]) -> VeilResult<()> {
        let total = ordered_role_ids.len() as i32;
        for (idx, role_id) in ordered_role_ids.iter().enumerate() {
            let pos = total - (idx as i32);
            self.execute(
                "UPDATE roles SET position = ?1 WHERE id = ?2 AND space_id = ?3",
                params![pos, role_id.to_string(), space_id.to_string()],
            )?;
        }
        Ok(())
    }

    pub fn next_role_position(&self, space_id: &Uuid) -> VeilResult<i32> {
        let pos: Option<i32> = self.query_row(
            "SELECT MAX(position) FROM roles WHERE space_id = ?1",
            params![space_id.to_string()],
            |row| row.get(0),
        )?;
        Ok(pos.unwrap_or(0) + 1)
    }

    pub fn get_role(&self, id: &Uuid) -> VeilResult<Option<RoleRow>> {
        let rows = self.query_map(
            "SELECT id, space_id, name, color, permissions, position, is_default FROM roles WHERE id = ?1",
            params![id.to_string()],
            row_to_role_row,
        )?;
        Ok(rows.into_iter().next())
    }

    pub fn list_roles(&self, space_id: &Uuid) -> VeilResult<Vec<RoleRow>> {
        self.query_map(
            "SELECT id, space_id, name, color, permissions, position, is_default FROM roles WHERE space_id = ?1 ORDER BY position DESC, name ASC",
            params![space_id.to_string()],
            row_to_role_row,
        )
    }

    pub fn delete_role(&self, id: &Uuid) -> VeilResult<()> {
        self.execute("DELETE FROM roles WHERE id = ?1", params![id.to_string()])?;
        Ok(())
    }

    pub fn update_channel_overrides(
        &self,
        channel_id: &Uuid,
        overrides: &[crate::models::channel::PermissionOverride],
    ) -> VeilResult<()> {
        let json = serde_json::to_string(overrides).map_err(|_| VeilError::SerializationError)?;
        self.execute(
            "UPDATE channels SET permission_overrides = ?1, updated_at = unixepoch() WHERE id = ?2",
            params![json, channel_id.to_string()],
        )?;
        Ok(())
    }

    // ── Invites ─────────────────────────────────────────────────────────────

    pub fn insert_invite(
        &self,
        id: &Uuid,
        code: &str,
        space_id: &Uuid,
        creator_id: &Uuid,
        max_uses: Option<u32>,
        expires_at: Option<DateTime<Utc>>,
    ) -> VeilResult<()> {
        self.execute(
            r#"INSERT INTO invites (id, code, space_id, creator_id, max_uses, used_count, expires_at)
               VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)"#,
            params![
                id.to_string(),
                code,
                space_id.to_string(),
                creator_id.to_string(),
                max_uses.map(|m| m as i64),
                expires_at.map(|dt| dt.timestamp()),
            ],
        )?;
        Ok(())
    }

pub fn get_invite_by_code(&self, code: &str) -> VeilResult<Option<InviteRow>> {
        let rows = self.query_map(
            r#"SELECT id, code, space_id, max_uses, used_count, expires_at
               FROM invites WHERE code = ?1"#,
            params![code],
            row_to_invite_row,
        )?;
        Ok(rows.into_iter().next())
    }

    // ── Friends ─────────────────────────────────────────────────────────────

    pub fn upsert_friend(&self, user_id: &Uuid, friend_id: &Uuid, status: &str) -> VeilResult<()> {
        self.execute(
            r#"INSERT INTO friends (user_id, friend_id, status) VALUES (?1, ?2, ?3)
               ON CONFLICT(user_id, friend_id) DO UPDATE SET status = ?3"#,
            params![user_id.to_string(), friend_id.to_string(), status],
        )?;
        Ok(())
    }

    pub fn list_friends(&self, user_id: &Uuid) -> VeilResult<Vec<FriendRow>> {
        self.query_map(
            r#"SELECT f.friend_id, COALESCE(p.username, 'kullanici'), COALESCE(p.display_name, 'Kullanıcı'), p.avatar_hash, f.status, COALESCE(p.online_status, 'offline')
               FROM friends f
               LEFT JOIN user_profiles p ON p.id = f.friend_id
               WHERE f.user_id = ?1 ORDER BY p.display_name ASC"#,
            params![user_id.to_string()],
            row_to_friend_row,
        )
    }

    #[allow(dead_code)] // unfriend flow lands next
    pub fn remove_friend(&self, user_id: &Uuid, friend_id: &Uuid) -> VeilResult<()> {
        self.execute(
            "DELETE FROM friends WHERE user_id = ?1 AND friend_id = ?2",
            params![user_id.to_string(), friend_id.to_string()],
        )?;
        Ok(())
    }

    #[allow(dead_code)] // DM gating lands next
    pub fn get_friend_status(&self, user_id: &Uuid, friend_id: &Uuid) -> VeilResult<Option<String>> {
        let rows = self.query_map(
            "SELECT status FROM friends WHERE user_id = ?1 AND friend_id = ?2",
            params![user_id.to_string(), friend_id.to_string()],
            |row| row.get::<_, String>(0),
        )?;
        Ok(rows.into_iter().next())
    }

    // ── Channel members (DM / group-DM membership) ──────────────────────────

    pub fn add_channel_member(&self, channel_id: &Uuid, user_id: &Uuid) -> VeilResult<()> {
        self.execute(
            "INSERT OR IGNORE INTO channel_members (channel_id, user_id) VALUES (?1, ?2)",
            params![channel_id.to_string(), user_id.to_string()],
        )?;
        Ok(())
    }

    pub fn list_channel_members(&self, channel_id: &Uuid) -> VeilResult<Vec<Uuid>> {
        self.query_map(
            "SELECT user_id FROM channel_members WHERE channel_id = ?1",
            params![channel_id.to_string()],
            |row| {
                let s: String = row.get(0)?;
                Ok(Uuid::parse_str(&s).unwrap_or_else(|_| Uuid::nil()))
            },
        )
    }

    /// Find an existing 1:1 DM channel shared by `me` and `peer` — a channel
    /// with exactly two members, both present.
    pub fn find_dm_with(&self, me: &Uuid, peer: &Uuid) -> VeilResult<Option<Uuid>> {
        let rows = self.query_map(
            r#"SELECT cm.channel_id
               FROM channel_members cm
               WHERE cm.user_id IN (?1, ?2)
                 AND cm.channel_id IN (
                     SELECT channel_id FROM channel_members
                     GROUP BY channel_id HAVING COUNT(*) = 2
                 )
               GROUP BY cm.channel_id
               HAVING COUNT(*) = 2
               LIMIT 1"#,
            params![me.to_string(), peer.to_string()],
            |row| row.get::<_, String>(0),
        )?;
        Ok(rows.into_iter().next().and_then(|s| Uuid::parse_str(&s).ok()))
    }

    pub fn channel_type_of(&self, channel_id: &Uuid) -> VeilResult<Option<String>> {
        let rows = self.query_map(
            "SELECT channel_type FROM channels WHERE id = ?1",
            params![channel_id.to_string()],
            |row| row.get::<_, String>(0),
        )?;
        Ok(rows.into_iter().next())
    }

    // ── User profiles (local cache of remote users) ─────────────────────────

    pub fn get_profile_by_username(&self, username: &str) -> VeilResult<Option<ProfileRow>> {
        let clean = username.trim().trim_start_matches('@');
        let rows = self.query_map(
            r#"SELECT id, username, display_name, avatar_hash FROM user_profiles
               WHERE lower(username) = lower(?1)
                  OR lower(display_name) = lower(?1)
                  OR id = ?1
               ORDER BY (lower(username) = lower(?1)) DESC, (id = ?1) DESC
               LIMIT 1"#,
            params![clean],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )?;
        let Some(row) = rows.into_iter().next() else {
            return Ok(None);
        };
        Ok(Some((
            Uuid::parse_str(&row.0).unwrap_or_else(|_| Uuid::nil()),
            row.1,
            row.2,
            row.3,
        )))
    }

    /// X25519 public key of a cached remote profile (DM ratchet key agreement).
    pub fn get_profile_dh_public(&self, user_id: &Uuid) -> VeilResult<Option<String>> {
        let rows = self.query_map(
            "SELECT dh_public_key FROM user_profiles WHERE id = ?1",
            params![user_id.to_string()],
            |row| row.get::<_, String>(0),
        )?;
        Ok(rows.into_iter().next().filter(|k| !k.is_empty()))
    }

    pub fn upsert_profile(
        &self,
        id: &Uuid,
        username: &str,
        display_name: &str,
        avatar_hash: Option<&str>,
        dh_public_key: Option<&str>,
        signing_public_key: Option<&str>,
        banner_hash: Option<&str>,
        bio_ciphertext: Option<&str>,
        custom_status: Option<&str>,
    ) -> VeilResult<()> {
        self.execute(
            r#"INSERT INTO user_profiles (id, username, display_name, avatar_hash, dh_public_key, signing_public_key, banner_hash, bio_ciphertext, custom_status, updated_at)
               VALUES (?1, ?2, ?3, ?4, COALESCE(?5, ''), COALESCE(?6, ''), ?7, ?8, ?9, unixepoch())
               ON CONFLICT(id) DO UPDATE SET
                 username = CASE WHEN ?2 != '' THEN ?2 ELSE user_profiles.username END,
                 display_name = CASE WHEN ?3 != '' THEN ?3 ELSE user_profiles.display_name END,
                 avatar_hash = COALESCE(?4, user_profiles.avatar_hash),
                 dh_public_key = COALESCE(?5, user_profiles.dh_public_key),
                 signing_public_key = COALESCE(?6, user_profiles.signing_public_key),
                 banner_hash = COALESCE(?7, user_profiles.banner_hash),
                 bio_ciphertext = COALESCE(?8, user_profiles.bio_ciphertext),
                 custom_status = COALESCE(?9, user_profiles.custom_status),
                 updated_at = unixepoch()"#,
            params![
                id.to_string(),
                username,
                display_name,
                avatar_hash,
                dh_public_key,
                signing_public_key,
                banner_hash,
                bio_ciphertext,
                custom_status,
            ],
        )?;
        Ok(())
    }

    pub fn set_user_profile_banner(&self, user_id: &Uuid, banner_hash: Option<&str>) -> VeilResult<()> {
        self.execute(
            "UPDATE user_profiles SET banner_hash = ?1, updated_at = unixepoch() WHERE id = ?2",
            params![banner_hash, user_id.to_string()],
        )?;
        Ok(())
    }

    pub fn update_presence(&self, user_id: &Uuid, status: &str) -> VeilResult<()> {
        self.execute(
            "UPDATE user_profiles SET online_status = ?1, updated_at = unixepoch() WHERE id = ?2",
            params![status, user_id.to_string()],
        )?;
        Ok(())
    }

    /// "Hakkımda" metnini (şifreli, base64) yerel profile yazar.
    pub fn update_profile_bio(&self, user_id: &Uuid, bio_ciphertext: Option<&str>) -> VeilResult<()> {
        self.execute(
            "UPDATE user_profiles SET bio_ciphertext = ?1, updated_at = unixepoch() WHERE id = ?2",
            params![bio_ciphertext, user_id.to_string()],
        )?;
        Ok(())
    }

    /// Profil satırı: görünen ad, kullanıcı adı, avatar, şifreli bio, durum.
    pub fn get_profile_by_id(
        &self,
        user_id: &Uuid,
    ) -> VeilResult<Option<(String, String, Option<String>, Option<String>, String)>> {
        let rows = self.query_map(
            "SELECT username, display_name, avatar_hash, bio_ciphertext, online_status
             FROM user_profiles WHERE id = ?1",
            params![user_id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )?;
        Ok(rows.into_iter().next())
    }

    pub fn update_custom_status(&self, user_id: &Uuid, status: Option<&str>) -> VeilResult<()> {
        self.execute(
            "UPDATE user_profiles SET custom_status = ?1, updated_at = unixepoch() WHERE id = ?2",
            params![status, user_id.to_string()],
        )?;
        self.execute(
            "UPDATE local_identity SET custom_status = ?1 WHERE id = ?2",
            params![status, user_id.to_string()],
        )?;
        Ok(())
    }

    pub fn update_local_bio(&self, user_id: &Uuid, bio: Option<&str>) -> VeilResult<()> {
        self.execute(
            "UPDATE local_identity SET bio = ?1 WHERE id = ?2",
            params![bio, user_id.to_string()],
        )?;
        Ok(())
    }

    /// Tam profil satırı: banner_hash ve custom_status dahil.
    pub fn get_profile_full_by_id(
        &self,
        user_id: &Uuid,
    ) -> VeilResult<Option<(String, String, Option<String>, Option<String>, Option<String>, String, Option<String>)>> {
        let rows = self.query_map(
            "SELECT p.username, p.display_name, p.avatar_hash, COALESCE(p.banner_hash, i.banner_hash) AS banner_hash, COALESCE(p.bio_ciphertext, i.bio) AS bio, p.online_status, COALESCE(p.custom_status, i.custom_status) AS custom_status
             FROM user_profiles p
             LEFT JOIN local_identity i ON i.id = p.id
             WHERE p.id = ?1",
            params![user_id.to_string()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                ))
            },
        )?;
        Ok(rows.into_iter().next())
    }

    /// Kayıt/hesap açma tarihi (unix saniye). Satır yoksa None.
    pub fn get_profile_created_at(&self, user_id: &Uuid) -> VeilResult<Option<i64>> {
        let rows = self.query_map(
            "SELECT created_at FROM user_profiles WHERE id = ?1",
            params![user_id.to_string()],
            |row| row.get::<_, i64>(0),
        )?;
        Ok(rows.into_iter().next())
    }
}

// ── Row mapping helpers ─────────────────────────────────────────────────────

fn row_to_channel_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ChannelRow> {
    Ok(ChannelRow {
        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_else(|_| Uuid::nil()),
        space_id: row
            .get::<_, Option<String>>(1)?
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok()),
        name: row.get(2)?,
        channel_type: row.get(3)?,
        position: row.get(4)?,
        is_nsfw: row.get::<_, i32>(5)? != 0,
        is_e2ee: row.get::<_, i32>(6)? != 0,
        unread_count: row.get::<_, i64>(7)?.max(0) as u32,
        mentioned: row.get::<_, i32>(8)? != 0,
        last_message_id: row
            .get::<_, Option<String>>(9)?
            .as_deref()
            .and_then(|s| Uuid::parse_str(s).ok()),
    })
}

fn row_to_space_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SpaceRow> {
    let my_roles_json: String = row.get(6)?;
    Ok(SpaceRow {
        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_else(|_| Uuid::nil()),
        name: row.get(1)?,
        icon_hash: row.get(2)?,
        owner_id: Uuid::parse_str(&row.get::<_, String>(3)?).unwrap_or_else(|_| Uuid::nil()),
        member_count: row.get::<_, i64>(4)?.max(0) as u32,
        is_owner: row.get::<_, i32>(5)? != 0,
        my_roles: parse_uuids(&my_roles_json),
        banner_hash: row.get(7)?,
        description: row.get(8)?,
        custom_link: row.get(9)?,
    })
}

fn row_to_role_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RoleRow> {
    let perms_json: String = row.get(4)?;
    Ok(RoleRow {
        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_else(|_| Uuid::nil()),
        space_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_else(|_| Uuid::nil()),
        name: row.get(2)?,
        color: row.get(3)?,
        permissions: serde_json::from_str(&perms_json).unwrap_or_default(),
        position: row.get::<_, Option<i32>>(5)?.unwrap_or(0),
        is_default: row.get::<_, Option<i32>>(6)?.unwrap_or(0) != 0,
    })
}

fn row_to_invite_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<InviteRow> {
    Ok(InviteRow {
        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_else(|_| Uuid::nil()),
        code: row.get(1)?,
        space_id: Uuid::parse_str(&row.get::<_, String>(2)?).unwrap_or_else(|_| Uuid::nil()),
        max_uses: row.get::<_, Option<i64>>(3)?.map(|m| m.max(0) as u32),
        used_count: row.get::<_, i64>(4)?.max(0) as u32,
        expires_at: row
            .get::<_, Option<i64>>(5)?
            .and_then(|t| DateTime::from_timestamp(t, 0)),
    })
}

fn row_to_member_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<MemberRow> {
    let role_ids_json: Option<String> = row.get(4)?;
    let role_ids_str = role_ids_json.unwrap_or_else(|| "[]".to_string());
    let user_id_str: String = row.get(0)?;
    let username: Option<String> = row.get(1)?;
    let display_name: Option<String> = row.get(2)?;
    let avatar_hash: Option<String> = row.get(3)?;
    let online_status: Option<String> = row.get(5)?;

    Ok(MemberRow {
        user_id: Uuid::parse_str(&user_id_str).unwrap_or_else(|_| Uuid::nil()),
        username: username.unwrap_or_else(|| "Kullanıcı".to_string()),
        display_name: display_name.unwrap_or_else(|| "Kullanıcı".to_string()),
        avatar_hash,
        role_ids: parse_uuids(&role_ids_str),
        online_status: online_status.unwrap_or_else(|| "offline".to_string()),
    })
}

fn row_to_friend_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FriendRow> {
    Ok(FriendRow {
        user_id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_else(|_| Uuid::nil()),
        username: row.get(1)?,
        display_name: row.get(2)?,
        avatar_hash: row.get(3)?,
        status: row.get(4)?,
        online_status: row.get(5)?,
    })
}
