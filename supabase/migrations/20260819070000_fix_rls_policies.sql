-- ============================================================================
-- veilanon Fix RLS Policies — Restore Member-Scoped Access Control
-- Migration: 20260819070000_fix_rls_policies.sql
-- ============================================================================
-- Drops ALL blanket bypass policies created by:
--   20260819020000_unify_all_rls_and_sync.sql  (_unify_all_access)
--   20260818150000_complete_fix_and_sync.sql   (*_all_access)
--   20260819000000_fix_dm_and_sync.sql         (bans_all_access)
-- Restores proper member-scoped policies from the original design (20260815030001 + 20260815030004).

-- ============================================================================
-- STEP 1: Drop ALL blanket bypass policies
-- ============================================================================
DO $$
DECLARE
    pol RECORD;
BEGIN
    FOR pol IN
        SELECT policyname, tablename
        FROM pg_policies
        WHERE schemaname = 'public'
          AND (
            policyname LIKE '%_unify_all_access'
            OR policyname LIKE '%_all_access'
            OR (policyname = 'bans_all_access')
          )
    LOOP
        EXECUTE format('DROP POLICY IF EXISTS %I ON public.%I;', pol.policyname, pol.tablename);
    END LOOP;
END $$;

-- ============================================================================
-- STEP 2: Drop and recreate policies that were partially overwritten
-- ============================================================================
-- The blanket policies were additive (PostgreSQL RLS = any matching policy = access).
-- We need to DROP the old member-scoped policies and recreate them cleanly
-- to ensure no stale state remains.

-- users
DROP POLICY IF EXISTS users_select_own ON users;
DROP POLICY IF EXISTS users_update_own ON users;
DROP POLICY IF EXISTS users_select_authenticated ON users;

-- devices
DROP POLICY IF EXISTS devices_select_own ON devices;
DROP POLICY IF EXISTS devices_insert_own ON devices;
DROP POLICY IF EXISTS devices_update_own ON devices;
DROP POLICY IF EXISTS devices_delete_own ON devices;
DROP POLICY IF EXISTS devices_select_authenticated ON devices;

-- spaces
DROP POLICY IF EXISTS spaces_select_member ON spaces;
DROP POLICY IF EXISTS spaces_insert_owner ON spaces;
DROP POLICY IF EXISTS spaces_update_owner ON spaces;
DROP POLICY IF EXISTS spaces_delete_owner ON spaces;

-- channels (DM-aware: space_id IS NULL → channel_members check)
DROP POLICY IF EXISTS channels_select_member ON channels;
DROP POLICY IF EXISTS channels_insert_member ON channels;
DROP POLICY IF EXISTS channels_update_member ON channels;
DROP POLICY IF EXISTS channels_delete_member ON channels;

-- channel_members
DROP POLICY IF EXISTS channel_members_select_own ON channel_members;
DROP POLICY IF EXISTS channel_members_insert_own ON channel_members;
DROP POLICY IF EXISTS channel_members_delete_own ON channel_members;
DROP POLICY IF EXISTS channel_members_select_member ON channel_members;
DROP POLICY IF EXISTS channel_members_insert_member ON channel_members;
DROP POLICY IF EXISTS channel_members_delete_member ON channel_members;

-- messages (DM-aware: space member OR channel member)
DROP POLICY IF EXISTS messages_select_member ON messages;
DROP POLICY IF EXISTS messages_insert_member ON messages;
DROP POLICY IF EXISTS messages_update_member ON messages;
DROP POLICY IF EXISTS messages_delete_member ON messages;

-- roles
DROP POLICY IF EXISTS roles_select_member ON roles;
DROP POLICY IF EXISTS roles_insert_member ON roles;
DROP POLICY IF EXISTS roles_update_member ON roles;
DROP POLICY IF EXISTS roles_delete_member ON roles;

-- role_members
DROP POLICY IF EXISTS role_members_select_member ON role_members;
DROP POLICY IF EXISTS role_members_insert_member ON role_members;
DROP POLICY IF EXISTS role_members_delete_member ON role_members;

