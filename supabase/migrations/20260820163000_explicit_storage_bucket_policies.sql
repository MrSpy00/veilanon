-- ============================================================================
-- veilanon Explicit Storage Bucket Policies (Banners, Avatars, Files)
-- Migration: 20260820163000_explicit_storage_bucket_policies.sql
-- ============================================================================

-- Ensure buckets exist and are public
INSERT INTO storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
VALUES 
    ('avatars', 'avatars', true, 10485760, ARRAY['image/jpeg', 'image/png', 'image/webp', 'image/gif']),
    ('banners', 'banners', true, 15728640, ARRAY['image/jpeg', 'image/png', 'image/webp', 'image/gif']),
    ('files', 'files', true, 52428800, NULL)
ON CONFLICT (id) DO UPDATE SET
    public = true,
    file_size_limit = EXCLUDED.file_size_limit,
    allowed_mime_types = EXCLUDED.allowed_mime_types;

-- Drop all old/generic policies on storage.objects
DO $$
DECLARE
    pol record;
BEGIN
    FOR pol IN 
        SELECT policyname 
        FROM pg_policies 
        WHERE schemaname = 'storage' AND tablename = 'objects'
    LOOP
        EXECUTE format('DROP POLICY IF EXISTS %I ON storage.objects;', pol.policyname);
    END LOOP;
END $$;

-- ── 1. BANNERS BUCKET POLICIES ─────────────────────────────────────────────
CREATE POLICY "Banners Public Read Access" ON storage.objects
    FOR SELECT TO public, anon, authenticated
    USING (bucket_id = 'banners');

CREATE POLICY "Banners Upload Access" ON storage.objects
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (bucket_id = 'banners');

CREATE POLICY "Banners Update Access" ON storage.objects
    FOR UPDATE TO public, anon, authenticated
    USING (bucket_id = 'banners')
    WITH CHECK (bucket_id = 'banners');

CREATE POLICY "Banners Delete Access" ON storage.objects
    FOR DELETE TO public, anon, authenticated
    USING (bucket_id = 'banners');

-- ── 2. AVATARS BUCKET POLICIES ─────────────────────────────────────────────
CREATE POLICY "Avatars Public Read Access" ON storage.objects
    FOR SELECT TO public, anon, authenticated
    USING (bucket_id = 'avatars');

CREATE POLICY "Avatars Upload Access" ON storage.objects
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (bucket_id = 'avatars');

CREATE POLICY "Avatars Update Access" ON storage.objects
    FOR UPDATE TO public, anon, authenticated
    USING (bucket_id = 'avatars')
    WITH CHECK (bucket_id = 'avatars');

CREATE POLICY "Avatars Delete Access" ON storage.objects
    FOR DELETE TO public, anon, authenticated
    USING (bucket_id = 'avatars');

-- ── 3. FILES BUCKET POLICIES ───────────────────────────────────────────────
CREATE POLICY "Files Public Read Access" ON storage.objects
    FOR SELECT TO public, anon, authenticated
    USING (bucket_id = 'files');

CREATE POLICY "Files Upload Access" ON storage.objects
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (bucket_id = 'files');

CREATE POLICY "Files Update Access" ON storage.objects
    FOR UPDATE TO public, anon, authenticated
    USING (bucket_id = 'files')
    WITH CHECK (bucket_id = 'files');

CREATE POLICY "Files Delete Access" ON storage.objects
    FOR DELETE TO public, anon, authenticated
    USING (bucket_id = 'files');
