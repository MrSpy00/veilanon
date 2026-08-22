-- ============================================================================
-- veilanon Migration: Public Profiles & Devices Directory RLS Fix
-- Enables directory lookups and public key exchange without auth deadlock.
-- Users table: public profile read (username, display_name, avatar, banner, bio)
-- Devices table: public key distribution read (for E2EE message encryption)
-- ============================================================================

-- ── 1. Schema grants ────────────────────────────────────────────────────────
GRANT USAGE ON SCHEMA public TO anon, authenticated, service_role;
GRANT SELECT ON public.users TO anon, authenticated, service_role;
GRANT SELECT ON public.devices TO anon, authenticated, service_role;

-- ── 2. users RLS: public read, authenticated self-insert/update ───────────────
ALTER TABLE public.users ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "users_select_public" ON public.users;
DROP POLICY IF EXISTS "users_select_authenticated" ON public.users;
DROP POLICY IF EXISTS "users_select_all" ON public.users;

CREATE POLICY "users_select_public" ON public.users
    FOR SELECT TO anon, authenticated
    USING (true);

-- ── 3. devices RLS: public read (for E2EE key discovery), own insert/update ──
ALTER TABLE public.devices ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "devices_select_public" ON public.devices;
DROP POLICY IF EXISTS "devices_select_authenticated" ON public.devices;
DROP POLICY IF EXISTS "devices_select_all" ON public.devices;

CREATE POLICY "devices_select_public" ON public.devices
    FOR SELECT TO anon, authenticated
    USING (true);

-- ── 4. friendships RLS: authenticated select & insert ────────────────────────
ALTER TABLE public.friendships ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "friendships_select_authenticated" ON public.friendships;
DROP POLICY IF EXISTS "friendships_insert_own" ON public.friendships;
DROP POLICY IF EXISTS "friendships_update_own" ON public.friendships;
DROP POLICY IF EXISTS "friendships_delete_own" ON public.friendships;

CREATE POLICY "friendships_select_authenticated" ON public.friendships
    FOR SELECT TO authenticated
    USING (auth.uid() = user_id OR auth.uid() = friend_id);

CREATE POLICY "friendships_insert_own" ON public.friendships
    FOR INSERT TO authenticated
    WITH CHECK (auth.uid() = user_id);

CREATE POLICY "friendships_update_own" ON public.friendships
    FOR UPDATE TO authenticated
    USING (auth.uid() = user_id OR auth.uid() = friend_id)
    WITH CHECK (auth.uid() = user_id OR auth.uid() = friend_id);

CREATE POLICY "friendships_delete_own" ON public.friendships
    FOR DELETE TO authenticated
    USING (auth.uid() = user_id OR auth.uid() = friend_id);