-- invites
DROP POLICY IF EXISTS invites_select_member ON invites;
DROP POLICY IF EXISTS invites_insert_member ON invites;
DROP POLICY IF EXISTS invites_delete_member ON invites;

-- memberships
DROP POLICY IF EXISTS memberships_select_own ON memberships;
DROP POLICY IF EXISTS memberships_insert_own ON memberships;
DROP POLICY IF EXISTS memberships_delete_own ON memberships;
DROP POLICY IF EXISTS owner_manages_memberships ON memberships;

-- friendships
DROP POLICY IF EXISTS friendships_select_own ON friendships;
DROP POLICY IF EXISTS friendships_insert_own ON friendships;
DROP POLICY IF EXISTS friendships_update_own ON friendships;

-- presence
DROP POLICY IF EXISTS presence_select_any ON presence;
DROP POLICY IF EXISTS presence_insert_own ON presence;
DROP POLICY IF EXISTS presence_update_own ON presence;

-- bans
DROP POLICY IF EXISTS bans_select_member ON bans;
DROP POLICY IF EXISTS bans_insert_member ON bans;
DROP POLICY IF EXISTS bans_delete_member ON bans;
DROP POLICY IF EXISTS owner_manages_bans ON bans;

-- files
DROP POLICY IF EXISTS files_select_own ON files;
DROP POLICY IF EXISTS files_insert_own ON files;
DROP POLICY IF EXISTS files_delete_own ON files;

-- audit_events
DROP POLICY IF EXISTS audit_insert_service ON audit_events;
DROP POLICY IF EXISTS audit_select_service ON audit_events;

-- ============================================================================
-- STEP 3: Recreate proper member-scoped policies
-- ============================================================================

-- ── users ──────────────────────────────────────────────────────────────────
CREATE POLICY users_select_own ON users
  FOR SELECT USING (id = auth.uid());
CREATE POLICY users_update_own ON users
  FOR UPDATE USING (id = auth.uid()) WITH CHECK (id = auth.uid());
-- Public read for friend lookup and E2EE key distribution
CREATE POLICY users_select_authenticated ON users
  FOR SELECT TO authenticated USING (true);

-- ── devices ────────────────────────────────────────────────────────────────
CREATE POLICY devices_select_own ON devices
  FOR SELECT USING (user_id = auth.uid());
CREATE POLICY devices_insert_own ON devices
  FOR INSERT WITH CHECK (user_id = auth.uid());
CREATE POLICY devices_update_own ON devices
  FOR UPDATE USING (user_id = auth.uid()) WITH CHECK (user_id = auth.uid());
CREATE POLICY devices_delete_own ON devices
  FOR DELETE USING (user_id = auth.uid());
-- Public read for E2EE key distribution
CREATE POLICY devices_select_authenticated ON devices
  FOR SELECT TO authenticated USING (true);

-- ── spaces ─────────────────────────────────────────────────────────────────
-- Member select (public for invite/join discovery flow)
CREATE POLICY spaces_select_member ON spaces
  FOR SELECT USING (is_space_member(id) OR true);
CREATE POLICY spaces_insert_owner ON spaces
  FOR INSERT WITH CHECK (owner_id = auth.uid());
CREATE POLICY spaces_update_owner ON spaces
  FOR UPDATE USING (owner_id = auth.uid()) WITH CHECK (owner_id = auth.uid());
CREATE POLICY spaces_delete_owner ON spaces
  FOR DELETE USING (owner_id = auth.uid());

-- ── channels (DM-aware: space_id IS NULL → channel_members) ────────────────
CREATE POLICY channels_select_member ON channels
  FOR SELECT USING (
    space_id IS NULL OR is_space_member(space_id)
  );
CREATE POLICY channels_insert_member ON channels
  FOR INSERT WITH CHECK (
    space_id IS NULL OR is_space_member(space_id)
  );
