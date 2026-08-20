-- ============================================================================
-- veilanon control-plane schema — part 11: comprehensive RLS and sync fixes
-- ----------------------------------------------------------------------------
-- Fixes:
--   1. friendships: Add DELETE policy so cancel/reject/remove works remotely.
--   2. channels: Support space_id IS NULL for DM and Group DM channels.
--   3. channel_members: Allow multi-user insertion for DMs and Group DMs.
--   4. spaces: Allow authenticated discovery (public search & previews).
--   5. invites: Allow authenticated lookup for joining spaces.
-- ============================================================================

-- 1. friendships DELETE policy
drop policy if exists "friendships_delete_own" on friendships;
create policy "friendships_delete_own"
  on friendships for delete
  using (auth.uid()::text = user_id::text or auth.uid()::text = friend_id::text);

-- 2. channels DM & Space awareness
drop policy if exists channels_select_member on channels;
create policy channels_select_member on channels
  for select using (space_id is null or is_space_member(space_id));

drop policy if exists channels_insert_member on channels;
create policy channels_insert_member on channels
  for insert with check (space_id is null or is_space_member(space_id));

drop policy if exists channels_update_member on channels;
create policy channels_update_member on channels
  for update using (space_id is null or is_space_member(space_id))
  with check (space_id is null or is_space_member(space_id));

drop policy if exists channels_delete_member on channels;
create policy channels_delete_member on channels
  for delete using (space_id is null or is_space_member(space_id));

-- 3. channel_members for DM & Group DM
drop policy if exists channel_members_insert_own on channel_members;
drop policy if exists channel_members_insert_member on channel_members;
create policy channel_members_insert_member on channel_members
  for insert with check (auth.uid() is not null);

drop policy if exists channel_members_select_own on channel_members;
drop policy if exists channel_members_select_member on channel_members;
create policy channel_members_select_member on channel_members
  for select using (user_id = auth.uid() or is_channel_member(channel_id));

drop policy if exists channel_members_delete_own on channel_members;
drop policy if exists channel_members_delete_member on channel_members;
create policy channel_members_delete_member on channel_members
  for delete using (user_id = auth.uid() or is_channel_member(channel_id));

-- 4. spaces public discovery
drop policy if exists spaces_select_member on spaces;
drop policy if exists spaces_select_authenticated on spaces;
create policy spaces_select_authenticated on spaces
  for select using (true);

-- 5. invites public lookup
drop policy if exists invites_select_member on invites;
drop policy if exists invites_select_authenticated on invites;
create policy invites_select_authenticated on invites
  for select using (true);
