-- ============================================================================
-- veilanon Comprehensive Realtime, Storage & Full-Duplex Sync Migration
-- Migration: 20260820160000_comprehensive_realtime_and_storage_sync.sql
-- ============================================================================

-- ── 1. Enable REPLICA IDENTITY FULL on all tables ──────────────────────────
-- Ensures that all UPDATE and DELETE events sent through Supabase Realtime
-- contain full row data (including updated avatar_hash, banner_hash,
-- custom_status, bio, reactions, attachments, role permissions, etc.)
DO $$
DECLARE
    tbl text;
    tables text[] := ARRAY[
        'users', 'spaces', 'channels', 'channel_members', 'messages',
        'roles', 'role_members', 'memberships', 'presence', 'friendships',
        'devices', 'bans', 'invites', 'files', 'mls_welcomes',
        'discord_webhooks', 'audit_events'
    ];
BEGIN
    FOREACH tbl IN ARRAY tables LOOP
        BEGIN
            EXECUTE format('ALTER TABLE public.%I REPLICA IDENTITY FULL;', tbl);
        EXCEPTION WHEN OTHERS THEN
            RAISE NOTICE 'Could not set REPLICA IDENTITY FULL on %: %', tbl, SQLERRM;
        END;
    END LOOP;
END $$;

-- ── 2. Ensure all tables are published to supabase_realtime ────────────────
DO $$
DECLARE
    tbl text;
    tables text[] := ARRAY[
        'users', 'spaces', 'channels', 'channel_members', 'messages',
        'roles', 'role_members', 'memberships', 'presence', 'friendships',
        'devices', 'bans', 'invites', 'files', 'mls_welcomes',
        'discord_webhooks', 'audit_events'
    ];
BEGIN
    FOREACH tbl IN ARRAY tables LOOP
        BEGIN
            EXECUTE format('ALTER PUBLICATION supabase_realtime ADD TABLE public.%I;', tbl);
        EXCEPTION WHEN OTHERS THEN
            NULL; -- already in publication
        END;
    END LOOP;
END $$;

-- ── 3. Configure storage buckets: avatars, banners, files ──────────────────
INSERT INTO storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
VALUES 
    ('avatars', 'avatars', true, 10485760, ARRAY['image/jpeg', 'image/png', 'image/webp', 'image/gif']),
    ('banners', 'banners', true, 15728640, ARRAY['image/jpeg', 'image/png', 'image/webp', 'image/gif']),
    ('files', 'files', true, 52428800, NULL)
ON CONFLICT (id) DO UPDATE SET
    public = true,
    file_size_limit = EXCLUDED.file_size_limit,
    allowed_mime_types = EXCLUDED.allowed_mime_types;

-- Storage object policies (Public Read, Authenticated/Anon Insert/Update/Delete)
DO $$
BEGIN
    -- Drop existing storage policies for clean recreate
    DROP POLICY IF EXISTS "Public Read Access on Buckets" ON storage.objects;
    DROP POLICY IF EXISTS "Authenticated and Anon Upload to Buckets" ON storage.objects;
    DROP POLICY IF EXISTS "Authenticated and Anon Update on Buckets" ON storage.objects;
    DROP POLICY IF EXISTS "Authenticated and Anon Delete on Buckets" ON storage.objects;
    
    CREATE POLICY "Public Read Access on Buckets" ON storage.objects
        FOR SELECT USING (bucket_id IN ('avatars', 'banners', 'files'));

    CREATE POLICY "Authenticated and Anon Upload to Buckets" ON storage.objects
        FOR INSERT WITH CHECK (bucket_id IN ('avatars', 'banners', 'files'));

    CREATE POLICY "Authenticated and Anon Update on Buckets" ON storage.objects
        FOR UPDATE USING (bucket_id IN ('avatars', 'banners', 'files'))
        WITH CHECK (bucket_id IN ('avatars', 'banners', 'files'));

    CREATE POLICY "Authenticated and Anon Delete on Buckets" ON storage.objects
        FOR DELETE USING (bucket_id IN ('avatars', 'banners', 'files'));
EXCEPTION WHEN OTHERS THEN
    RAISE NOTICE 'Storage policy update notice: %', SQLERRM;
END $$;

-- ── 4. Ensure auth trigger creates public user profile ─────────────────────
CREATE OR REPLACE FUNCTION public.handle_new_user()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER SET search_path = public
AS $$
BEGIN
    INSERT INTO public.users (id, username, display_name, created_at)
    VALUES (
        new.id,
        COALESCE(new.raw_user_meta_data->>'username', split_part(new.email, '@', 1)),
        COALESCE(new.raw_user_meta_data->>'display_name', split_part(new.email, '@', 1)),
        now()
    )
    ON CONFLICT (id) DO UPDATE SET
        username = EXCLUDED.username,
        display_name = EXCLUDED.display_name;
    RETURN new;
END;
$$;

DROP TRIGGER IF EXISTS on_auth_user_created ON auth.users;
CREATE TRIGGER on_auth_user_created
    AFTER INSERT ON auth.users
    FOR EACH ROW EXECUTE FUNCTION public.handle_new_user();

-- ── 5. Grant permissions to authenticated & anon roles ─────────────────────
GRANT USAGE ON SCHEMA public TO anon, authenticated, service_role;
GRANT ALL ON ALL TABLES IN SCHEMA public TO anon, authenticated, service_role;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO anon, authenticated, service_role;
GRANT ALL ON ALL ROUTINES IN SCHEMA public TO anon, authenticated, service_role;

ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO anon, authenticated, service_role;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO anon, authenticated, service_role;
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON ROUTINES TO anon, authenticated, service_role;
