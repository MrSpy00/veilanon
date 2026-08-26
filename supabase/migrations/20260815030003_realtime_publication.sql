-- ============================================================================
-- veilanon control-plane schema — part 4: realtime publication
-- Enables Postgres changes streaming for the tables the desktop client
-- subscribes to (ciphertext envelopes only — no plaintext is added to the
-- stream beyond what the tables already store).
--
-- NOTE: 20260815030002_friendships.sql already added messages/presence/
-- friendships to the publication, and 20260815030004_dm_crypto.sql adds
-- channel_members. PostgreSQL 15 has no `ADD TABLE IF NOT EXISTS`, so this
-- migration is written idempotently: tables that are already members are
-- skipped. This keeps `supabase db push` working on both fresh databases
-- and databases that applied the earlier migrations.
-- ============================================================================

do $$
begin
  if not exists (
    select 1 from pg_publication_tables
    where pubname = 'supabase_realtime' and schemaname = 'public' and tablename = 'messages'
  ) then
    alter publication supabase_realtime add table messages;
  end if;

  if not exists (
    select 1 from pg_publication_tables
    where pubname = 'supabase_realtime' and schemaname = 'public' and tablename = 'presence'
  ) then
    alter publication supabase_realtime add table presence;
  end if;

  if not exists (
    select 1 from pg_publication_tables
    where pubname = 'supabase_realtime' and schemaname = 'public' and tablename = 'friendships'
  ) then
    alter publication supabase_realtime add table friendships;
  end if;

  if not exists (
    select 1 from pg_publication_tables
    where pubname = 'supabase_realtime' and schemaname = 'public' and tablename = 'channel_members'
  ) then
    alter publication supabase_realtime add table channel_members;
  end if;
end $$;
