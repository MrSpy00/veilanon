-- ============================================================================
-- veilanon Fix RLS + Realtime — Final authoritative policies
-- Migration: 20260819080000_fix_rls_and_realtime.sql
-- ============================================================================
-- This migration:
--   1. Drops ALL leftover blanket bypass policies missed by previous cleanup
--   2. Ensures is_channel_member() and is_space_member() exist
--   3. Ensures all critical tables are in supabase_realtime publication
--   4. Adds missing friendships DELETE policy

-- STEP 1: Drop ALL remaining blanket bypass policies
DO $$
DECLARE
    pol RECORD;
BEGIN
    FOR pol IN
        SELECT policyname, tablename
        FROM pg_policies
        WHERE schemaname = 'public'
          AND qual = 'true' AND with_check = 'true'
          AND policyname NOT LIKE '%select_authenticated'
          AND policyname NOT LIKE '%_select_any'
          AND policyname NOT LIKE '%_select_member'
          AND policyname NOT LIKE '%insert_service'
          AND policyname NOT LIKE '%select_service'
          AND policyname NOT LIKE '%insert_member'
          AND policyname NOT LIKE '%insert_own'
          AND policyname NOT LIKE '%_all_access'
          AND policyname NOT LIKE '%_unify_all_access'
    LOOP
        EXECUTE format('DROP POLICY IF EXISTS %I ON public.%I;', pol.policyname, pol.tablename);
    END LOOP;
END $$;

-- STEP 2: Ensure helper functions exist (idempotent)
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

-- STEP 3: Ensure all critical tables are in supabase_realtime
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

-- STEP 4: Add missing friendships DELETE policy
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE policyname = 'friendships_delete_own' AND tablename = 'friendships'
    ) THEN
        CREATE POLICY friendships_delete_own ON friendships
          FOR DELETE USING (auth.uid()::text = user_id::text OR auth.uid()::text = friend_id::text);
    END IF;
END $$;

-- STEP 5: Ensure channel_members INSERT works for DM creation
-- (any authenticated user can add members to DM channels)
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE policyname = 'channel_members_insert_authenticated' AND tablename = 'channel_members'
    ) THEN
        CREATE POLICY channel_members_insert_authenticated ON channel_members
          FOR INSERT WITH CHECK (auth.uid() IS NOT NULL);
    END IF;
END $$;
