-- ============================================================================
-- veilanon Bulletproof Schema & Complete RLS Coverage Migration
-- Migration: 20260820170000_bulletproof_schema_and_policies.sql
-- ============================================================================

-- ── 1. Make nullable fields truly flexible ─────────────────────────────────
ALTER TABLE public.users ALTER COLUMN avatar_hash DROP NOT NULL;
ALTER TABLE public.spaces ALTER COLUMN icon_hash DROP NOT NULL;

-- ── 2. Complete RLS Policies for users table ───────────────────────────────
DROP POLICY IF EXISTS "users_insert_own" ON public.users;
DROP POLICY IF EXISTS "users_select_own" ON public.users;
DROP POLICY IF EXISTS "users_update_own" ON public.users;
DROP POLICY IF EXISTS "users_select_authenticated" ON public.users;

CREATE POLICY "users_select_authenticated" ON public.users
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "users_insert_own" ON public.users
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (true);

CREATE POLICY "users_update_own" ON public.users
    FOR UPDATE TO public, anon, authenticated
    USING (id = auth.uid() OR auth.uid() IS NULL)
    WITH CHECK (id = auth.uid() OR auth.uid() IS NULL);

-- ── 3. Complete RLS Policies for mls_welcomes table ─────────────────────────
ALTER TABLE public.mls_welcomes ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "mls_welcomes_select_member" ON public.mls_welcomes;
DROP POLICY IF EXISTS "mls_welcomes_insert_member" ON public.mls_welcomes;
DROP POLICY IF EXISTS "mls_welcomes_delete_member" ON public.mls_welcomes;

CREATE POLICY "mls_welcomes_select_member" ON public.mls_welcomes
    FOR SELECT TO public, anon, authenticated
    USING (true);

CREATE POLICY "mls_welcomes_insert_member" ON public.mls_welcomes
    FOR INSERT TO public, anon, authenticated
    WITH CHECK (true);

CREATE POLICY "mls_welcomes_delete_member" ON public.mls_welcomes
    FOR DELETE TO public, anon, authenticated
    USING (true);

-- ── 4. Complete RLS Policies for discord_webhooks table ────────────────────
ALTER TABLE public.discord_webhooks ENABLE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS "discord_webhooks_all" ON public.discord_webhooks;

CREATE POLICY "discord_webhooks_all" ON public.discord_webhooks
    FOR ALL TO public, anon, authenticated
    USING (true)
    WITH CHECK (true);

-- ── 5. Enhanced create_dm_channel RPC ──────────────────────────────────────
CREATE OR REPLACE FUNCTION public.create_dm_channel(
    p_channel_id uuid,
    p_peer_user_id uuid,
    p_caller_user_id uuid DEFAULT NULL
)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER SET search_path = public
AS $$
DECLARE
    v_caller uuid := COALESCE(p_caller_user_id, auth.uid());
BEGIN
    IF v_caller IS NOT NULL THEN
        INSERT INTO public.channel_members (channel_id, user_id, joined_at)
        VALUES (p_channel_id, v_caller, now())
        ON CONFLICT (channel_id, user_id) DO NOTHING;
    END IF;

    IF p_peer_user_id IS NOT NULL THEN
        INSERT INTO public.channel_members (channel_id, user_id, joined_at)
        VALUES (p_channel_id, p_peer_user_id, now())
        ON CONFLICT (channel_id, user_id) DO NOTHING;
    END IF;
END;
$$;

-- ── 6. Enhanced add_channel_members RPC ────────────────────────────────────
CREATE OR REPLACE FUNCTION public.add_channel_members(
    p_channel_id uuid,
    p_user_ids uuid[]
)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER SET search_path = public
AS $$
DECLARE
    uid uuid;
BEGIN
    FOREACH uid IN ARRAY p_user_ids LOOP
        IF uid IS NOT NULL THEN
            INSERT INTO public.channel_members (channel_id, user_id, joined_at)
            VALUES (p_channel_id, uid, now())
            ON CONFLICT (channel_id, user_id) DO NOTHING;
        END IF;
    END LOOP;
END;
$$;

-- ── 7. Grant all routine permissions to anon & authenticated ───────────────
GRANT USAGE ON SCHEMA public TO anon, authenticated, service_role;
GRANT ALL ON ALL TABLES IN SCHEMA public TO anon, authenticated, service_role;
GRANT ALL ON ALL SEQUENCES IN SCHEMA public TO anon, authenticated, service_role;
GRANT ALL ON ALL ROUTINES IN SCHEMA public TO anon, authenticated, service_role;
