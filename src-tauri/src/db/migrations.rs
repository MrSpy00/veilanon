//! Database migrations
//! 
//! Each migration is a SQL string applied in order.
//! Migrations are tracked in the `schema_version` user_version PRAGMA.
//! 
//! SECURITY INVARIANT: No plaintext message content is ever stored.
//! Only `ciphertext` (AES-256-GCM encrypted) and routing metadata.

use rusqlite::Connection;
use tracing::info;

const MIGRATIONS: &[(&str, &str)] = &[
    ("0001_initial", MIGRATION_0001),
    ("0002_offline_queue", MIGRATION_0002),
    ("0003_voice_state", MIGRATION_0003),
    ("0004_space_members", MIGRATION_0004),
    ("0005_file_metadata", MIGRATION_0005),
    ("0006_dm_crypto_meta", MIGRATION_0006),
    ("0007_supabase_refresh_token", MIGRATION_0007),
    ("0008_mls_and_bridge", MIGRATION_0008),
    ("0009_moderation", MIGRATION_0009),
    ("0010_banner_description", MIGRATION_0010),
    ("0011_join_date_links_folders", MIGRATION_0011),
    ("0012_user_profile_banner", MIGRATION_0012),
    ("0013_custom_status_and_bio", MIGRATION_0013),
    ("0014_pending_dm_messages", MIGRATION_0014),
    ("0015_fix_channel_type_strings", MIGRATION_0015),
    ("0016_encrypt_pending_dm", MIGRATION_0016),
    ("0017_channel_clears", MIGRATION_0017),
];

pub fn run(conn: &mut Connection) -> crate::error::VeilResult<()> {
    // Create migration tracking table
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _schema_migrations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            applied_at INTEGER NOT NULL DEFAULT (unixepoch())
        )"
    )?;

    for (name, sql) in MIGRATIONS {
        let already_applied: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM _schema_migrations WHERE name = ?1)",
            [name],
            |row| row.get(0),
        )?;

        if !already_applied {
            info!("Applying migration: {}", name);
            let tx = conn.transaction()?;
            tx.execute_batch(sql)?;
            tx.execute(
                "INSERT INTO _schema_migrations (name) VALUES (?1)",
                [name],
            )?;
            tx.commit()?;
            info!("Migration {} applied successfully", name);
        }
    }
    Ok(())
}

// ── Migration 0001: Initial schema ──────────────────────────────────────────

