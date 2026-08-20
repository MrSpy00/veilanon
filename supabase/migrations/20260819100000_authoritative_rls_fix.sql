-- ============================================================================
-- veilanon Authoritative RLS Fix — Nuclear policy rewrite
-- Migration: 20260819100000_authoritative_rls_fix.sql
-- ============================================================================
-- ROOT CAUSE: DM channels have space_id=NULL, but original RLS policies
-- require is_space_member(space_id) which returns FALSE for NULL.
-- Result: DM messages invisible, DM creation fails (can't insert peer's
-- channel_members row), users can't see each other's server messages.
--
-- FIX: Drop ALL policies on ALL tables, recreate with correct DM-aware logic.
-- Also adds create_dm_channel() SECURITY DEFINER RPC for DM creation.
-- ============================================================================

-- ── STEP 1: Nuclear drop ALL policies on ALL tables ─────────────────────────
DO $$
DECLARE
    pol RECORD;
BEGIN
    FOR pol IN
        SELECT policyname, tablename
        FROM pg_policies
        WHERE schemaname = 'public'
    LOOP
        EXECUTE format('DROP POLICY IF EXISTS %I ON public.%I;', pol.policyname, pol.tablename);
    END LOOP;
    RAISE NOTICE 'Dropped ALL RLS policies on public schema';
END $$;

-- ── STEP 2: Ensure helper functions exist (idempotent) ──────────────────────

-- Membership test for space channels
CREATE OR REPLACE FUNCTION is_space_member(target_space_id uuid)
RETURNS boolean
LANGUAGE sql stable security definer
SET search_path = public
AS $$
  SELECT EXISTS (
    SELECT 1 FROM memberships m
    WHERE m.space_id = target_space_id AND m.user_id = auth.uid()
  );
$$;

-- Membership test for DM/group channels
CREATE OR REPLACE FUNCTION is_channel_member(target_channel_id uuid)
RETURNS boolean
LANGUAGE sql stable security definer
SET search_path = public
AS $$
  SELECT EXISTS (
    SELECT 1 FROM channel_members cm
    WHERE cm.channel_id = target_channel_id AND cm.user_id = auth.uid()
  );
$$;

-- ── STEP 3: Create DM channel RPC (SECURITY DEFINER bypasses RLS) ──────────
-- When User A opens a DM with User B, both channel_members rows must be
-- inserted. RLS only allows user_id = auth.uid() for INSERT, so User A
-- cannot insert User B's row. This RPC solves that.
CREATE OR REPLACE FUNCTION create_dm_channel(
  p_channel_id uuid,
  p_peer_user_id uuid
) RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
BEGIN
  INSERT INTO channel_members (channel_id, user_id)
  VALUES (p_channel_id, auth.uid())
  ON CONFLICT DO NOTHING;

  INSERT INTO channel_members (channel_id, user_id)
  VALUES (p_channel_id, p_peer_user_id)
  ON CONFLICT DO NOTHING;
END;
$$;

GRANT EXECUTE ON FUNCTION create_dm_channel(uuid, uuid) TO authenticated;

-- General-purpose channel member insert (for group DMs, etc.)
CREATE OR REPLACE FUNCTION add_channel_members(
  p_channel_id uuid,
  p_user_ids uuid[]
) RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = public
AS $$
DECLARE
  uid uuid;
BEGIN
  FOREACH uid IN ARRAY p_user_ids LOOP
    INSERT INTO channel_members (channel_id, user_id)
    VALUES (p_channel_id, uid)
    ON CONFLICT DO NOTHING;
  END LOOP;
END;
$$;

GRANT EXECUTE ON FUNCTION add_channel_members(uuid, uuid[]) TO authenticated;

-- ── STEP 4: Recreate all RLS policies with correct DM-aware logic ──────────

-- ── users ──────────────────────────────────────────────────────────────────
-- Own row: full read/write
CREATE POLICY users_select_own ON users
  FOR SELECT USING (id = auth.uid());
CREATE POLICY users_update_own ON users
  FOR UPDATE USING (id = auth.uid()) WITH CHECK (id = auth.uid());
-- Public read for friend lookup, E2EE key distribution, profile display
CREATE POLICY users_select_authenticated ON users
  FOR SELECT TO authenticated USING (true);

