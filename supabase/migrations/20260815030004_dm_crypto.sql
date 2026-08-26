-- ============================================================================
-- veilanon control-plane schema — part 4: 1:1 DM support
-- ----------------------------------------------------------------------------
-- Adds:
--   * channel_members        — membership of space-less (DM) channels
--   * is_channel_member()    — RLS helper for DM channels
--   * messages.crypto_meta   — per-message Double-Ratchet header (no keys)
--   * DM-aware message RLS   — space members OR dm channel members
-- ============================================================================

create table channel_members (
  channel_id uuid not null references channels(id) on delete cascade,
  user_id    uuid not null references users(id) on delete cascade,
  joined_at  timestamptz not null default now(),
  primary key (channel_id, user_id)
);

alter table channel_members enable row level security;

-- DM channels have no space: relax the NOT NULL and extend the type check.
alter table channels alter column space_id drop not null;
alter table channels drop constraint channels_channel_type_check;
alter table channels add constraint channels_channel_type_check
  check (channel_type in ('text','voice','category','announcement','forum','dm','group_dm'));

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

create policy channel_members_select_own on channel_members
  for select using (user_id = auth.uid());
create policy channel_members_insert_own on channel_members
  for insert with check (user_id = auth.uid());
create policy channel_members_delete_own on channel_members
  for delete using (user_id = auth.uid());

-- Per-message crypto metadata: JSON Double-Ratchet header for 1:1 DMs.
-- NULL for deterministic-key messages. Never contains plaintext or keys.
alter table messages add column if not exists crypto_meta text;

-- DM-aware message access: replace the space-only policies so space-less
-- channels resolve through channel_members instead of is_space_member(NULL).
drop policy if exists messages_select_member on messages;
create policy messages_select_member on messages
  for select using (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and c.space_id is not null
               and is_space_member(c.space_id))
    or
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and c.space_id is null
               and is_channel_member(c.id))
  );

drop policy if exists messages_insert_member on messages;
create policy messages_insert_member on messages
  for insert with check (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and c.space_id is not null
               and is_space_member(c.space_id))
    or
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and c.space_id is null
               and is_channel_member(c.id))
  );

drop policy if exists messages_update_member on messages;
create policy messages_update_member on messages
  for update
  using (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and c.space_id is not null
               and is_space_member(c.space_id))
    or
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and c.space_id is null
               and is_channel_member(c.id))
  )
  with check (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and c.space_id is not null
               and is_space_member(c.space_id))
    or
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and c.space_id is null
               and is_channel_member(c.id))
  );

drop policy if exists messages_delete_member on messages;
create policy messages_delete_member on messages
  for delete using (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and c.space_id is not null
               and is_space_member(c.space_id))
    or
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and c.space_id is null
               and is_channel_member(c.id))
  );

alter publication supabase_realtime add table channel_members;
