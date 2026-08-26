-- ============================================================================
-- veilanon Migration: nullable avatar/banner on users
-- The app writes SQL NULL when a user clears their avatar/banner. The columns
-- were created NOT NULL DEFAULT '' in 20260823000000, which made those clears
-- fail the users upsert (check constraint) and left stale hashes in the DB.
-- Making them nullable lets "no avatar/banner" be expressed correctly.
-- ============================================================================

ALTER TABLE public.users ALTER COLUMN avatar_hash DROP NOT NULL;
ALTER TABLE public.users ALTER COLUMN banner_hash DROP NOT NULL;