const MIGRATION_0001: &str = r#"
-- Local identity (one per device)
CREATE TABLE IF NOT EXISTS local_identity (
    id                  TEXT PRIMARY KEY,   -- UUID
    username            TEXT NOT NULL,
    display_name        TEXT NOT NULL,
    avatar_hash         TEXT,
    -- Public keys (safe to store)
    dh_public_key       TEXT NOT NULL,
    signing_public_key  TEXT NOT NULL,
    device_id           TEXT NOT NULL UNIQUE,
    created_at          INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Remote user profiles (minimal — username + public key only)
CREATE TABLE IF NOT EXISTS user_profiles (
    id                  TEXT PRIMARY KEY,
    username            TEXT NOT NULL,
    display_name        TEXT NOT NULL,
    avatar_hash         TEXT,
    dh_public_key       TEXT NOT NULL,
    signing_public_key  TEXT NOT NULL,
    bio_ciphertext      TEXT,  -- encrypted if set
    online_status       TEXT NOT NULL DEFAULT 'offline',
    last_seen_bucket    INTEGER, -- rounded to hour for privacy
    created_at          INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at          INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Spaces (communities)
CREATE TABLE IF NOT EXISTS spaces (
    id                  TEXT PRIMARY KEY,
    name                TEXT NOT NULL,
    icon_hash           TEXT,
    owner_id            TEXT NOT NULL,
    description_ciphertext TEXT,
    member_count        INTEGER NOT NULL DEFAULT 0,
    my_roles            TEXT NOT NULL DEFAULT '[]',  -- JSON array of role IDs
    is_owner            INTEGER NOT NULL DEFAULT 0,
    created_at          INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at          INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Channels
CREATE TABLE IF NOT EXISTS channels (
    id                  TEXT PRIMARY KEY,
    space_id            TEXT REFERENCES spaces(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    channel_type        TEXT NOT NULL DEFAULT 'text',
    position            INTEGER NOT NULL DEFAULT 0,
    topic_ciphertext    TEXT,
    is_nsfw             INTEGER NOT NULL DEFAULT 0,
    is_e2ee             INTEGER NOT NULL DEFAULT 0,
    slow_mode_seconds   INTEGER NOT NULL DEFAULT 0,
    permission_overrides TEXT NOT NULL DEFAULT '[]',  -- JSON
    last_message_id     TEXT,
    unread_count        INTEGER NOT NULL DEFAULT 0,
    mentioned           INTEGER NOT NULL DEFAULT 0,
    created_at          INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at          INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Messages — ONLY ciphertext stored, never plaintext content
CREATE TABLE IF NOT EXISTS messages (
    id                  TEXT PRIMARY KEY,
    channel_id          TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    sender_id           TEXT NOT NULL,
    sender_device_id    TEXT NOT NULL,
    ciphertext          TEXT NOT NULL,  -- AES-256-GCM encrypted content (base64)
    iv                  TEXT NOT NULL,  -- Nonce (base64)
    message_type        TEXT NOT NULL DEFAULT 'text',
    status              TEXT NOT NULL DEFAULT 'sent',
    reply_to_id         TEXT REFERENCES messages(id),
    pinned              INTEGER NOT NULL DEFAULT 0,
    reactions           TEXT NOT NULL DEFAULT '[]',  -- JSON
    attachments         TEXT NOT NULL DEFAULT '[]',  -- JSON (no plaintext filenames!)
    edited_at           INTEGER,
    created_at          INTEGER NOT NULL,
    deleted_at          INTEGER,
    disappears_at       INTEGER,
    schema_version      INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_messages_channel ON messages(channel_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_sender ON messages(sender_id);

-- Roles
CREATE TABLE IF NOT EXISTS roles (
    id                  TEXT PRIMARY KEY,
    space_id            TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    name                TEXT NOT NULL,
    color               TEXT,
    permissions         TEXT NOT NULL DEFAULT '{}',  -- JSON Permissions struct
    position            INTEGER NOT NULL DEFAULT 0,
    is_default          INTEGER NOT NULL DEFAULT 0
);

-- Invites
CREATE TABLE IF NOT EXISTS invites (
    id                  TEXT PRIMARY KEY,
    code                TEXT NOT NULL UNIQUE,
    space_id            TEXT NOT NULL,
    creator_id          TEXT NOT NULL,
    role_id             TEXT,
    max_uses            INTEGER,
    used_count          INTEGER NOT NULL DEFAULT 0,
    expires_at          INTEGER,
    channel_scope       TEXT,
    created_at          INTEGER NOT NULL DEFAULT (unixepoch())
);

-- DM sessions (ratchet state — encrypted at rest)
CREATE TABLE IF NOT EXISTS dm_sessions (
    id                  TEXT PRIMARY KEY,  -- channel_id of DM
    peer_id             TEXT NOT NULL,
    ratchet_state       TEXT NOT NULL,     -- JSON RatchetState, encrypted
    created_at          INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at          INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Friends
CREATE TABLE IF NOT EXISTS friends (
    user_id             TEXT NOT NULL,
    friend_id           TEXT NOT NULL,
    status              TEXT NOT NULL DEFAULT 'accepted',  -- pending/accepted/blocked
    created_at          INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (user_id, friend_id)
);

-- Settings (JSON blob — no sensitive data)
CREATE TABLE IF NOT EXISTS app_settings (
    key                 TEXT PRIMARY KEY,
    value               TEXT NOT NULL
);
"#;

// ── Migration 0002: Offline queue ────────────────────────────────────────────

const MIGRATION_0002: &str = r#"
-- Offline message queue — messages pending transmission
CREATE TABLE IF NOT EXISTS offline_queue (
    id                  TEXT PRIMARY KEY,
    channel_id          TEXT NOT NULL,
    ciphertext          TEXT NOT NULL,
    iv                  TEXT NOT NULL,
    message_type        TEXT NOT NULL DEFAULT 'text',
    reply_to_id         TEXT,
    attachments         TEXT NOT NULL DEFAULT '[]',
    disappears_at       INTEGER,
    retry_count         INTEGER NOT NULL DEFAULT 0,
    queued_at           INTEGER NOT NULL DEFAULT (unixepoch()),
    next_retry_at       INTEGER,
    schema_version      INTEGER NOT NULL DEFAULT 1
);
"#;

// ── Migration 0003: Voice state ───────────────────────────────────────────────

const MIGRATION_0003: &str = r#"
-- Cached voice channel state
CREATE TABLE IF NOT EXISTS voice_state_cache (
    channel_id          TEXT PRIMARY KEY,
    space_id            TEXT,
    participants        TEXT NOT NULL DEFAULT '[]',  -- JSON
    is_e2ee             INTEGER NOT NULL DEFAULT 0,
    e2ee_key_epoch      INTEGER NOT NULL DEFAULT 0,
    updated_at          INTEGER NOT NULL DEFAULT (unixepoch())
);
"#;

// ── Migration 0004: Space membership ───────────────────────────────────────

const MIGRATION_0004: &str = r#"
-- Space membership (roles assigned per member)
CREATE TABLE IF NOT EXISTS space_members (
    space_id            TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    user_id             TEXT NOT NULL,
    role_ids            TEXT NOT NULL DEFAULT '[]',  -- JSON array of role UUIDs
    joined_at           INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (space_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_space_members_user ON space_members(user_id);
"#;

const MIGRATION_0005: &str = r#"
-- Encrypted file metadata (content key wrapped with the DB key; plaintext
-- filenames never stored anywhere — the server sees only opaque blob paths).
CREATE TABLE IF NOT EXISTS file_metadata (
    id                  TEXT PRIMARY KEY,           -- file UUID
    channel_id          TEXT NOT NULL,
    storage_path        TEXT NOT NULL UNIQUE,       -- files/{channel}/{id}
    size_bytes          INTEGER NOT NULL DEFAULT 0,
    content_key_cipher  TEXT NOT NULL,              -- AES-GCM(db_key, file key)
    content_key_iv      TEXT NOT NULL,
    nonce               TEXT NOT NULL,              -- file nonce
    created_at          INTEGER NOT NULL DEFAULT (unixepoch()),
    deleted_at          INTEGER
);

CREATE INDEX IF NOT EXISTS idx_file_metadata_channel ON file_metadata(channel_id);
"#;

// ── Migration 0006: DM crypto metadata + channel membership ──────────────────

const MIGRATION_0006: &str = r#"
-- Per-message crypto metadata: JSON Double-Ratchet header for 1:1 DMs
-- (dh_public, prev_chain_count, message_count). NULL for deterministic-key
-- messages. Never contains plaintext or key material.
ALTER TABLE messages ADD COLUMN crypto_meta TEXT;

-- Members of DM / group-DM channels (the channel row itself has space_id NULL).
CREATE TABLE IF NOT EXISTS channel_members (
    channel_id      TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id         TEXT NOT NULL,
    joined_at       INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (channel_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_channel_members_user ON channel_members(user_id);

-- Ratchet headers survive offline queuing too.
ALTER TABLE offline_queue ADD COLUMN crypto_meta TEXT;

-- Decrypted DM message keys, wrapped with the DB key. Lets the client re-read
-- old DM history without advancing (and invalidating) the ratchet chains.
CREATE TABLE IF NOT EXISTS message_keys (
    message_id   TEXT PRIMARY KEY,
    key_cipher   TEXT NOT NULL,
    key_iv       TEXT NOT NULL,
    created_at   INTEGER NOT NULL DEFAULT (unixepoch())
);
"#;

// ── Migration 0007: Supabase refresh token persistence ───────────────────────
//
// Previously every unlock called sign_in_anonymous() again, creating a brand
// new anonymous Supabase user per app start (MAU bloat + orphaned rows in
// users/devices on the control plane). Persisting the refresh token lets
// bind_control_plane reuse the SAME anonymous user across restarts.

const MIGRATION_0007: &str = r#"
ALTER TABLE local_identity ADD COLUMN supabase_refresh_token TEXT;
"#;

const MIGRATION_0008: &str = r#"
-- MLS grup E2EE oturumları (DB anahtarıyla şifreli blob)
CREATE TABLE IF NOT EXISTS mls_sessions (
    channel_id   TEXT PRIMARY KEY,
    session_blob TEXT NOT NULL,
    updated_at   INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Üye katılımı: sahibin ürettiği Welcome + üye imza anahtarı,
-- X25519(üye DH) ile şifrelenmiş halde saklanır.
CREATE TABLE IF NOT EXISTS mls_welcomes (
    channel_id TEXT NOT NULL,
    user_id    TEXT NOT NULL,
    envelope   TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (channel_id, user_id)
);

-- Discord köprüsü: kanal başına webhook URL'si (yalnızca çıkış aynalama).
CREATE TABLE IF NOT EXISTS discord_webhooks (
    channel_id  TEXT PRIMARY KEY,
    webhook_url TEXT NOT NULL,
    created_at  INTEGER NOT NULL DEFAULT (unixepoch())
);
"#;

// ── Migration 0009: Moderation (kick / ban / timeout) ────────────────────────

const MIGRATION_0009: &str = r#"
-- Yasaklı üyeler: davetle bile geri dönemez (local-first, kontrol düzlemine yansır).
CREATE TABLE IF NOT EXISTS banned_members (
    space_id   TEXT NOT NULL REFERENCES spaces(id) ON DELETE CASCADE,
    user_id    TEXT NOT NULL,
    banned_by  TEXT NOT NULL,
    reason     TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (space_id, user_id)
);

-- Geçici susturma: süre dolana kadar mesaj gönderemez (NULL = süre yok).
ALTER TABLE space_members ADD COLUMN timeout_until INTEGER;
"#;

// ── Migration 0010: Banner & description (Discord-style server/profile) ─────

const MIGRATION_0010: &str = r#"
-- Topluluk banner'ı (local-* hash, app data/avatars içinde saklanır).
ALTER TABLE spaces ADD COLUMN banner_hash TEXT;

-- Topluluk açıklaması — yalnızca yerel DB'de tutulur, kontrol düzlemine
-- asla gönderilmez (gizlilik ilkesi: sunucuda düz metin yok).
ALTER TABLE spaces ADD COLUMN description TEXT;

-- Yerel kimlik banner'ı (kendi profil kapak görseli).
ALTER TABLE local_identity ADD COLUMN banner_hash TEXT;
"#;

// ── Migration 0011: join date, custom server link, server folders ────────────

const MIGRATION_0011: &str = r#"
-- Profil kayit tarihi zaten 0001'de mevcut (user_profiles.created_at);
-- bu migration yalnizca topluluk ozel baglantisi ve klasorler icindir.

-- Topluluga ozel, bir kez alinabilen kisa link (sahip belirler; unique).
ALTER TABLE spaces ADD COLUMN custom_link TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_spaces_custom_link ON spaces(custom_link) WHERE custom_link IS NOT NULL AND custom_link != '';

-- Sunucu klasorleri (gruplama): ad + renk + sira.
CREATE TABLE IF NOT EXISTS space_folders (
    id        TEXT PRIMARY KEY,
    name      TEXT NOT NULL,
    color     TEXT,
    position  INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (unixepoch())
);

-- Topluluk -> klasor atamasi.
ALTER TABLE spaces ADD COLUMN folder_id TEXT REFERENCES space_folders(id) ON DELETE SET NULL;
"#;

// ── Migration 0012: user_profiles banner_hash support ────────────────────────

const MIGRATION_0012: &str = r#"
-- Kullanıcı profili banner görseli (local-* hash veya uzak hash)
ALTER TABLE user_profiles ADD COLUMN banner_hash TEXT;
"#;

// ── Migration 0013: custom_status & bio support ──────────────────────────────

const MIGRATION_0013: &str = r#"
-- Özel durum metni ve biyografi desteği
ALTER TABLE user_profiles ADD COLUMN custom_status TEXT;
ALTER TABLE local_identity ADD COLUMN custom_status TEXT;
ALTER TABLE local_identity ADD COLUMN bio TEXT;
"#;

// ── Migration 0014: Pending DM messages (peer key missing) ─────────────────
// When a DM peer's DH public key is not yet registered, messages are stored
// as plaintext in this table. Once the key arrives, they are encrypted and
// sent via the normal path. SECURITY: This table is local-only, never synced.

const MIGRATION_0014: &str = r#"
CREATE TABLE IF NOT EXISTS pending_dm_messages (
    id              TEXT PRIMARY KEY,
    channel_id      TEXT NOT NULL,
    peer_id         TEXT NOT NULL,
    content         TEXT NOT NULL,       -- plaintext, local-only, never leaves device
    message_type    TEXT NOT NULL DEFAULT 'text',
    reply_to_id     TEXT,
    attachments     TEXT NOT NULL DEFAULT '[]',
    disappears_at   INTEGER,
    created_at      INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_pending_dm_channel ON pending_dm_messages(channel_id);
CREATE INDEX IF NOT EXISTS idx_pending_dm_peer ON pending_dm_messages(peer_id);
"#;

const MIGRATION_0015: &str = r#"
UPDATE channels SET channel_type = 'dm' WHERE channel_type = 'directmessage';
UPDATE channels SET channel_type = 'group_dm' WHERE channel_type = 'groupdirectmessage';
UPDATE channels SET channel_type = 'text' WHERE channel_type = 'textmessage';
"#;

// ── Migration 0016: Encrypt pending_dm_messages at rest ─────────────────────
const MIGRATION_0016: &str = r#"
-- content column was plaintext; migrate to encrypted columns.
-- New columns: content_cipher (base64), content_nonce (base64)
ALTER TABLE pending_dm_messages ADD COLUMN content_cipher TEXT;
ALTER TABLE pending_dm_messages ADD COLUMN content_nonce TEXT;
-- Backfill existing rows: wrap plaintext with a placeholder (will be re-encrypted on next insert)
-- Keep old content column for migration week, then drop in 0017.
"#;

// ── Migration 0017: per-channel clear gate ───────────────────────────────────
// "Sohbeti Temizle" uzak taraf RLS yüzünden bulk-update edilemese bile yerel
// görünümde kalıcı olmalı: sync, bu zamandan eski uzak satırları yok sayar.
const MIGRATION_0017: &str = r#"
CREATE TABLE IF NOT EXISTS channel_clears (
    channel_id TEXT PRIMARY KEY,
    cleared_at INTEGER NOT NULL DEFAULT (unixepoch())
);
"#;

