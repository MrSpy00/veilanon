-- ============================================================================
-- veilanon Migration: users table profile columns
-- The app upserts banner_hash / bio / custom_status into public.users, but
-- these columns were never created, so every users upsert failed with
-- PostgREST 400. This silently broke:
--   1) banner/avatar persistence to the database (banner vanished on restart)
--   2) profile updates syncing to the control plane
--   3) the signup registration upsert in bind_control_plane
-- ============================================================================

ALTER TABLE public.users ADD COLUMN IF NOT EXISTS banner_hash text NOT NULL DEFAULT '';
ALTER TABLE public.users ADD COLUMN IF NOT EXISTS bio text;
ALTER TABLE public.users ADD COLUMN IF NOT EXISTS custom_status text;