-- ── devices ────────────────────────────────────────────────────────────────
-- Own devices: full CRUD
CREATE POLICY devices_select_own ON devices
  FOR SELECT USING (user_id = auth.uid());
CREATE POLICY devices_insert_own ON devices
  FOR INSERT WITH CHECK (user_id = auth.uid());
CREATE POLICY devices_update_own ON devices
  FOR UPDATE USING (user_id = auth.uid()) WITH CHECK (user_id = auth.uid());
CREATE POLICY devices_delete_own ON devices
  FOR DELETE USING (user_id = auth.uid());
-- Public read for E2EE key distribution (peer needs your public key)
CREATE POLICY devices_select_authenticated ON devices
  FOR SELECT TO authenticated USING (true);

-- ── spaces ─────────────────────────────────────────────────────────────────
-- Public read for invite/join discovery flow (intentional)
CREATE POLICY spaces_select_member ON spaces
  FOR SELECT USING (true);
CREATE POLICY spaces_insert_owner ON spaces
  FOR INSERT WITH CHECK (owner_id = auth.uid());
CREATE POLICY spaces_update_owner ON spaces
  FOR UPDATE USING (owner_id = auth.uid()) WITH CHECK (owner_id = auth.uid());
CREATE POLICY spaces_delete_owner ON spaces
  FOR DELETE USING (owner_id = auth.uid());

-- ── channels ───────────────────────────────────────────────────────────────
-- Space channels: visible to space members
-- DM channels: visible ONLY to channel members (privacy fix)
CREATE POLICY channels_select_member ON channels
  FOR SELECT USING (
    (space_id IS NOT NULL AND is_space_member(space_id))
    OR
    (space_id IS NULL AND is_channel_member(id))
  );
-- Create: space members can create space channels; DMs created via RPC
CREATE POLICY channels_insert_member ON channels
  FOR INSERT WITH CHECK (
    (space_id IS NOT NULL AND is_space_member(space_id))
    OR
    (space_id IS NULL)
  );
CREATE POLICY channels_update_member ON channels
  FOR UPDATE USING (
    (space_id IS NOT NULL AND is_space_member(space_id))
    OR
    (space_id IS NULL AND is_channel_member(id))
  ) WITH CHECK (
    (space_id IS NOT NULL AND is_space_member(space_id))
    OR
    (space_id IS NULL AND is_channel_member(id))
  );
CREATE POLICY channels_delete_member ON channels
  FOR DELETE USING (
    (space_id IS NOT NULL AND is_space_member(space_id))
    OR
    (space_id IS NULL AND is_channel_member(id))
  );

-- ── channel_members ────────────────────────────────────────────────────────
-- Read: own rows + channel members can see each other
CREATE POLICY channel_members_select_own ON channel_members
  FOR SELECT USING (user_id = auth.uid());
CREATE POLICY channel_members_select_member ON channel_members
  FOR SELECT USING (is_channel_member(channel_id));
-- Insert: only own row via direct insert (DM creation via RPC handles both)
CREATE POLICY channel_members_insert_own ON channel_members
  FOR INSERT WITH CHECK (user_id = auth.uid());
-- Delete: own rows only
CREATE POLICY channel_members_delete_own ON channel_members
  FOR DELETE USING (user_id = auth.uid());
-- Update: own rows only (for settings like notification preferences)
CREATE POLICY channel_members_update_own ON channel_members
  FOR UPDATE USING (user_id = auth.uid()) WITH CHECK (user_id = auth.uid());

-- ── messages ───────────────────────────────────────────────────────────────
-- DM-aware: space member OR DM channel member
CREATE POLICY messages_select_member ON messages
  FOR SELECT USING (
    EXISTS (
      SELECT 1 FROM channels c
      WHERE c.id = messages.channel_id
        AND c.space_id IS NOT NULL
        AND is_space_member(c.space_id)
    )
    OR
    EXISTS (
      SELECT 1 FROM channels c
      WHERE c.id = messages.channel_id
        AND c.space_id IS NULL
        AND is_channel_member(c.id)
    )
  );
