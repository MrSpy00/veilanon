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
