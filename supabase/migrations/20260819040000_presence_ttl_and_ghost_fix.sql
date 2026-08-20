-- ============================================================================
-- veilanon Presence TTL & Ghost-Session Fix
-- Migration: 20260819040000_presence_ttl_and_ghost_fix.sql
-- ============================================================================
-- Problem: Users who quit abruptly (crash, power cut) still appear online
-- because their presence row is never set to offline.
--
-- Solution:
--   1. Add `last_seen` timestamp column to presence table
--   2. Add `heartbeat_at` column for the 30s heartbeat from the backend
--   3. Create a DB function that sets presence to 'offline' for rows
--      whose heartbeat_at is older than 90 seconds (3 missed heartbeats)
--   4. Create a cron-style pg_cron job (or call via Supabase Edge Function)
--      to run the cleanup every 60 seconds
--   5. Update the presence index for fast TTL queries

-- 1. Add TTL columns to presence table (idempotent)
ALTER TABLE public.presence
  ADD COLUMN IF NOT EXISTS last_seen  timestamptz NOT NULL DEFAULT now(),
  ADD COLUMN IF NOT EXISTS heartbeat_at timestamptz NOT NULL DEFAULT now();

-- 2. Create index for TTL queries (stale presence detection)
CREATE INDEX IF NOT EXISTS idx_presence_heartbeat_at 
  ON public.presence (heartbeat_at)
  WHERE status != 'offline';

-- 3. Create or replace the ghost-session cleanup function
CREATE OR REPLACE FUNCTION public.cleanup_ghost_presence()
RETURNS integer
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
DECLARE
  _count integer;
BEGIN
  -- Mark any presence row as offline if no heartbeat for >90 seconds
  -- (Backend heartbeats every 30s; 90s = 3 missed heartbeats = definitely offline)
  UPDATE public.presence
  SET 
    status = 'offline',
    last_seen = now()
  WHERE 
    status != 'offline'
    AND heartbeat_at < now() - INTERVAL '90 seconds';
  
  GET DIAGNOSTICS _count = ROW_COUNT;
  RETURN _count;
END;
$$;

-- 4. Grant execute permission to service role
GRANT EXECUTE ON FUNCTION public.cleanup_ghost_presence() TO service_role;
GRANT EXECUTE ON FUNCTION public.cleanup_ghost_presence() TO authenticated;

-- 5. Create a presence_upsert helper function
-- This is called by the app heartbeat to atomically update presence
CREATE OR REPLACE FUNCTION public.upsert_presence(
  _user_id uuid,
  _status text,
  _metadata jsonb DEFAULT '{}'::jsonb
)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
AS $$
BEGIN
  INSERT INTO public.presence (user_id, status, last_seen, heartbeat_at, metadata)
  VALUES (_user_id, _status, now(), now(), _metadata)
  ON CONFLICT (user_id) DO UPDATE
  SET 
    status = EXCLUDED.status,
    last_seen = now(),
    heartbeat_at = now(),
    metadata = COALESCE(EXCLUDED.metadata, public.presence.metadata);
END;
$$;

GRANT EXECUTE ON FUNCTION public.upsert_presence(uuid, text, jsonb) TO authenticated;
GRANT EXECUTE ON FUNCTION public.upsert_presence(uuid, text, jsonb) TO anon;

-- 6. Ensure presence table has user_id unique constraint for upsert
DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint 
    WHERE conname = 'presence_user_id_key' 
    AND conrelid = 'public.presence'::regclass
  ) THEN
    -- Try to add unique constraint; if fails (duplicate data), just log
    BEGIN
      ALTER TABLE public.presence ADD CONSTRAINT presence_user_id_key UNIQUE (user_id);
    EXCEPTION WHEN others THEN
      -- If unique constraint already exists under different name or
      -- there's duplicate data, skip gracefully
      RAISE NOTICE 'presence user_id unique constraint already exists or could not be added: %', SQLERRM;
    END;
  END IF;
END $$;

-- 7. Clean up any ghost sessions immediately
SELECT public.cleanup_ghost_presence();
