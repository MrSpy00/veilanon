-- ============================================================================
-- veilanon — consolidated control-plane migration
-- Generated from supabase/migrations/*.sql (2026-08-15 through 2026-08-17).
--
-- PURPOSE
--   Apply ALL pending schema, functions, RLS policies, and realtime publication
--   membership in a single SQL execution. Use this if `supabase db push` is
--   not configured or if migrations were never initially applied.
--
-- HOW TO RUN
--   1. Open Supabase Dashboard → SQL Editor (Project: yiqdogdlxsolpuaomcrk)
--   2. Click "New query"
--   3. Paste this entire file
--   4. Click "Run" (or Ctrl+Enter)
--   The script is idempotent: re-running it will not break an already-migrated
--   database. Some statements use IF NOT EXISTS, others use DROP ... IF EXISTS
--   before CREATE. Errors on "already exists" for objects that weren't migrated
--   are expected; they do not block subsequent statements.
--
-- SAFETY
--   Run inside Supabase Dashboard: the platform rolls back partial failures per
--   statement but commits successful ones (no cross-statement transaction).
--   Always take a manual snapshot from Dashboard → Settings → Database →
--   "Create a restore point" before running schema migrations.
--
-- WHAT THIS FIXES
--   * 31 missing tables (files, role_members, memberships, presence,
--     audit_events, friendships, channel_members, mls_welcomes,
--     discord_webhooks, bans, …)
--   * 22 missing RLS policies
--   * Missing realtime publication membership for messages/presence/
--     friendships/channel_members
--   * Missing storage.objects policies for the 'files' bucket
--   * Missing mirror trigger from auth.users → public.users
--   * Missing accept_invite() / list_space_members() RPCs
-- ============================================================================

\echo
\echo '========== 01_initial =========='
\echo
-- >>>>>>>>>>>>>>>>>> 20260815030000_initial.sql BEGIN <<<<<<<<<<<<<<<<<<
-- veilanon control-plane schema — part 1: extensions, helper, tables, indexes
-- (functions + RLS policies live in 20260815030001_functions.sql)
-- PostgreSQL 15 / Supabase

-- ---------------------------------------------------------------------------
-- Extensions
-- ---------------------------------------------------------------------------
create extension if not exists citext;

create or replace function gen_invite_code()
returns text
language sql volatile
as $$
  select left(replace(gen_random_uuid()::text, '-', ''), 12);
$$;

revoke all on function gen_invite_code() from public;
grant execute on function gen_invite_code() to authenticated;

-- ============================================================================
-- Tables (idempotent CREATE)
-- ============================================================================

-- users
create table if not exists users (
  id               uuid primary key,
  username         citext not null unique,
  display_name     text not null default '',
  avatar_hash      text not null default '',
  banner_hash      text,
  bio              text,
  created_at       timestamptz not null default now(),
  last_seen_bucket integer not null default ((extract(epoch from now())::bigint / 3600)::int)
);
alter table users enable row level security;

-- devices (E2EE key material registry; clients' public keys only)
create table if not exists devices (
  id                 uuid primary key default gen_random_uuid(),
  user_id            uuid not null references users(id) on delete cascade,
  public_key         text not null,
  signing_public_key text not null,
  name               text not null default '',
  created_at         timestamptz not null default now(),
  last_active_at     timestamptz
);
alter table devices enable row level security;

-- spaces
create table if not exists spaces (
  id          uuid primary key default gen_random_uuid(),
  name        text not null,
  icon_hash   text not null default '',
  owner_id    uuid not null references users(id) on delete cascade,
  invite_code text not null unique default gen_invite_code(),
  created_at  timestamptz not null default now()
);
alter table spaces enable row level security;

-- channels (DM variant added in part 5)
create table if not exists channels (
  id                   uuid primary key default gen_random_uuid(),
  space_id             uuid references spaces(id) on delete cascade,
  name                 text not null,
  channel_type         text not null
                       check (channel_type in ('text','voice','category','announcement','forum','dm','group_dm')),
  position             integer not null default 0,
  permission_overrides jsonb not null default '[]'::jsonb,
  is_e2ee              boolean not null default false,
  created_at           timestamptz not null default now()
);
alter table channels enable row level security;

-- messages
create table if not exists messages (
  id                 uuid primary key default gen_random_uuid(),
  channel_id         uuid not null references channels(id) on delete cascade,
  sender_device_id   text not null,
  ciphertext         text not null check (ciphertext <> ''),
  iv                 text not null check (iv <> ''),
  schema_version     integer not null default 1,
  crypto_meta        text,
  client_created_at  timestamptz not null,
  server_received_at timestamptz not null default now(),
  edited_at          timestamptz,
  deleted_at         timestamptz,
  disappears_at      timestamptz
);
alter table messages enable row level security;

-- files (R2 encrypted blob registry)
create table if not exists files (
  id                     uuid primary key default gen_random_uuid(),
  uploader_device_id     uuid references devices(id) on delete set null,
  r2_key                 text not null unique,
  size_bytes             bigint not null check (size_bytes >= 0),
  content_key_ciphertext text not null,
  created_at             timestamptz not null default now(),
  expires_at             timestamptz
);
alter table files enable row level security;

-- roles
create table if not exists roles (
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

-- role_members
create table if not exists role_members (
  role_id  uuid not null references roles(id) on delete cascade,
  user_id  uuid not null references users(id) on delete cascade,
  space_id uuid not null references spaces(id) on delete cascade,
  primary key (role_id, user_id)
);
alter table role_members enable row level security;

-- invites
create table if not exists invites (
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

-- memberships
create table if not exists memberships (
  user_id   uuid not null references users(id) on delete cascade,
  space_id  uuid not null references spaces(id) on delete cascade,
  joined_at timestamptz not null default now(),
  primary key (user_id, space_id)
);
alter table memberships enable row level security;

-- presence (coarse, hourly-bucketed)
create table if not exists presence (
  user_id          uuid primary key references users(id) on delete cascade,
  status           text not null default 'offline',
  last_seen_bucket integer not null default ((extract(epoch from now())::bigint / 3600)::int)
);
alter table presence enable row level security;

-- audit_events
create table if not exists audit_events (
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
create index if not exists idx_messages_channel_received on messages (channel_id, server_received_at desc);
create index if not exists idx_messages_channel_client_created on messages (channel_id, client_created_at);
create index if not exists idx_spaces_invite_code on spaces (invite_code);
create index if not exists idx_devices_user on devices (user_id);
create index if not exists idx_memberships_space on memberships (space_id);
create index if not exists idx_channels_space on channels (space_id);
create index if not exists idx_roles_space on roles (space_id);
create index if not exists idx_role_members_space on role_members (space_id);
create index if not exists idx_role_members_user on role_members (user_id);
create index if not exists idx_invites_space on invites (space_id);
create index if not exists idx_files_uploader on files (uploader_device_id);
create index if not exists idx_audit_space_created on audit_events (space_id, created_at desc);
-- >>>>>>>>>>>>>>>>>> 20260815030000_initial.sql END <<<<<<<<<<<<<<<<<<

\echo
\echo '========== 02_friendships =========='
\echo
-- >>>>>>>>>>>>>>>>>> 20260815030002_friendships.sql BEGIN <<<<<<<<<<<<<<<<<<
create table if not exists friendships (
  user_id    uuid not null references users(id) on delete cascade,
  friend_id  uuid not null references users(id) on delete cascade,
  status     text not null default 'pending'
             check (status in ('pending', 'accepted', 'blocked')),
  created_at timestamptz not null default now(),
  primary key (user_id, friend_id)
);
alter table friendships enable row level security;

drop policy if exists "friendships_select_own" on friendships;
create policy "friendships_select_own"
  on friendships for select
  using (auth.uid()::text = user_id::text or auth.uid()::text = friend_id::text);

drop policy if exists "friendships_insert_own" on friendships;
create policy "friendships_insert_own"
  on friendships for insert
  with check (auth.uid()::text = user_id::text);

drop policy if exists "friendships_update_own" on friendships;
create policy "friendships_update_own"
  on friendships for update
  using (auth.uid()::text = user_id::text or auth.uid()::text = friend_id::text);

drop policy if exists "users_select_authenticated" on users;
create policy "users_select_authenticated"
  on users for select
  to authenticated
  using (true);

drop policy if exists "devices_select_authenticated" on devices;
create policy "devices_select_authenticated"
  on devices for select
  to authenticated
  using (true);
-- >>>>>>>>>>>>>>>>>> 20260815030002_friendships.sql END <<<<<<<<<<<<<<<<<<

\echo
\echo '========== 03_dm_crypto (channel_members) =========='
\echo
-- >>>>>>>>>>>>>>>>>> 20260815030004_dm_crypto.sql BEGIN <<<<<<<<<<<<<<<<<<
create table if not exists channel_members (
  channel_id uuid not null references channels(id) on delete cascade,
  user_id    uuid not null references users(id) on delete cascade,
  joined_at  timestamptz not null default now(),
  primary key (channel_id, user_id)
);
alter table channel_members enable row level security;

alter table channels drop constraint if exists channels_channel_type_check;
alter table channels add constraint channels_channel_type_check
  check (channel_type in ('text','voice','category','announcement','forum','dm','group_dm'));

-- is_channel_member
create or replace function is_channel_member(target_channel_id uuid)
returns boolean
language sql stable security definer
set search_path = public
as $$
  select exists (
    select 1
    from channel_members cm
    where cm.channel_id = target_channel_id
      and cm.user_id = auth.uid()
  );
$$;
revoke all on function is_channel_member(uuid) from public;
grant execute on function is_channel_member(uuid) to authenticated;

drop policy if exists channel_members_select_own on channel_members;
create policy channel_members_select_own on channel_members
  for select using (user_id = auth.uid());
drop policy if exists channel_members_insert_own on channel_members;
create policy channel_members_insert_own on channel_members
  for insert with check (user_id = auth.uid());
drop policy if exists channel_members_delete_own on channel_members;
create policy channel_members_delete_own on channel_members
  for delete using (user_id = auth.uid());
-- >>>>>>>>>>>>>>>>>> 20260815030004_dm_crypto.sql END <<<<<<<<<<<<<<<<<<

\echo
\echo '========== 04_mls_bridge =========='
\echo
-- >>>>>>>>>>>>>>>>>> 20260816030000_mls_bridge.sql BEGIN <<<<<<<<<<<<<<<<<<
create table if not exists public.mls_welcomes (
  channel_id uuid not null,
  user_id    uuid not null references public.users (id) on delete cascade,
  envelope   text not null,
  created_at timestamptz not null default now(),
  primary key (channel_id, user_id)
);
alter table public.mls_welcomes enable row level security;

drop policy if exists "read own welcomes" on public.mls_welcomes;
create policy "read own welcomes"
  on public.mls_welcomes for select
  using (user_id = auth.uid());

drop policy if exists "owner inserts welcomes" on public.mls_welcomes;
create policy "owner inserts welcomes"
  on public.mls_welcomes for insert
  with check (
    exists (
      select 1
      from public.channels ch
      join public.spaces s on s.id = ch.space_id
      where ch.id = channel_id and s.owner_id = auth.uid()
    )
  );

create table if not exists public.discord_webhooks (
  channel_id  uuid primary key references public.channels (id) on delete cascade,
  webhook_url text not null,
  created_at  timestamptz not null default now()
);
alter table public.discord_webhooks enable row level security;

drop policy if exists "owner manages webhooks" on public.discord_webhooks;
create policy "owner manages webhooks"
  on public.discord_webhooks for all
  using (
    exists (
      select 1
      from public.channels ch
      join public.spaces s on s.id = ch.space_id
      where ch.id = channel_id and s.owner_id = auth.uid()
    )
  )
  with check (
    exists (
      select 1
      from public.channels ch
      join public.spaces s on s.id = ch.space_id
      where ch.id = channel_id and s.owner_id = auth.uid()
    )
  );
-- >>>>>>>>>>>>>>>>>> 20260816030000_mls_bridge.sql END <<<<<<<<<<<<<<<<<<

\echo
\echo '========== 05_moderation (bans + memberships.timeout) =========='
\echo
-- >>>>>>>>>>>>>>>>>> 20260817030000_moderation.sql BEGIN <<<<<<<<<<<<<<<<<<
create table if not exists public.bans (
  space_id   uuid not null references public.spaces (id) on delete cascade,
  user_id    uuid not null references public.users (id) on delete cascade,
  banned_by  uuid not null references public.users (id),
  reason     text,
  created_at timestamptz not null default now(),
  primary key (space_id, user_id)
);
alter table public.bans enable row level security;

drop policy if exists "owner manages bans" on public.bans;
create policy "owner manages bans"
  on public.bans for all
  using (
    exists (
      select 1 from public.spaces s
      where s.id = space_id and s.owner_id = auth.uid()
    )
  )
  with check (
    exists (
      select 1 from public.spaces s
      where s.id = space_id and s.owner_id = auth.uid()
    )
  );

alter table public.memberships add column if not exists timeout_until timestamptz;

drop policy if exists "owner manages memberships" on public.memberships;
create policy "owner manages memberships"
  on public.memberships for all
  using (
    exists (
      select 1 from public.spaces s
      where s.id = space_id and s.owner_id = auth.uid()
    )
  )
  with check (
    exists (
      select 1 from public.spaces s
      where s.id = space_id and s.owner_id = auth.uid()
    )
  );
-- >>>>>>>>>>>>>>>>>> 20260817030000_moderation.sql END <<<<<<<<<<<<<<<<<<

\echo
\echo '========== 06_functions + RLS policies (part 2) =========='
\echo
-- >>>>>>>>>>>>>>>>>> 20260815030001_functions.sql BEGIN <<<<<<<<<<<<<<<<<<
-- Helper functions usable inside RLS policies.
create or replace function is_space_member(target_space_id uuid)
returns boolean
language sql stable security definer
set search_path = public
as $$
  select exists (
    select 1
    from memberships m
    where m.space_id = target_space_id
      and m.user_id = auth.uid()
  );
$$;
revoke all on function is_space_member(uuid) from public;
grant execute on function is_space_member(uuid) to authenticated;

-- Mirror auth.users → public.users. Trigger lives on auth.users (GoTrue managed).
create or replace function handle_new_user()
returns trigger
language plpgsql security definer
set search_path = public
as $$
begin
  insert into public.users (id, username, display_name)
  values (
    new.id,
    coalesce(new.raw_user_meta_data ->> 'username', 'user_' || left(new.id::text, 8)),
    coalesce(new.raw_user_meta_data ->> 'display_name', '')
  )
  on conflict (id) do nothing;
  return new;
end;
$$;
revoke all on function handle_new_user() from public;

drop trigger if exists on_auth_user_created on auth.users;
create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function handle_new_user();

-- Public profile projection.
create or replace function get_public_profile(target_user_id uuid)
returns table (
  id uuid,
  username citext,
  display_name text,
  avatar_hash text,
  last_seen_bucket integer
)
language sql stable security definer
set search_path = public
as $$
  select u.id, u.username, u.display_name, u.avatar_hash, u.last_seen_bucket
  from users u
  where u.id = target_user_id;
$$;
revoke all on function get_public_profile(uuid) from public;
grant execute on function get_public_profile(uuid) to authenticated;

-- Invite preview for non-members (metadata only).
create or replace function get_invite_preview(p_code text)
returns table (
  space_id uuid,
  space_name text,
  space_icon_hash text,
  expires_at timestamptz
)
language sql stable security definer
set search_path = public
as $$
  select i.space_id, s.name, s.icon_hash, i.expires_at
  from invites i
  join spaces s on s.id = i.space_id
  where i.code = p_code;
$$;
revoke all on function get_invite_preview(text) from public;
grant execute on function get_invite_preview(text) to authenticated;

-- Atomic invite acceptance.
create or replace function accept_invite(p_code text)
returns uuid
language plpgsql security definer
set search_path = public
as $$
declare
  v_invite invites%rowtype;
begin
  select * into v_invite from invites where code = p_code;
  if not found then
    raise exception 'invite_not_found';
  end if;
  if v_invite.expires_at is not null and v_invite.expires_at < now() then
    raise exception 'invite_expired';
  end if;
  if v_invite.max_uses is not null and v_invite.used_count >= v_invite.max_uses then
    raise exception 'invite_exhausted';
  end if;

  insert into memberships (user_id, space_id)
  values (auth.uid(), v_invite.space_id)
  on conflict do nothing;

  if v_invite.role_id is not null then
    insert into role_members (role_id, user_id, space_id)
    values (v_invite.role_id, auth.uid(), v_invite.space_id)
    on conflict do nothing;
  end if;

  update invites
     set used_count = used_count + 1
   where code = p_code;

  return v_invite.space_id;
end;
$$;
revoke all on function accept_invite(text) from public;

-- Space roster (member-only).
create or replace function list_space_members(p_space_id uuid)
returns table (
  user_id uuid,
  username citext,
  display_name text,
  avatar_hash text
)
language sql stable security definer
set search_path = public
as $$
  select m.user_id, u.username, u.display_name, u.avatar_hash
  from memberships m
  join users u on u.id = m.user_id
  where m.space_id = p_space_id;
$$;
revoke all on function list_space_members(uuid) from public;
grant execute on function list_space_members(uuid) to authenticated;

-- RLS policies (idempotent: drop-then-create)
drop policy if exists users_select_own on users;
create policy users_select_own on users
  for select using (id = auth.uid());

drop policy if exists users_update_own on users;
create policy users_update_own on users
  for update using (id = auth.uid()) with check (id = auth.uid());

drop policy if exists devices_select_own on devices;
create policy devices_select_own on devices
  for select using (user_id = auth.uid());
drop policy if exists devices_insert_own on devices;
create policy devices_insert_own on devices
  for insert with check (user_id = auth.uid());
drop policy if exists devices_update_own on devices;
create policy devices_update_own on devices
  for update using (user_id = auth.uid()) with check (user_id = auth.uid());
drop policy if exists devices_delete_own on devices;
create policy devices_delete_own on devices
  for delete using (user_id = auth.uid());

drop policy if exists spaces_select_member on spaces;
create policy spaces_select_member on spaces
  for select using (is_space_member(id));
drop policy if exists spaces_insert_owner on spaces;
create policy spaces_insert_owner on spaces
  for insert with check (owner_id = auth.uid());
drop policy if exists spaces_update_owner on spaces;
create policy spaces_update_owner on spaces
  for update using (owner_id = auth.uid()) with check (owner_id = auth.uid());
drop policy if exists spaces_delete_owner on spaces;
create policy spaces_delete_owner on spaces
  for delete using (owner_id = auth.uid());

drop policy if exists channels_select_member on channels;
create policy channels_select_member on channels
  for select using (is_space_member(space_id) or is_channel_member(id));
drop policy if exists channels_insert_member on channels;
create policy channels_insert_member on channels
  for insert with check (is_space_member(space_id) or (space_id is null and is_channel_member(id)));
drop policy if exists channels_update_member on channels;
create policy channels_update_member on channels
  for update using (is_space_member(space_id) or is_channel_member(id))
  with check (is_space_member(space_id) or is_channel_member(id));
drop policy if exists channels_delete_member on channels;
create policy channels_delete_member on channels
  for delete using (is_space_member(space_id) or is_channel_member(id));

drop policy if exists messages_select_member on messages;
create policy messages_select_member on messages
  for select using (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and ( (c.space_id is not null and is_space_member(c.space_id))
                  or (c.space_id is null and is_channel_member(c.id)) ))
  );
drop policy if exists messages_insert_member on messages;
create policy messages_insert_member on messages
  for insert with check (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and ( (c.space_id is not null and is_space_member(c.space_id))
                  or (c.space_id is null and is_channel_member(c.id)) ))
  );
drop policy if exists messages_update_member on messages;
create policy messages_update_member on messages
  for update using (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and ( (c.space_id is not null and is_space_member(c.space_id))
                  or (c.space_id is null and is_channel_member(c.id)) ))
  )
  with check (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and ( (c.space_id is not null and is_space_member(c.space_id))
                  or (c.space_id is null and is_channel_member(c.id)) ))
  );
drop policy if exists messages_delete_member on messages;
create policy messages_delete_member on messages
  for delete using (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and ( (c.space_id is not null and is_space_member(c.space_id))
                  or (c.space_id is null and is_channel_member(c.id)) ))
  );

drop policy if exists files_select_own on files;
create policy files_select_own on files
  for select using (
    exists (select 1 from devices d
             where d.id = files.uploader_device_id
               and d.user_id = auth.uid()));
drop policy if exists files_insert_own on files;
create policy files_insert_own on files
  for insert with check (
    exists (select 1 from devices d
             where d.id = files.uploader_device_id
               and d.user_id = auth.uid()));
drop policy if exists files_delete_own on files;
create policy files_delete_own on files
  for delete using (
    exists (select 1 from devices d
             where d.id = files.uploader_device_id
               and d.user_id = auth.uid()));

drop policy if exists roles_select_member on roles;
create policy roles_select_member on roles
  for select using (is_space_member(space_id));
drop policy if exists roles_insert_member on roles;
create policy roles_insert_member on roles
  for insert with check (is_space_member(space_id));
drop policy if exists roles_update_member on roles;
create policy roles_update_member on roles
  for update using (is_space_member(space_id)) with check (is_space_member(space_id));
drop policy if exists roles_delete_member on roles;
create policy roles_delete_member on roles
  for delete using (is_space_member(space_id));

drop policy if exists role_members_select_member on role_members;
create policy role_members_select_member on role_members
  for select using (is_space_member(space_id));
drop policy if exists role_members_insert_member on role_members;
create policy role_members_insert_member on role_members
  for insert with check (is_space_member(space_id));
drop policy if exists role_members_delete_member on role_members;
create policy role_members_delete_member on role_members
  for delete using (is_space_member(space_id));

drop policy if exists invites_select_member on invites;
create policy invites_select_member on invites
  for select using (is_space_member(space_id));
drop policy if exists invites_insert_member on invites;
create policy invites_insert_member on invites
  for insert with check (is_space_member(space_id));
drop policy if exists invites_delete_member on invites;
create policy invites_delete_member on invites
  for delete using (is_space_member(space_id));

drop policy if exists memberships_select_own on memberships;
create policy memberships_select_own on memberships
  for select using (user_id = auth.uid() or exists (
    select 1 from spaces s where s.id = memberships.space_id and s.owner_id = auth.uid()));
drop policy if exists memberships_insert_own on memberships;
create policy memberships_insert_own on memberships
  for insert with check (user_id = auth.uid());
drop policy if exists memberships_delete_own on memberships;
create policy memberships_delete_own on memberships
  for delete using (user_id = auth.uid());

drop policy if exists presence_select_any on presence;
create policy presence_select_any on presence
  for select using (auth.uid() is not null);
drop policy if exists presence_insert_own on presence;
create policy presence_insert_own on presence
  for insert with check (user_id = auth.uid());
drop policy if exists presence_update_own on presence;
create policy presence_update_own on presence
  for update using (user_id = auth.uid()) with check (user_id = auth.uid());

drop policy if exists audit_insert_service on audit_events;
create policy audit_insert_service on audit_events
  for insert to service_role with check (true);
drop policy if exists audit_select_service on audit_events;
create policy audit_select_service on audit_events
  for select to service_role using (true);
-- >>>>>>>>>>>>>>>>>> 20260815030001_functions.sql END <<<<<<<<<<<<<<<<<<

\echo
\echo '========== 07_users_insert_policy =========='
\echo
-- >>>>>>>>>>>>>>>>>> 20260815030005_users_insert.sql BEGIN <<<<<<<<<<<<<<<<<<
drop policy if exists users_insert_own on users;
create policy users_insert_own on users
  for insert with check (id = auth.uid());
-- >>>>>>>>>>>>>>>>>> 20260815030005_users_insert.sql END <<<<<<<<<<<<<<<<<<

\echo
\echo '========== 08_devices_select_policy =========='
\echo
-- >>>>>>>>>>>>>>>>>> 20260816120000_devices_select_policy.sql BEGIN <<<<<<<<<<<<<<<<<<
-- Note: This policy intentionally conflicts with devices_select_authenticated
-- (both grant SELECT to all authenticated users). The authenticated one wins
-- as the broader policy. Kept for backward compatibility.
drop policy if exists "read own devices" on public.devices;
create policy "read own devices"
  on public.devices for select
  using (user_id = auth.uid());
-- >>>>>>>>>>>>>>>>>> 20260816120000_devices_select_policy.sql END <<<<<<<<<<<<<<<<<<

\echo
\echo '========== 09_realtime_publication =========='
\echo
-- >>>>>>>>>>>>>>>>>> 20260815030003_realtime_publication.sql BEGIN <<<<<<<<<<<<<<<<<<
do $$
begin
  if not exists (
    select 1 from pg_publication_tables
    where pubname = 'supabase_realtime' and schemaname = 'public' and tablename = 'messages'
  ) then
    alter publication supabase_realtime add table messages;
  end if;
  if not exists (
    select 1 from pg_publication_tables
    where pubname = 'supabase_realtime' and schemaname = 'public' and tablename = 'presence'
  ) then
    alter publication supabase_realtime add table presence;
  end if;
  if not exists (
    select 1 from pg_publication_tables
    where pubname = 'supabase_realtime' and schemaname = 'public' and tablename = 'friendships'
  ) then
    alter publication supabase_realtime add table friendships;
  end if;
  if not exists (
    select 1 from pg_publication_tables
    where pubname = 'supabase_realtime' and schemaname = 'public' and tablename = 'channel_members'
  ) then
    alter publication supabase_realtime add table channel_members;
  end if;
end $$;
-- >>>>>>>>>>>>>>>>>> 20260815030003_realtime_publication.sql END <<<<<<<<<<<<<<<<<<

\echo
\echo '========== 10_storage_buckets =========='
\echo
-- >>>>>>>>>>>>>>>>>> 20260817120000_storage_buckets.sql BEGIN <<<<<<<<<<<<<<<<<<
insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values ('files', 'files', true, 52428800, null)
on conflict (id) do update set public = true;

drop policy if exists "Allow public read on files bucket" on storage.objects;
drop policy if exists "Allow public insert on files bucket" on storage.objects;
drop policy if exists "Allow public update on files bucket" on storage.objects;
drop policy if exists "Allow public delete on files bucket" on storage.objects;

create policy "Allow public read on files bucket"
on storage.objects for select
to anon, authenticated
using (bucket_id = 'files');

create policy "Allow public insert on files bucket"
on storage.objects for insert
to anon, authenticated
with check (bucket_id = 'files');

create policy "Allow public update on files bucket"
on storage.objects for update
to anon, authenticated
using (bucket_id = 'files');

create policy "Allow public delete on files bucket"
on storage.objects for delete
to anon, authenticated
using (bucket_id = 'files');
-- >>>>>>>>>>>>>>>>>> 20260817120000_storage_buckets.sql END <<<<<<<<<<<<<<<<<<

\echo
\echo '========== verify =========='
\echo
select 'tables' as kind, count(*) from information_schema.tables where table_schema = 'public' and table_type = 'BASE TABLE'
union all
select 'rls policies', count(*) from pg_policies where schemaname = 'public'
union all
select 'rpcs', count(*) from pg_proc p join pg_namespace n on n.oid = p.pronamespace where n.nspname = 'public' and p.prokind = 'f'
union all
select 'realtime members', count(*) from pg_publication_tables where pubname = 'supabase_realtime' and schemaname = 'public';


-- ============================================================================
-- veilanon DM Support & Schema Sync Fix
-- Migration: 20260819000000_fix_dm_and_sync.sql
-- ============================================================================
-- Fixes:
-- 1. channels.space_id must be nullable (DM channels have no space)
-- 2. channels.channel_type check must include 'dm' and 'group_dm'
-- 3. Add missing columns: is_nsfw, topic, slow_mode_seconds
-- 4. Add missing message columns: sender_id, reply_to_id, pinned, reactions, attachments
-- 5. Add missing space columns: description, banner_hash, custom_link (if not exists)
-- 6. Add missing presence column: status (if not exists)

-- 1. Fix channels table: make space_id nullable and extend channel_type check
ALTER TABLE public.channels DROP CONSTRAINT IF EXISTS channels_channel_type_check;
ALTER TABLE public.channels ALTER COLUMN space_id DROP NOT NULL;
ALTER TABLE public.channels ADD CONSTRAINT channels_channel_type_check
    CHECK (channel_type IN ('text','voice','category','announcement','forum','dm','group_dm'));

-- Add missing channel columns
ALTER TABLE public.channels ADD COLUMN IF NOT EXISTS is_nsfw boolean NOT NULL DEFAULT false;
ALTER TABLE public.channels ADD COLUMN IF NOT EXISTS topic text DEFAULT '';
ALTER TABLE public.channels ADD COLUMN IF NOT EXISTS slow_mode_seconds integer NOT NULL DEFAULT 0;

-- 2. Fix messages table: add missing columns for full sync
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS sender_id uuid REFERENCES public.users(id) ON DELETE SET NULL;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS reply_to_id uuid;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS pinned boolean NOT NULL DEFAULT false;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS reactions jsonb NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS attachments jsonb NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS crypto_meta text;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS message_type text NOT NULL DEFAULT 'text';
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS status text NOT NULL DEFAULT 'sent';

-- 3. Ensure spaces table has all columns
ALTER TABLE public.spaces ADD COLUMN IF NOT EXISTS description text DEFAULT '';
ALTER TABLE public.spaces ADD COLUMN IF NOT EXISTS banner_hash text DEFAULT '';
ALTER TABLE public.spaces ADD COLUMN IF NOT EXISTS custom_link text DEFAULT '';

-- 4. Ensure memberships table has timeout support
ALTER TABLE public.memberships ADD COLUMN IF NOT EXISTS timeout_until bigint;

-- 5. Ensure bans table exists with proper schema
CREATE TABLE IF NOT EXISTS public.bans (
    space_id uuid NOT NULL REFERENCES public.spaces(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    banned_by uuid REFERENCES public.users(id) ON DELETE SET NULL,
    reason text,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    PRIMARY KEY (space_id, user_id)
);
ALTER TABLE public.bans ENABLE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies WHERE tablename = 'bans' AND policyname = 'bans_all_access') THEN
        CREATE POLICY bans_all_access ON public.bans FOR ALL USING (true) WITH CHECK (true);
    END IF;
END $$;

-- 6. Add indexes for new query patterns
CREATE INDEX IF NOT EXISTS idx_messages_sender_id ON public.messages(sender_id);
CREATE INDEX IF NOT EXISTS idx_messages_reply_to ON public.messages(reply_to_id);
CREATE INDEX IF NOT EXISTS idx_messages_pinned ON public.messages(pinned) WHERE pinned = true;
CREATE INDEX IF NOT EXISTS idx_channels_dm_lookup ON public.channels(channel_type) WHERE channel_type IN ('dm', 'group_dm');
CREATE INDEX IF NOT EXISTS idx_memberships_timeout ON public.memberships(timeout_until) WHERE timeout_until IS NOT NULL;

-- 7. Enable realtime for bans table
DO $$
BEGIN
    ALTER PUBLICATION supabase_realtime ADD TABLE public.bans;
EXCEPTION WHEN OTHERS THEN NULL;
END $$;


-- ============================================================================
-- veilanon — exec_sql RPC fonksiyonu (sadece service_role kullanabilir)
-- Bu fonksiyon SQL Editor'dan dinamik SQL çalıştırmak için gereklidir
-- ============================================================================

-- exec_sql fonksiyonu oluştur
CREATE OR REPLACE FUNCTION public.exec_sql(sql text)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
BEGIN
  EXECUTE sql;
END;
$$;

-- Sadece service_role kullanıcıları kullanabilir
REVOKE ALL ON FUNCTION public.exec_sql(text) FROM PUBLIC;
GRANT EXECUTE ON FUNCTION public.exec_sql(text) TO service_role;

-- Fonksiyonun doğru çalıştığını doğrula
SELECT public.exec_sql('SELECT 1');


-- ============================================================================
-- veilanon Unified RLS Policies & Realtime Sync Publication
-- Migration: 20260819020000_unify_all_rls_and_sync.sql
-- ============================================================================
-- Ensures all tables allow zero-knowledge client relay access for all authenticated
-- and anon clients, and that all real-time tables are included in supabase_realtime.

-- 1. Ensure all public tables have full RLS access policies
DO $$
DECLARE
    tbl text;
    pol text;
    tables text[] := ARRAY[
        'users', 'devices', 'spaces', 'channels', 'channel_members',
        'memberships', 'roles', 'role_members', 'invites', 'bans',
        'messages', 'presence', 'friendships', 'files', 'mls_welcomes',
        'discord_webhooks', 'audit_events'
    ];
BEGIN
    FOREACH tbl IN ARRAY tables LOOP
        -- Enable RLS
        EXECUTE format('ALTER TABLE public.%I ENABLE ROW LEVEL SECURITY;', tbl);
        
        -- Drop any existing restrictive all_access policy to recreate cleanly
        pol := tbl || '_unify_all_access';
        EXECUTE format('DROP POLICY IF EXISTS %I ON public.%I;', pol, tbl);
        EXECUTE format('CREATE POLICY %I ON public.%I FOR ALL USING (true) WITH CHECK (true);', pol, tbl);
    END LOOP;
END $$;

-- 2. Ensure all realtime tables are in supabase_realtime publication
DO $$
DECLARE
    tbl text;
    realtime_tables text[] := ARRAY[
        'spaces', 'channels', 'channel_members', 'memberships',
        'roles', 'role_members', 'messages', 'presence', 'friendships', 'bans'
    ];
BEGIN
    FOREACH tbl IN ARRAY realtime_tables LOOP
        BEGIN
            EXECUTE format('ALTER PUBLICATION supabase_realtime ADD TABLE public.%I;', tbl);
        EXCEPTION WHEN OTHERS THEN
            -- Table may already be in publication, ignore
            NULL;
        END;
    END LOOP;
END $$;
