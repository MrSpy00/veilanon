-- ============================================================================
-- veilanon Universal E2EE Zero-Knowledge RLS Policies Migration
-- Migration: 20260820200000_universal_e2ee_zero_knowledge_rls.sql
-- ============================================================================

-- ── 1. spaces ──────────────────────────────────────────────────────────────
ALTER TABLE public.spaces ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "spaces_select_member" ON public.spaces;
DROP POLICY IF EXISTS "spaces_select_public" ON public.spaces;
DROP POLICY IF EXISTS "spaces_insert_owner" ON public.spaces;
DROP POLICY IF EXISTS "spaces_update_owner" ON public.spaces;
DROP POLICY IF EXISTS "spaces_delete_owner" ON public.spaces;

CREATE POLICY "spaces_select_all" ON public.spaces
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "spaces_insert_owner" ON public.spaces
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (owner_id = auth.uid() OR auth.uid() IS NULL);

CREATE POLICY "spaces_update_owner" ON public.spaces
    FOR UPDATE TO public, anon, authenticated
    USING (owner_id = auth.uid() OR auth.uid() IS NULL)
    WITH CHECK (owner_id = auth.uid() OR auth.uid() IS NULL);

CREATE POLICY "spaces_delete_owner" ON public.spaces
    FOR DELETE TO public, anon, authenticated
    USING (owner_id = auth.uid() OR auth.uid() IS NULL);

-- ── 2. memberships ──────────────────────────────────────────────────────────
ALTER TABLE public.memberships ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "memberships_select_own" ON public.memberships;
DROP POLICY IF EXISTS "memberships_select_member" ON public.memberships;
DROP POLICY IF EXISTS "memberships_insert_own" ON public.memberships;
DROP POLICY IF EXISTS "memberships_delete_own" ON public.memberships;
DROP POLICY IF EXISTS "owner_manages_memberships" ON public.memberships;

CREATE POLICY "memberships_select_all" ON public.memberships
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "memberships_insert_own" ON public.memberships
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (user_id = auth.uid() OR auth.uid() IS NULL);

CREATE POLICY "memberships_delete_own" ON public.memberships
    FOR DELETE TO public, anon, authenticated
    USING (
        user_id = auth.uid() 
        OR auth.uid() IS NULL 
        OR EXISTS (SELECT 1 FROM public.spaces s WHERE s.id = memberships.space_id AND (s.owner_id = auth.uid() OR auth.uid() IS NULL))
    );

-- ── 3. channels ────────────────────────────────────────────────────────────
ALTER TABLE public.channels ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "channels_select_member" ON public.channels;
DROP POLICY IF EXISTS "channels_insert_member" ON public.channels;
DROP POLICY IF EXISTS "channels_update_member" ON public.channels;
DROP POLICY IF EXISTS "channels_delete_member" ON public.channels;

CREATE POLICY "channels_select_all" ON public.channels
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "channels_insert_all" ON public.channels
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (true);

CREATE POLICY "channels_update_all" ON public.channels
    FOR UPDATE TO public, anon, authenticated
    USING (true)
    WITH CHECK (true);

CREATE POLICY "channels_delete_all" ON public.channels
    FOR DELETE TO public, anon, authenticated
    USING (true);

-- ── 4. channel_members ─────────────────────────────────────────────────────
ALTER TABLE public.channel_members ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "channel_members_select_own" ON public.channel_members;
DROP POLICY IF EXISTS "channel_members_select_member" ON public.channel_members;
DROP POLICY IF EXISTS "channel_members_insert_own" ON public.channel_members;
DROP POLICY IF EXISTS "channel_members_update_own" ON public.channel_members;
DROP POLICY IF EXISTS "channel_members_delete_own" ON public.channel_members;

CREATE POLICY "channel_members_select_all" ON public.channel_members
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "channel_members_insert_all" ON public.channel_members
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (true);

CREATE POLICY "channel_members_update_all" ON public.channel_members
    FOR UPDATE TO public, anon, authenticated
    USING (true)
    WITH CHECK (true);

CREATE POLICY "channel_members_delete_all" ON public.channel_members
    FOR DELETE TO public, anon, authenticated
    USING (true);

-- ── 5. messages ────────────────────────────────────────────────────────────
ALTER TABLE public.messages ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "messages_select_member" ON public.messages;
DROP POLICY IF EXISTS "messages_insert_member" ON public.messages;
DROP POLICY IF EXISTS "messages_update_member" ON public.messages;
DROP POLICY IF EXISTS "messages_delete_member" ON public.messages;

CREATE POLICY "messages_select_all" ON public.messages
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "messages_insert_all" ON public.messages
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (true);

CREATE POLICY "messages_update_all" ON public.messages
    FOR UPDATE TO public, anon, authenticated
    USING (true)
    WITH CHECK (true);

CREATE POLICY "messages_delete_all" ON public.messages
    FOR DELETE TO public, anon, authenticated
    USING (true);

-- ── 6. devices ─────────────────────────────────────────────────────────────
ALTER TABLE public.devices ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "devices_select_own" ON public.devices;
DROP POLICY IF EXISTS "devices_select_authenticated" ON public.devices;
DROP POLICY IF EXISTS "devices_insert_own" ON public.devices;
DROP POLICY IF EXISTS "devices_update_own" ON public.devices;
DROP POLICY IF EXISTS "devices_delete_own" ON public.devices;

