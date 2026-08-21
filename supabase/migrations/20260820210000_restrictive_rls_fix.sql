-- ============================================================================
-- veilanon Restrictive RLS Fix — Replaces world-open policies from
-- 20260820200000 with authenticated-only, least-privilege policies.
-- Secret env values are never embedded; this file contains no secrets.
-- ============================================================================

-- ── 1. spaces: only authenticated users can read; insert/update/delete = owner ─
ALTER TABLE public.spaces ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "spaces_select_all" ON public.spaces;
DROP POLICY IF EXISTS "spaces_insert_owner" ON public.spaces;
DROP POLICY IF EXISTS "spaces_update_owner" ON public.spaces;
DROP POLICY IF EXISTS "spaces_delete_owner" ON public.spaces;

CREATE POLICY "spaces_select_authenticated" ON public.spaces
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "spaces_insert_owner" ON public.spaces
    FOR INSERT TO authenticated
    WITH CHECK (owner_id = auth.uid());
CREATE POLICY "spaces_update_owner" ON public.spaces
    FOR UPDATE TO authenticated
    USING (owner_id = auth.uid())
    WITH CHECK (owner_id = auth.uid());
CREATE POLICY "spaces_delete_owner" ON public.spaces
    FOR DELETE TO authenticated
    USING (owner_id = auth.uid());

-- ── 2. memberships: select authenticated; insert/delete own or owner ────────
ALTER TABLE public.memberships ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "memberships_select_all" ON public.memberships;
DROP POLICY IF EXISTS "memberships_insert_own" ON public.memberships;
DROP POLICY IF EXISTS "memberships_delete_own" ON public.memberships;

CREATE POLICY "memberships_select_authenticated" ON public.memberships
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "memberships_insert_own" ON public.memberships
    FOR INSERT TO authenticated
    WITH CHECK (user_id = auth.uid());
CREATE POLICY "memberships_delete_own" ON public.memberships
    FOR DELETE TO authenticated
    USING (
        user_id = auth.uid()
        OR EXISTS (SELECT 1 FROM public.spaces s WHERE s.id = memberships.space_id AND s.owner_id = auth.uid())
    );

-- ── 3. channels: select authenticated; insert/update/delete authenticated ───
ALTER TABLE public.channels ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "channels_select_all" ON public.channels;
DROP POLICY IF EXISTS "channels_insert_all" ON public.channels;
DROP POLICY IF EXISTS "channels_update_all" ON public.channels;
DROP POLICY IF EXISTS "channels_delete_all" ON public.channels;

CREATE POLICY "channels_select_authenticated" ON public.channels
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "channels_insert_authenticated" ON public.channels
    FOR INSERT TO authenticated
    WITH CHECK (auth.uid() IS NOT NULL);
CREATE POLICY "channels_update_authenticated" ON public.channels
    FOR UPDATE TO authenticated
    USING (auth.uid() IS NOT NULL)
    WITH CHECK (auth.uid() IS NOT NULL);
CREATE POLICY "channels_delete_authenticated" ON public.channels
    FOR DELETE TO authenticated
    USING (auth.uid() IS NOT NULL);

-- ── 4. channel_members ───────────────────────────────────────────────────────
ALTER TABLE public.channel_members ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "channel_members_select_all" ON public.channel_members;
DROP POLICY IF EXISTS "channel_members_insert_all" ON public.channel_members;
DROP POLICY IF EXISTS "channel_members_update_all" ON public.channel_members;
DROP POLICY IF EXISTS "channel_members_delete_all" ON public.channel_members;

CREATE POLICY "channel_members_select_authenticated" ON public.channel_members
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "channel_members_insert_authenticated" ON public.channel_members
    FOR INSERT TO authenticated
    WITH CHECK (auth.uid() IS NOT NULL);
CREATE POLICY "channel_members_update_authenticated" ON public.channel_members
    FOR UPDATE TO authenticated
    USING (auth.uid() IS NOT NULL)
    WITH CHECK (auth.uid() IS NOT NULL);
CREATE POLICY "channel_members_delete_authenticated" ON public.channel_members
    FOR DELETE TO authenticated
    USING (auth.uid() IS NOT NULL);