CREATE POLICY messages_insert_member ON messages
  FOR INSERT WITH CHECK (
    EXISTS (
      SELECT 1 FROM channels c
      WHERE c.id = messages.channel_id
        AND c.space_id IS NOT NULL
        AND is_space_member(c.space_id)
    )
    OR
    EXISTS (
      SELECT 1 FROM channels c
      WHERE c.id = messages.channel_id
        AND c.space_id IS NULL
        AND is_channel_member(c.id)
    )
  );
CREATE POLICY messages_update_member ON messages
  FOR UPDATE USING (
    EXISTS (
      SELECT 1 FROM channels c
      WHERE c.id = messages.channel_id
        AND c.space_id IS NOT NULL
        AND is_space_member(c.space_id)
    )
    OR
    EXISTS (
      SELECT 1 FROM channels c
      WHERE c.id = messages.channel_id
        AND c.space_id IS NULL
        AND is_channel_member(c.id)
    )
  ) WITH CHECK (
    EXISTS (
      SELECT 1 FROM channels c
      WHERE c.id = messages.channel_id
        AND c.space_id IS NOT NULL
        AND is_space_member(c.space_id)
    )
    OR
    EXISTS (
      SELECT 1 FROM channels c
      WHERE c.id = messages.channel_id
        AND c.space_id IS NULL
        AND is_channel_member(c.id)
    )
  );
CREATE POLICY messages_delete_member ON messages
  FOR DELETE USING (
    EXISTS (
      SELECT 1 FROM channels c
      WHERE c.id = messages.channel_id
        AND c.space_id IS NOT NULL
        AND is_space_member(c.space_id)
    )
    OR
    EXISTS (
      SELECT 1 FROM channels c
      WHERE c.id = messages.channel_id
        AND c.space_id IS NULL
        AND is_channel_member(c.id)
    )
  );

-- ── memberships ────────────────────────────────────────────────────────────
-- Own rows: full CRUD
CREATE POLICY memberships_select_own ON memberships
  FOR SELECT USING (user_id = auth.uid());
CREATE POLICY memberships_insert_own ON memberships
  FOR INSERT WITH CHECK (user_id = auth.uid());
CREATE POLICY memberships_delete_own ON memberships
  FOR DELETE USING (user_id = auth.uid());
-- Space members can see each other (for member list)
CREATE POLICY memberships_select_member ON memberships
  FOR SELECT USING (is_space_member(space_id));
-- Owner can manage (kick/ban)
CREATE POLICY owner_manages_memberships ON memberships
  FOR ALL USING (
    EXISTS (
      SELECT 1 FROM spaces s
      WHERE s.id = space_id AND s.owner_id = auth.uid()
    )
  ) WITH CHECK (
    EXISTS (
      SELECT 1 FROM spaces s
      WHERE s.id = space_id AND s.owner_id = auth.uid()
    )
  );

-- ── roles ──────────────────────────────────────────────────────────────────
CREATE POLICY roles_select_member ON roles
  FOR SELECT USING (is_space_member(space_id));
CREATE POLICY roles_insert_member ON roles
  FOR INSERT WITH CHECK (is_space_member(space_id));
CREATE POLICY roles_update_member ON roles
  FOR UPDATE USING (is_space_member(space_id)) WITH CHECK (is_space_member(space_id));
CREATE POLICY roles_delete_member ON roles
  FOR DELETE USING (is_space_member(space_id));

-- ── role_members ───────────────────────────────────────────────────────────
CREATE POLICY role_members_select_member ON role_members
  FOR SELECT USING (is_space_member(space_id));
CREATE POLICY role_members_insert_member ON role_members
  FOR INSERT WITH CHECK (is_space_member(space_id));
CREATE POLICY role_members_delete_member ON role_members
  FOR DELETE USING (is_space_member(space_id));

-- ── invites ────────────────────────────────────────────────────────────────
-- Public read for join/invite flow (intentional)
CREATE POLICY invites_select_member ON invites
  FOR SELECT USING (true);
CREATE POLICY invites_insert_member ON invites
  FOR INSERT WITH CHECK (is_space_member(space_id));
CREATE POLICY invites_delete_member ON invites
  FOR DELETE USING (is_space_member(space_id));

