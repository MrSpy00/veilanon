-- ============================================================================
-- veilanon control-plane schema — part 3: friendships + public key registry
-- Local-first friend graph mirrored to the control plane so requests survive
-- device changes. Privacy invariant: no content, only user-id pairs + status.
--
-- NOTE: table-level RLS policies for users/devices/spaces/channels/messages/
-- presence/memberships already exist in 20260815030001_functions.sql. This
-- migration only ADDS what is missing: the friendships table and the public
-- read paths that E2EE key distribution and friend lookup depend on.
-- ============================================================================

create table friendships (
  user_id    uuid not null references users(id) on delete cascade,
  friend_id  uuid not null references users(id) on delete cascade,
  status     text not null default 'pending'
             check (status in ('pending', 'accepted', 'blocked')),
  created_at timestamptz not null default now(),
  primary key (user_id, friend_id)
);

alter table friendships enable row level security;

-- A user manages only their own side of a friendship.
create policy "friendships_select_own"
  on friendships for select
  using (auth.uid()::text = user_id::text or auth.uid()::text = friend_id::text);

create policy "friendships_insert_own"
  on friendships for insert
  with check (auth.uid()::text = user_id::text);

create policy "friendships_update_own"
  on friendships for update
  using (auth.uid()::text = user_id::text or auth.uid()::text = friend_id::text);

-- Public profile read: usernames are the friend-lookup key, so any
-- authenticated client must be able to resolve them. Presence stays coarse.
create policy "users_select_authenticated"
  on users for select
  to authenticated
  using (true);

-- Public device registry: E2EE key distribution requires reading other
-- users' public keys (their public halves only — never private material).
create policy "devices_select_authenticated"
  on devices for select
  to authenticated
  using (true);

-- NOTE: realtime publication membership for messages/presence/friendships is
-- handled idempotently in 20260815030003_realtime_publication.sql. This
-- migration must NOT add them here again — PostgreSQL 15 would error with
-- "relation is already member of publication" on re-runs.
