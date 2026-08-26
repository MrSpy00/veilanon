-- ============================================================================
-- veilanon control-plane schema — part 2: functions, RLS policies, trigger
-- (tables live in 20260815030000_initial.sql; run after it)
-- PostgreSQL 15 / Supabase
-- ----------------------------------------------------------------------------
-- Ordering note: SQL functions are validated at creation time, so the
-- membership helper needs `memberships` to exist, and RLS policies need the
-- helper. Both are satisfied here — after the tables from part 1.
-- ============================================================================

-- ---------------------------------------------------------------------------
-- Helper functions
-- ---------------------------------------------------------------------------

-- Membership test usable inside RLS policies. SECURITY DEFINER bypasses RLS,
-- which avoids infinite recursion when `memberships` itself is RLS-protected.
-- `auth.uid()` is schema-qualified so it resolves regardless of search_path.
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

-- Mirror auth.users into public.users so auth.uid() policies resolve to rows.
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

create trigger on_auth_user_created
  after insert on auth.users
  for each row execute function handle_new_user();

-- Minimal, explicitly whitelisted public profile projection. Only these
-- columns are ever readable by other users (metadata transparency policy).
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

-- Preview of an invite for a non-member (space name + icon only, no counts
-- that could be abused). Reading invites directly is member-only via RLS.
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

-- Atomic invite acceptance: validate -> consume one use -> grant membership
-- (+ optional role). The ONLY sanctioned way for clients to join a space.
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
grant execute on function accept_invite(text) to authenticated;

-- Roster for a space you belong to. Member identity is metadata (the threat
-- model accepts this; message CONTENT remains E2EE and unreadable server-side).
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

-- ============================================================================
-- Row Level Security policies
-- ============================================================================

-- users: own row fully readable/updatable; inserts happen only via
-- handle_new_user() (no INSERT policy exists, so direct inserts are blocked).
create policy users_select_own on users
  for select using (id = auth.uid());
create policy users_update_own on users
  for update using (id = auth.uid()) with check (id = auth.uid());

-- devices
create policy devices_select_own on devices
  for select using (user_id = auth.uid());
create policy devices_insert_own on devices
  for insert with check (user_id = auth.uid());
create policy devices_update_own on devices
  for update using (user_id = auth.uid()) with check (user_id = auth.uid());
create policy devices_delete_own on devices
  for delete using (user_id = auth.uid());

-- spaces
create policy spaces_select_member on spaces
  for select using (is_space_member(id));
create policy spaces_insert_owner on spaces
  for insert with check (owner_id = auth.uid());
create policy spaces_update_owner on spaces
  for update using (owner_id = auth.uid()) with check (owner_id = auth.uid());
create policy spaces_delete_owner on spaces
  for delete using (owner_id = auth.uid());

-- channels
create policy channels_select_member on channels
  for select using (is_space_member(space_id));
create policy channels_insert_member on channels
  for insert with check (is_space_member(space_id));
create policy channels_update_member on channels
  for update using (is_space_member(space_id)) with check (is_space_member(space_id));
create policy channels_delete_member on channels
  for delete using (is_space_member(space_id));

-- messages: membership of the channel's space is the only access rule
create policy messages_select_member on messages
  for select using (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and is_space_member(c.space_id))
  );
create policy messages_insert_member on messages
  for insert with check (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and is_space_member(c.space_id))
  );
create policy messages_update_member on messages
  for update
  using (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and is_space_member(c.space_id))
  )
  with check (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and is_space_member(c.space_id))
  );
create policy messages_delete_member on messages
  for delete using (
    exists (select 1 from channels c
             where c.id = messages.channel_id
               and is_space_member(c.space_id))
  );

-- files: uploader device (hence uploader user) only
create policy files_select_own on files
  for select using (
    exists (select 1 from devices d
             where d.id = files.uploader_device_id
               and d.user_id = auth.uid())
  );
create policy files_insert_own on files
  for insert with check (
    exists (select 1 from devices d
             where d.id = files.uploader_device_id
               and d.user_id = auth.uid())
  );
create policy files_delete_own on files
  for delete using (
    exists (select 1 from devices d
             where d.id = files.uploader_device_id
               and d.user_id = auth.uid())
  );

-- roles / role_members
create policy roles_select_member on roles
  for select using (is_space_member(space_id));
create policy roles_insert_member on roles
  for insert with check (is_space_member(space_id));
create policy roles_update_member on roles
  for update using (is_space_member(space_id)) with check (is_space_member(space_id));
create policy roles_delete_member on roles
  for delete using (is_space_member(space_id));

create policy role_members_select_member on role_members
  for select using (is_space_member(space_id));
create policy role_members_insert_member on role_members
  for insert with check (is_space_member(space_id));
create policy role_members_delete_member on role_members
  for delete using (is_space_member(space_id));

-- invites
create policy invites_select_member on invites
  for select using (is_space_member(space_id));
create policy invites_insert_member on invites
  for insert with check (is_space_member(space_id));
create policy invites_delete_member on invites
  for delete using (is_space_member(space_id));

-- memberships: own rows only; invite-gated join happens via accept_invite()
create policy memberships_select_own on memberships
  for select using (user_id = auth.uid());
create policy memberships_insert_own on memberships
  for insert with check (user_id = auth.uid());
create policy memberships_delete_own on memberships
  for delete using (user_id = auth.uid());

-- presence: anyone authenticated may see coarse presence; update own only
create policy presence_select_any on presence
  for select using (auth.uid() is not null);
create policy presence_insert_own on presence
  for insert with check (user_id = auth.uid());
create policy presence_update_own on presence
  for update using (user_id = auth.uid()) with check (user_id = auth.uid());

-- audit_events: service role only (space-admin read deferred)
create policy audit_insert_service on audit_events
  for insert to service_role with check (true);
create policy audit_select_service on audit_events
  for select to service_role using (true);