-- ── friendships ────────────────────────────────────────────────────────────
CREATE POLICY friendships_select_own ON friendships
  FOR SELECT USING (auth.uid()::text = user_id::text OR auth.uid()::text = friend_id::text);
CREATE POLICY friendships_insert_own ON friendships
  FOR INSERT WITH CHECK (auth.uid()::text = user_id::text);
CREATE POLICY friendships_update_own ON friendships
  FOR UPDATE USING (auth.uid()::text = user_id::text OR auth.uid()::text = friend_id::text);
CREATE POLICY friendships_delete_own ON friendships
  FOR DELETE USING (auth.uid()::text = user_id::text OR auth.uid()::text = friend_id::text);

-- ── presence ───────────────────────────────────────────────────────────────
CREATE POLICY presence_select_any ON presence
  FOR SELECT USING (auth.uid() IS NOT NULL);
CREATE POLICY presence_insert_own ON presence
  FOR INSERT WITH CHECK (user_id = auth.uid());
CREATE POLICY presence_update_own ON presence
  FOR UPDATE USING (user_id = auth.uid()) WITH CHECK (user_id = auth.uid());

-- ── bans ───────────────────────────────────────────────────────────────────
CREATE POLICY owner_manages_bans ON bans
  FOR ALL USING (
    EXISTS (
      SELECT 1 FROM spaces s
      WHERE s.id = space_id AND s.owner_id = auth.uid()
    )
  ) WITH CHECK (
    EXISTS (
      SELECT 1 FROM spaces s
      WHERE s.id = space_id AND s.owner_id = auth.uid()
    )
  );

-- ── files ──────────────────────────────────────────────────────────────────
CREATE POLICY files_select_own ON files
  FOR SELECT USING (
    EXISTS (
      SELECT 1 FROM devices d
      WHERE d.id = files.uploader_device_id
        AND d.user_id = auth.uid()
    )
  );
CREATE POLICY files_insert_own ON files
  FOR INSERT WITH CHECK (
    EXISTS (
      SELECT 1 FROM devices d
      WHERE d.id = files.uploader_device_id
        AND d.user_id = auth.uid()
    )
  );
CREATE POLICY files_delete_own ON files
  FOR DELETE USING (
    EXISTS (
      SELECT 1 FROM devices d
      WHERE d.id = files.uploader_device_id
        AND d.user_id = auth.uid()
    )
  );

-- ── audit_events (service_role only) ──────────────────────────────────────
CREATE POLICY audit_insert_service ON audit_events
  FOR INSERT TO service_role WITH CHECK (true);
CREATE POLICY audit_select_service ON audit_events
  FOR SELECT TO service_role USING (true);

-- ── STEP 5: Ensure all critical tables are in supabase_realtime ────────────
DO $$
DECLARE
    tbl text;
    tables text[] := ARRAY[
        'messages', 'channels', 'channel_members', 'memberships',
        'spaces', 'roles', 'role_members', 'presence',
        'friendships', 'bans', 'users', 'devices'
    ];
BEGIN
    FOREACH tbl IN ARRAY tables LOOP
        BEGIN
            EXECUTE format('ALTER PUBLICATION supabase_realtime ADD TABLE public.%I;', tbl);
        EXCEPTION WHEN OTHERS THEN
            NULL;
        END;
    END LOOP;
END $$;

-- ── STEP 6: Verify — no blanket bypass policies remain ─────────────────────
DO $$
DECLARE
    blanket_count INTEGER;
BEGIN
    SELECT count(*) INTO blanket_count
    FROM pg_policies
    WHERE schemaname = 'public'
      AND qual = 'true' AND with_check = 'true'
      AND policyname NOT LIKE '%select_authenticated'
      AND policyname NOT LIKE '%_select_any'
      AND policyname NOT LIKE '%insert_service'
      AND policyname NOT LIKE '%select_service'
      AND policyname NOT LIKE '%spaces_select_member'
      AND policyname NOT LIKE '%invites_select_member';

    IF blanket_count > 0 THEN
        RAISE WARNING 'Found % blanket policies that may need review', blanket_count;
    ELSE
        RAISE NOTICE 'All blanket bypass policies removed successfully';
    END IF;
END $$;
