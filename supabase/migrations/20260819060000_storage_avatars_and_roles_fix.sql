-- ============================================================================
-- veilanon Storage Avatars Bucket, Role Members Constraint & Profile Sync Fix
-- Migration: 20260819060000_storage_avatars_and_roles_fix.sql
-- ============================================================================

-- 1. Ensure public 'avatars' bucket exists in Supabase Storage
INSERT INTO storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
VALUES ('avatars', 'avatars', true, 10485760, ARRAY['image/jpeg', 'image/png', 'image/webp', 'image/gif'])
ON CONFLICT (id) DO UPDATE SET public = true;

-- Ensure public 'files' bucket also exists
INSERT INTO storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
VALUES ('files', 'files', true, 52428800, null)
ON CONFLICT (id) DO UPDATE SET public = true;

-- 2. Storage Objects RLS Policies for both 'files' and 'avatars' buckets
DROP POLICY IF EXISTS "Allow public read on storage buckets" ON storage.objects;
DROP POLICY IF EXISTS "Allow public insert on storage buckets" ON storage.objects;
DROP POLICY IF EXISTS "Allow public update on storage buckets" ON storage.objects;
DROP POLICY IF EXISTS "Allow public delete on storage buckets" ON storage.objects;

DROP POLICY IF EXISTS "Allow public read on files bucket" ON storage.objects;
DROP POLICY IF EXISTS "Allow public insert on files bucket" ON storage.objects;
DROP POLICY IF EXISTS "Allow public update on files bucket" ON storage.objects;
DROP POLICY IF EXISTS "Allow public delete on files bucket" ON storage.objects;

DROP POLICY IF EXISTS "Allow public read on avatars bucket" ON storage.objects;
DROP POLICY IF EXISTS "Allow public insert on avatars bucket" ON storage.objects;
DROP POLICY IF EXISTS "Allow public update on avatars bucket" ON storage.objects;
DROP POLICY IF EXISTS "Allow public delete on avatars bucket" ON storage.objects;

CREATE POLICY "Allow public read on storage buckets"
ON storage.objects FOR SELECT
TO anon, authenticated
USING (bucket_id IN ('files', 'avatars'));

CREATE POLICY "Allow public insert on storage buckets"
ON storage.objects FOR INSERT
TO anon, authenticated
WITH CHECK (bucket_id IN ('files', 'avatars'));

CREATE POLICY "Allow public update on storage buckets"
ON storage.objects FOR UPDATE
TO anon, authenticated
USING (bucket_id IN ('files', 'avatars'));

CREATE POLICY "Allow public delete on storage buckets"
ON storage.objects FOR DELETE
TO anon, authenticated
USING (bucket_id IN ('files', 'avatars'));

-- 3. Fix role_members space_id column constraint if needed
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_schema = 'public' AND table_name = 'role_members' AND column_name = 'space_id' AND is_nullable = 'NO'
    ) THEN
        ALTER TABLE public.role_members ALTER COLUMN space_id DROP NOT NULL;
    END IF;
END $$;

-- Ensure users table has all required sync columns
ALTER TABLE public.users ADD COLUMN IF NOT EXISTS bio text DEFAULT '';
ALTER TABLE public.users ADD COLUMN IF NOT EXISTS custom_status text DEFAULT '';
ALTER TABLE public.users ADD COLUMN IF NOT EXISTS banner_hash text DEFAULT '';
ALTER TABLE public.users ADD COLUMN IF NOT EXISTS dm_privacy text DEFAULT 'everyone';

-- Ensure presence table has custom_status and dm_privacy
ALTER TABLE public.presence ADD COLUMN IF NOT EXISTS custom_status text DEFAULT '';
ALTER TABLE public.presence ADD COLUMN IF NOT EXISTS dm_privacy text DEFAULT 'everyone';

-- Ensure all tables are in Realtime publication
DO $$
BEGIN
    ALTER PUBLICATION supabase_realtime ADD TABLE public.users;
EXCEPTION WHEN OTHERS THEN NULL;
END $$;

DO $$
BEGIN
    ALTER PUBLICATION supabase_realtime ADD TABLE public.presence;
EXCEPTION WHEN OTHERS THEN NULL;
END $$;

DO $$
BEGIN
    ALTER PUBLICATION supabase_realtime ADD TABLE public.role_members;
EXCEPTION WHEN OTHERS THEN NULL;
END $$;

DO $$
BEGIN
    ALTER PUBLICATION supabase_realtime ADD TABLE public.spaces;
EXCEPTION WHEN OTHERS THEN NULL;
END $$;
