-- ============================================================================
-- veilanon Custom Status, Bio, Banner & Extended Profile Sync
-- Migration: 20260819050000_custom_status_and_profile_sync.sql
-- ============================================================================

-- 1. Users table extension
alter table public.users add column if not exists bio text default '';
alter table public.users add column if not exists custom_status text default '';
alter table public.users add column if not exists banner_hash text default '';
alter table public.users add column if not exists dm_privacy text default 'everyone';

-- 2. Presence table extension
alter table public.presence add column if not exists custom_status text default '';
alter table public.presence add column if not exists dm_privacy text default 'everyone';

-- 3. Ensure index for presence querying
create index if not exists idx_presence_custom_status on public.presence(custom_status);

-- 4. Add to Realtime publication
do $$
begin
    alter publication supabase_realtime add table public.users;
exception when others then null;
end $$;

do $$
begin
    alter publication supabase_realtime add table public.presence;
exception when others then null;
end $$;
