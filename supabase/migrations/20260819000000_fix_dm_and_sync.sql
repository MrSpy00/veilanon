-- ============================================================================
-- veilanon DM Support & Schema Sync Fix
-- Migration: 20260819000000_fix_dm_and_sync.sql
-- ============================================================================
-- Fixes:
-- 1. channels.space_id must be nullable (DM channels have no space)
-- 2. channels.channel_type check must include 'dm' and 'group_dm'
-- 3. Add missing columns: is_nsfw, topic, slow_mode_seconds
-- 4. Add missing message columns: sender_id, reply_to_id, pinned, reactions, attachments
-- 5. Add missing space columns: description, banner_hash, custom_link (if not exists)
-- 6. Add missing presence column: status (if not exists)

-- 1. Fix channels table: make space_id nullable and extend channel_type check
ALTER TABLE public.channels DROP CONSTRAINT IF EXISTS channels_channel_type_check;
ALTER TABLE public.channels ALTER COLUMN space_id DROP NOT NULL;
ALTER TABLE public.channels ADD CONSTRAINT channels_channel_type_check
    CHECK (channel_type IN ('text','voice','category','announcement','forum','dm','group_dm'));

-- Add missing channel columns
ALTER TABLE public.channels ADD COLUMN IF NOT EXISTS is_nsfw boolean NOT NULL DEFAULT false;
ALTER TABLE public.channels ADD COLUMN IF NOT EXISTS topic text DEFAULT '';
ALTER TABLE public.channels ADD COLUMN IF NOT EXISTS slow_mode_seconds integer NOT NULL DEFAULT 0;

-- 2. Fix messages table: add missing columns for full sync
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS sender_id uuid REFERENCES public.users(id) ON DELETE SET NULL;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS reply_to_id uuid;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS pinned boolean NOT NULL DEFAULT false;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS reactions jsonb NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS attachments jsonb NOT NULL DEFAULT '[]'::jsonb;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS crypto_meta text;
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS message_type text NOT NULL DEFAULT 'text';
ALTER TABLE public.messages ADD COLUMN IF NOT EXISTS status text NOT NULL DEFAULT 'sent';

-- 3. Ensure spaces table has all columns
ALTER TABLE public.spaces ADD COLUMN IF NOT EXISTS description text DEFAULT '';
ALTER TABLE public.spaces ADD COLUMN IF NOT EXISTS banner_hash text DEFAULT '';
ALTER TABLE public.spaces ADD COLUMN IF NOT EXISTS custom_link text DEFAULT '';

-- 4. Ensure memberships table has timeout support
ALTER TABLE public.memberships ADD COLUMN IF NOT EXISTS timeout_until bigint;

-- 5. Ensure bans table exists with proper schema
CREATE TABLE IF NOT EXISTS public.bans (
    space_id uuid NOT NULL REFERENCES public.spaces(id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    banned_by uuid REFERENCES public.users(id) ON DELETE SET NULL,
    reason text,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    PRIMARY KEY (space_id, user_id)
);
ALTER TABLE public.bans ENABLE ROW LEVEL SECURITY;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_policies WHERE tablename = 'bans' AND policyname = 'bans_all_access') THEN
        CREATE POLICY bans_all_access ON public.bans FOR ALL USING (true) WITH CHECK (true);
    END IF;
END $$;

-- 6. Add indexes for new query patterns
CREATE INDEX IF NOT EXISTS idx_messages_sender_id ON public.messages(sender_id);
CREATE INDEX IF NOT EXISTS idx_messages_reply_to ON public.messages(reply_to_id);
CREATE INDEX IF NOT EXISTS idx_messages_pinned ON public.messages(pinned) WHERE pinned = true;
CREATE INDEX IF NOT EXISTS idx_channels_dm_lookup ON public.channels(channel_type) WHERE channel_type IN ('dm', 'group_dm');
CREATE INDEX IF NOT EXISTS idx_memberships_timeout ON public.memberships(timeout_until) WHERE timeout_until IS NOT NULL;

-- 7. Enable realtime for bans table
DO $$
BEGIN
    ALTER PUBLICATION supabase_realtime ADD TABLE public.bans;
EXCEPTION WHEN OTHERS THEN NULL;
END $$;