-- ── 5. messages: ciphertext only; authenticated read/write ──────────────────
ALTER TABLE public.messages ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "messages_select_all" ON public.messages;
DROP POLICY IF EXISTS "messages_insert_all" ON public.messages;
DROP POLICY IF EXISTS "messages_update_all" ON public.messages;
DROP POLICY IF EXISTS "messages_delete_all" ON public.messages;

CREATE POLICY "messages_select_authenticated" ON public.messages
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "messages_insert_authenticated" ON public.messages
    FOR INSERT TO authenticated
    WITH CHECK (auth.uid() IS NOT NULL AND sender_id = auth.uid()::text);
CREATE POLICY "messages_update_authenticated" ON public.messages
    FOR UPDATE TO authenticated
    USING (auth.uid() IS NOT NULL)
    WITH CHECK (auth.uid() IS NOT NULL);
CREATE POLICY "messages_delete_authenticated" ON public.messages
    FOR DELETE TO authenticated
    USING (auth.uid() IS NOT NULL);

-- ── 6. devices ───────────────────────────────────────────────────────────────
ALTER TABLE public.devices ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "devices_select_all" ON public.devices;
DROP POLICY IF EXISTS "devices_insert_all" ON public.devices;
DROP POLICY IF EXISTS "devices_update_all" ON public.devices;
DROP POLICY IF EXISTS "devices_delete_all" ON public.devices;

CREATE POLICY "devices_select_authenticated" ON public.devices
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "devices_insert_own" ON public.devices
    FOR INSERT TO authenticated
    WITH CHECK (user_id = auth.uid());
CREATE POLICY "devices_update_own" ON public.devices
    FOR UPDATE TO authenticated
    USING (user_id = auth.uid())
    WITH CHECK (user_id = auth.uid());
CREATE POLICY "devices_delete_own" ON public.devices
    FOR DELETE TO authenticated
    USING (user_id = auth.uid());

-- ── 7. presence ───────────────────────────────────────────────────────────────
ALTER TABLE public.presence ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "presence_select_all" ON public.presence;
DROP POLICY IF EXISTS "presence_insert_all" ON public.presence;
DROP POLICY IF EXISTS "presence_update_all" ON public.presence;

CREATE POLICY "presence_select_authenticated" ON public.presence
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "presence_insert_own" ON public.presence
    FOR INSERT TO authenticated
    WITH CHECK (user_id = auth.uid());
CREATE POLICY "presence_update_own" ON public.presence
    FOR UPDATE TO authenticated
    USING (user_id = auth.uid())
    WITH CHECK (user_id = auth.uid());

-- ── 8. roles & role_members ──────────────────────────────────────────────────
ALTER TABLE public.roles ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "roles_select_all" ON public.roles;
DROP POLICY IF EXISTS "roles_insert_all" ON public.roles;
DROP POLICY IF EXISTS "roles_update_all" ON public.roles;
DROP POLICY IF EXISTS "roles_delete_all" ON public.roles;

CREATE POLICY "roles_select_authenticated" ON public.roles
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "roles_insert_authenticated" ON public.roles
    FOR INSERT TO authenticated
    WITH CHECK (auth.uid() IS NOT NULL);
CREATE POLICY "roles_update_authenticated" ON public.roles
    FOR UPDATE TO authenticated
    USING (auth.uid() IS NOT NULL)
    WITH CHECK (auth.uid() IS NOT NULL);
CREATE POLICY "roles_delete_authenticated" ON public.roles
    FOR DELETE TO authenticated
    USING (auth.uid() IS NOT NULL);

ALTER TABLE public.role_members ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "role_members_select_all" ON public.role_members;
DROP POLICY IF EXISTS "role_members_insert_all" ON public.role_members;
DROP POLICY IF EXISTS "role_members_delete_all" ON public.role_members;

CREATE POLICY "role_members_select_authenticated" ON public.role_members
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "role_members_insert_authenticated" ON public.role_members
    FOR INSERT TO authenticated
    WITH CHECK (auth.uid() IS NOT NULL);
CREATE POLICY "role_members_delete_authenticated" ON public.role_members
    FOR DELETE TO authenticated
    USING (auth.uid() IS NOT NULL);

