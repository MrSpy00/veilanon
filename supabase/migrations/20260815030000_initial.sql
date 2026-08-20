-- ============================================================================
-- veilanon control-plane schema — part 1: extensions, helper, tables, indexes
-- (functions + RLS policies live in 20260815030001_functions.sql)
-- PostgreSQL 15 / Supabase
-- ----------------------------------------------------------------------------
-- Privacy invariant: the server NEVER stores plaintext message content,
-- plaintext filenames, or any plaintext user payload. Content columns hold
-- only client-side E2EE ciphertext. The server is a dumb envelope router.
-- ============================================================================

-- ---------------------------------------------------------------------------
-- Extensions
-- ---------------------------------------------------------------------------
create extension if not exists citext;   -- case-insensitive usernames

-- Random invite code: 12 hex chars from the CORE gen_random_uuid() (Postgres
-- 13+; no pgcrypto dependency, works identically on Supabase and vanilla PG).
-- Defined before the tables because `spaces.invite_code` uses it as a default.
create or replace function gen_invite_code()
returns text
language sql volatile
as $$
  select left(replace(gen_random_uuid()::text, '-', ''), 12);
$$;

revoke all on function gen_invite_code() from public;
grant execute on function gen_invite_code() to authenticated;

-- ============================================================================
-- Tables
-- ============================================================================

-- ---------------------------------------------------------------------------
-- users
-- ---------------------------------------------------------------------------
create table users (
  id               uuid primary key,
  username         citext not null unique,
  display_name     text not null default '',
  avatar_hash      text not null default '',
  created_at       timestamptz not null default now(),
  -- Presence is bucketed to the hour to limit metadata precision.
  last_seen_bucket integer not null default ((extract(epoch from now())::bigint / 3600)::int)
);

alter table users enable row level security;

