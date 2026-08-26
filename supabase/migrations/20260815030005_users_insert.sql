-- ============================================================================
-- veilanon control-plane schema — part 5: users self-insert policy
-- ----------------------------------------------------------------------------
-- The client mirrors its profile via `upsert` (INSERT ... ON CONFLICT DO
-- UPDATE). `users` only had SELECT/UPDATE policies, so the INSERT half of the
-- upsert was rejected by RLS (42501). Allow the own row only.
-- ============================================================================

create policy users_insert_own on users
  for insert with check (id = auth.uid());