-- ── 9. friendships ───────────────────────────────────────────────────────────
ALTER TABLE public.friendships ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "friendships_select_all" ON public.friendships;
DROP POLICY IF EXISTS "friendships_insert_all" ON public.friendships;
DROP POLICY IF EXISTS "friendships_update_all" ON public.friendships;
DROP POLICY IF EXISTS "friendships_delete_all" ON public.friendships;

CREATE POLICY "friendships_select_authenticated" ON public.friendships
    FOR SELECT TO authenticated
    USING (auth.uid() = user_id OR auth.uid() = friend_id);
CREATE POLICY "friendships_insert_own" ON public.friendships
    FOR INSERT TO authenticated
    WITH CHECK (user_id = auth.uid());
CREATE POLICY "friendships_update_own" ON public.friendships
    FOR UPDATE TO authenticated
    USING (user_id = auth.uid() OR friend_id = auth.uid())
    WITH CHECK (user_id = auth.uid() OR friend_id = auth.uid());
CREATE POLICY "friendships_delete_own" ON public.friendships
    FOR DELETE TO authenticated
    USING (user_id = auth.uid() OR friend_id = auth.uid());

-- ── 10. invites & bans ───────────────────────────────────────────────────────
ALTER TABLE public.invites ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "invites_select_all" ON public.invites;
DROP POLICY IF EXISTS "invites_insert_all" ON public.invites;
DROP POLICY IF EXISTS "invites_delete_all" ON public.invites;

CREATE POLICY "invites_select_authenticated" ON public.invites
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "invites_insert_authenticated" ON public.invites
    FOR INSERT TO authenticated
    WITH CHECK (auth.uid() IS NOT NULL);
CREATE POLICY "invites_delete_authenticated" ON public.invites
    FOR DELETE TO authenticated
    USING (auth.uid() IS NOT NULL);

ALTER TABLE public.bans ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "bans_all" ON public.bans;
CREATE POLICY "bans_select_authenticated" ON public.bans
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "bans_insert_authenticated" ON public.bans
    FOR INSERT TO authenticated
    WITH CHECK (auth.uid() IS NOT NULL);
CREATE POLICY "bans_delete_authenticated" ON public.bans
    FOR DELETE TO authenticated
    USING (auth.uid() IS NOT NULL);

-- ── 11. files (storage metadata) ─────────────────────────────────────────────
ALTER TABLE public.files ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "files_select_all" ON public.files;
DROP POLICY IF EXISTS "files_insert_all" ON public.files;
DROP POLICY IF EXISTS "files_delete_all" ON public.files;

CREATE POLICY "files_select_authenticated" ON public.files
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
CREATE POLICY "files_insert_authenticated" ON public.files
    FOR INSERT TO authenticated
    WITH CHECK (auth.uid() IS NOT NULL);
CREATE POLICY "files_delete_authenticated" ON public.files
    FOR DELETE TO authenticated
    USING (auth.uid() IS NOT NULL);

-- ── 12. users: authenticated read, own write ─────────────────────────────────
ALTER TABLE public.users ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "users_select_all" ON public.users;
CREATE POLICY "users_select_authenticated" ON public.users
    FOR SELECT TO authenticated
    USING (auth.uid() IS NOT NULL);
DROP POLICY IF EXISTS "users_insert_own" ON public.users;
CREATE POLICY "users_insert_own" ON public.users
    FOR INSERT TO authenticated
    WITH CHECK (id = auth.uid());
DROP POLICY IF EXISTS "users_update_own" ON public.users;
CREATE POLICY "users_update_own" ON public.users
    FOR UPDATE TO authenticated
    USING (id = auth.uid())
    WITH CHECK (id = auth.uid());

-- Re-apply grants (authenticated only, anon removed from data plane)
REVOKE ALL ON ALL TABLES IN SCHEMA public FROM anon, public;
GRANT USAGE ON SCHEMA public TO authenticated, service_role;
GRANT ALL ON ALL TABLES IN SCHEMA public TO authenticated, service_role;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO authenticated, service_role;
GRANT ALL ON ALL ROUTINES IN SCHEMA public TO authenticated, service_role;
-- anon sadece auth için gerekli minimal yetkiyi korur
GRANT USAGE ON SCHEMA public TO anon;
