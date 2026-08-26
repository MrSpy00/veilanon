-- ============================================================================
-- veilanon Fix Realtime + Channel Members — Corrective migration
-- Migration: 20260819090000_fix_realtime_and_channel_members.sql
-- ============================================================================
-- This migration:
--   1. Adds all critical tables to supabase_realtime (idempotent)
--   2. Adds UPDATE policy for channel_members (upsert support)
--   3. Ensures friendships_delete_own exists

-- STEP 1: Ensure all critical tables are in supabase_realtime publication
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

-- STEP 2: Add UPDATE policy for channel_members (missing — causes upsert failures)
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE policyname = 'channel_members_update_own' AND tablename = 'channel_members'
    ) THEN
        CREATE POLICY channel_members_update_own ON channel_members
            FOR UPDATE USING (user_id = auth.uid()) WITH CHECK (user_id = auth.uid());
    END IF;
END $$;

-- STEP 3: Ensure friendships_delete_own exists
DO $$ BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_policies WHERE policyname = 'friendships_delete_own' AND tablename = 'friendships'
    ) THEN
        CREATE POLICY friendships_delete_own ON friendships
            FOR DELETE USING (auth.uid()::text = user_id::text OR auth.uid()::text = friend_id::text);
    END IF;
END $$;