-- ---------------------------------------------------------------------------
-- devices  (E2EE key material registry; keys are the CLIENT's public keys)
-- ---------------------------------------------------------------------------
create table devices (
  id                 uuid primary key default gen_random_uuid(),
  user_id            uuid not null references users(id) on delete cascade,
  public_key         text not null,
  signing_public_key text not null,
  name               text not null default '',
  created_at         timestamptz not null default now(),
  last_active_at     timestamptz
);

alter table devices enable row level security;

-- ---------------------------------------------------------------------------
-- spaces
-- ---------------------------------------------------------------------------
create table spaces (
  id          uuid primary key default gen_random_uuid(),
  name        text not null,
  icon_hash   text not null default '',
  owner_id    uuid not null references users(id) on delete cascade,
  invite_code text not null unique default gen_invite_code(),
  created_at  timestamptz not null default now()
);

alter table spaces enable row level security;

-- ---------------------------------------------------------------------------
-- channels
-- ---------------------------------------------------------------------------
create table channels (
  id                   uuid primary key default gen_random_uuid(),
  space_id             uuid not null references spaces(id) on delete cascade,
  name                 text not null,
  channel_type         text not null
                       check (channel_type in ('text','voice','category','announcement','forum')),
  position             integer not null default 0,
  permission_overrides jsonb not null default '[]'::jsonb,
  is_e2ee              boolean not null default false,
  created_at           timestamptz not null default now()
);

alter table channels enable row level security;

-- ---------------------------------------------------------------------------
-- messages
-- NO plaintext content column. NO plaintext filename column. The server can
-- see only: which channel, which device, opaque ciphertext/iv blobs, timing.
-- ---------------------------------------------------------------------------
create table messages (
  id                 uuid primary key default gen_random_uuid(),
  channel_id         uuid not null references channels(id) on delete cascade,
  -- Deliberately text + no FK: the device id is opaque to the server and may
  -- outlive the devices row (devices are user-deletable).
  sender_device_id   text not null,
  ciphertext         text not null check (ciphertext <> ''),
  iv                 text not null check (iv <> ''),
  schema_version     integer not null default 1,
  client_created_at  timestamptz not null,
  server_received_at timestamptz not null default now(),
  edited_at          timestamptz,
  deleted_at         timestamptz,   -- tombstone; ciphertext may be NULLed by client
  disappears_at      timestamptz    -- client-side burn-after-read hint
);

alter table messages enable row level security;

-- ---------------------------------------------------------------------------
-- files
-- r2_key points at a blob in R2. The blob itself is AES-256-GCM encrypted
-- client-side; content_key_ciphertext holds the wrapped per-file key.
-- ---------------------------------------------------------------------------
create table files (
  id                     uuid primary key default gen_random_uuid(),
  uploader_device_id     uuid references devices(id) on delete set null,
  r2_key                 text not null unique,
  size_bytes             bigint not null check (size_bytes >= 0),
  content_key_ciphertext text not null,
  created_at             timestamptz not null default now(),
  expires_at             timestamptz
);

alter table files enable row level security;

-- ---------------------------------------------------------------------------
-- roles / role_members
-- ---------------------------------------------------------------------------
create table roles (
  id          uuid primary key default gen_random_uuid(),
  space_id    uuid not null references spaces(id) on delete cascade,
  name        text not null,
  color       text not null default '#99aab5',
  permissions jsonb not null default '{}'::jsonb,
  position    integer not null default 0,
  is_default  boolean not null default false,
  unique (space_id, name)
);

alter table roles enable row level security;

create table role_members (
  role_id  uuid not null references roles(id) on delete cascade,
  user_id  uuid not null references users(id) on delete cascade,
  space_id uuid not null references spaces(id) on delete cascade,
  primary key (role_id, user_id)
);

alter table role_members enable row level security;

-- ---------------------------------------------------------------------------
-- invites
-- ---------------------------------------------------------------------------
create table invites (
  id         uuid primary key default gen_random_uuid(),
  space_id   uuid not null references spaces(id) on delete cascade,
  code       text not null unique default gen_invite_code(),
  creator_id uuid references users(id) on delete set null,
  role_id    uuid references roles(id) on delete set null,
  max_uses   integer,
  used_count integer not null default 0,
  expires_at timestamptz,
  created_at timestamptz not null default now(),
  check (max_uses is null or max_uses > 0),
  check (used_count >= 0)
);

alter table invites enable row level security;

-- ---------------------------------------------------------------------------
-- memberships
-- Direct self-insert is intentionally allowed per spec (used by migration and
-- self-host tooling). The invite-gated path is accept_invite(); clients must
-- use it. See docs/THREAT_MODEL.md (residual risk R-1).
-- ---------------------------------------------------------------------------
create table memberships (
  user_id   uuid not null references users(id) on delete cascade,
  space_id  uuid not null references spaces(id) on delete cascade,
  joined_at timestamptz not null default now(),
  primary key (user_id, space_id)
);

alter table memberships enable row level security;

-- ---------------------------------------------------------------------------
-- presence  (coarse, hourly-bucketed)
-- ---------------------------------------------------------------------------
create table presence (
  user_id          uuid primary key references users(id) on delete cascade,
  status           text not null default 'offline',
  last_seen_bucket integer not null default ((extract(epoch from now())::bigint / 3600)::int)
);

alter table presence enable row level security;

-- ---------------------------------------------------------------------------
-- audit_events
-- NO content fields by design. Insert: service role only. Read: service role
-- only for now (space-admin read is deferred to a later migration).
-- ---------------------------------------------------------------------------
create table audit_events (
  id         uuid primary key default gen_random_uuid(),
  space_id   uuid references spaces(id) on delete cascade,
  actor_id   uuid references users(id) on delete set null,
  event_type text not null check (event_type <> ''),
  target_id  text,
  created_at timestamptz not null default now()
);

alter table audit_events enable row level security;

-- ============================================================================
-- Indexes
-- ============================================================================

-- Required by spec: messages(channel_id, server_received_at desc)
create index idx_messages_channel_received on messages (channel_id, server_received_at desc);
-- Fetch-by-channel for client sync windows.
create index idx_messages_channel_client_created on messages (channel_id, client_created_at);
-- Spec-required (unique constraint already covers it; kept explicit per spec).
create index idx_spaces_invite_code on spaces (invite_code);
-- Spec-required.
create index idx_devices_user on devices (user_id);
-- Spec-required.
create index idx_memberships_space on memberships (space_id);

-- Supporting indexes.
create index idx_channels_space on channels (space_id);
create index idx_roles_space on roles (space_id);
create index idx_role_members_space on role_members (space_id);
create index idx_role_members_user on role_members (user_id);
create index idx_invites_space on invites (space_id);
create index idx_files_uploader on files (uploader_device_id);
create index idx_audit_space_created on audit_events (space_id, created_at desc);
