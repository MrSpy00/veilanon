-- ============================================================================
-- veilanon Realtime Users & Storage Avatars Publication
-- Migration: 20260819030000_realtime_users_and_avatars.sql
-- ============================================================================

-- Ensure users table is in supabase_realtime publication
DO $$
BEGIN
    BEGIN
        ALTER PUBLICATION supabase_realtime ADD TABLE public.users;
    EXCEPTION WHEN OTHERS THEN
        NULL;
    END;
END $$;

-- Ensure presence table is in supabase_realtime publication
DO $$
BEGIN
    BEGIN
        ALTER PUBLICATION supabase_realtime ADD TABLE public.presence;
    EXCEPTION WHEN OTHERS THEN
        NULL;
    END;
END $$;