CREATE POLICY "devices_select_all" ON public.devices
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "devices_insert_all" ON public.devices
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (user_id = auth.uid() OR auth.uid() IS NULL);

CREATE POLICY "devices_update_all" ON public.devices
    FOR UPDATE TO public, anon, authenticated
    USING (user_id = auth.uid() OR auth.uid() IS NULL)
    WITH CHECK (user_id = auth.uid() OR auth.uid() IS NULL);

CREATE POLICY "devices_delete_all" ON public.devices
    FOR DELETE TO public, anon, authenticated
    USING (user_id = auth.uid() OR auth.uid() IS NULL);

-- ── 7. presence ────────────────────────────────────────────────────────────
ALTER TABLE public.presence ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "presence_select_any" ON public.presence;
DROP POLICY IF EXISTS "presence_insert_own" ON public.presence;
DROP POLICY IF EXISTS "presence_update_own" ON public.presence;

CREATE POLICY "presence_select_all" ON public.presence
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "presence_insert_all" ON public.presence
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (true);

CREATE POLICY "presence_update_all" ON public.presence
    FOR UPDATE TO public, anon, authenticated
    USING (true)
    WITH CHECK (true);

-- ── 8. roles & role_members ────────────────────────────────────────────────
ALTER TABLE public.roles ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "roles_select_member" ON public.roles;
DROP POLICY IF EXISTS "roles_insert_member" ON public.roles;
DROP POLICY IF EXISTS "roles_update_member" ON public.roles;
DROP POLICY IF EXISTS "roles_delete_member" ON public.roles;

CREATE POLICY "roles_select_all" ON public.roles
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "roles_insert_all" ON public.roles
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (true);

CREATE POLICY "roles_update_all" ON public.roles
    FOR UPDATE TO public, anon, authenticated
    USING (true)
    WITH CHECK (true);

CREATE POLICY "roles_delete_all" ON public.roles
    FOR DELETE TO public, anon, authenticated
    USING (true);

ALTER TABLE public.role_members ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "role_members_select_member" ON public.role_members;
DROP POLICY IF EXISTS "role_members_insert_member" ON public.role_members;
DROP POLICY IF EXISTS "role_members_delete_member" ON public.role_members;

CREATE POLICY "role_members_select_all" ON public.role_members
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "role_members_insert_all" ON public.role_members
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (true);

CREATE POLICY "role_members_delete_all" ON public.role_members
    FOR DELETE TO public, anon, authenticated
    USING (true);

-- ── 9. friendships ─────────────────────────────────────────────────────────
ALTER TABLE public.friendships ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "friendships_select_own" ON public.friendships;
DROP POLICY IF EXISTS "friendships_insert_own" ON public.friendships;
DROP POLICY IF EXISTS "friendships_update_own" ON public.friendships;
DROP POLICY IF EXISTS "friendships_delete_own" ON public.friendships;

CREATE POLICY "friendships_select_all" ON public.friendships
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "friendships_insert_all" ON public.friendships
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (true);

CREATE POLICY "friendships_update_all" ON public.friendships
    FOR UPDATE TO public, anon, authenticated
    USING (true)
    WITH CHECK (true);

CREATE POLICY "friendships_delete_all" ON public.friendships
    FOR DELETE TO public, anon, authenticated
    USING (true);

-- ── 10. invites & bans ─────────────────────────────────────────────────────
ALTER TABLE public.invites ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "invites_select_member" ON public.invites;
DROP POLICY IF EXISTS "invites_insert_member" ON public.invites;
DROP POLICY IF EXISTS "invites_delete_member" ON public.invites;

CREATE POLICY "invites_select_all" ON public.invites
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "invites_insert_all" ON public.invites
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (true);

CREATE POLICY "invites_delete_all" ON public.invites
    FOR DELETE TO public, anon, authenticated
    USING (true);

ALTER TABLE public.bans ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "owner_manages_bans" ON public.bans;

CREATE POLICY "bans_all" ON public.bans
    FOR ALL TO public, anon, authenticated
    USING (true)
    WITH CHECK (true);

-- ── 11. files ──────────────────────────────────────────────────────────────
ALTER TABLE public.files ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "files_select_own" ON public.files;
DROP POLICY IF EXISTS "files_insert_own" ON public.files;
DROP POLICY IF EXISTS "files_delete_own" ON public.files;

CREATE POLICY "files_select_all" ON public.files
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "files_insert_all" ON public.files
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (true);

CREATE POLICY "files_delete_all" ON public.files
    FOR DELETE TO public, anon, authenticated
    USING (true);

-- ── 12. Permissions grant ──────────────────────────────────────────────────
GRANT USAGE ON SCHEMA public TO anon, authenticated, service_role;
GRANT ALL ON ALL TABLES IN SCHEMA public TO anon, authenticated, service_role;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO anon, authenticated, service_role;
GRANT ALL ON ALL ROUTINES IN SCHEMA public TO anon, authenticated, service_role;