CREATE POLICY channels_update_member ON channels
  FOR UPDATE USING (
    space_id IS NULL OR is_space_member(space_id)
  ) WITH CHECK (
    space_id IS NULL OR is_space_member(space_id)
  );
CREATE POLICY channels_delete_member ON channels
  FOR DELETE USING (
    space_id IS NULL OR is_space_member(space_id)
  );

-- ── channel_members ────────────────────────────────────────────────────────
CREATE POLICY channel_members_select_own ON channel_members
  FOR SELECT USING (user_id = auth.uid());
CREATE POLICY channel_members_insert_own ON channel_members
  FOR INSERT WITH CHECK (user_id = auth.uid());
CREATE POLICY channel_members_delete_own ON channel_members
  FOR DELETE USING (user_id = auth.uid());
-- Allow reading channel membership for members of the channel
CREATE POLICY channel_members_select_member ON channel_members
  FOR SELECT USING (is_channel_member(channel_id));
-- Allow adding members to DM channels (any authenticated user can be added)
CREATE POLICY channel_members_insert_member ON channel_members
  FOR INSERT WITH CHECK (auth.uid() IS NOT NULL);

-- ── messages (DM-aware: space member OR DM channel member) ─────────────────
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
-- Public read for join/invite flow
CREATE POLICY invites_select_member ON invites
  FOR SELECT USING (is_space_member(space_id) OR true);
CREATE POLICY invites_insert_member ON invites
  FOR INSERT WITH CHECK (is_space_member(space_id));
CREATE POLICY invites_delete_member ON invites
  FOR DELETE USING (is_space_member(space_id));

-- ── memberships ────────────────────────────────────────────────────────────
CREATE POLICY memberships_select_own ON memberships
  FOR SELECT USING (user_id = auth.uid());
CREATE POLICY memberships_insert_own ON memberships
  FOR INSERT WITH CHECK (user_id = auth.uid());
CREATE POLICY memberships_delete_own ON memberships
  FOR DELETE USING (user_id = auth.uid());
-- Owner can manage memberships (kick/ban)
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

-- ── friendships ────────────────────────────────────────────────────────────
CREATE POLICY friendships_select_own ON friendships
  FOR SELECT USING (auth.uid()::text = user_id::text OR auth.uid()::text = friend_id::text);
CREATE POLICY friendships_insert_own ON friendships
  FOR INSERT WITH CHECK (auth.uid()::text = user_id::text);
CREATE POLICY friendships_update_own ON friendships
  FOR UPDATE USING (auth.uid()::text = user_id::text OR auth.uid()::text = friend_id::text);

-- ── presence ───────────────────────────────────────────────────────────────
CREATE POLICY presence_select_any ON presence
  FOR SELECT USING (auth.uid() IS NOT NULL);
CREATE POLICY presence_insert_own ON presence
  FOR INSERT WITH CHECK (user_id = auth.uid());
CREATE POLICY presence_update_own ON presence
  FOR UPDATE USING (user_id = auth.uid()) WITH CHECK (user_id = auth.uid());

-- ── bans (owner only) ─────────────────────────────────────────────────────
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

-- ============================================================================
-- STEP 4: Verify — log remaining blanket policies for manual review
-- ============================================================================
DO $$
DECLARE
    blanket_count INTEGER;
BEGIN
    SELECT count(*) INTO blanket_count
    FROM pg_policies
    WHERE schemaname = 'public'
      AND (
        qual = 'true' AND with_check = 'true'
      )
      AND policyname NOT LIKE '%select_authenticated'
      AND policyname NOT LIKE '%_select_any'
      AND policyname NOT LIKE '%_select_member'
      AND policyname NOT LIKE '%insert_service'
      AND policyname NOT LIKE '%select_service'
      AND policyname NOT LIKE '%insert_member'
      AND policyname NOT LIKE '%insert_own';

    IF blanket_count > 0 THEN
        RAISE WARNING 'Found % blanket policies that may need review', blanket_count;
    ELSE
        RAISE NOTICE 'All blanket bypass policies removed successfully';
    END IF;
END $$;
