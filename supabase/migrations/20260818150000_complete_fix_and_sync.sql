-- ============================================================================
-- veilanon Supabase Comprehensive Fix, Schema Sync & Realtime Configuration
-- Migration: 20260818150000_complete_fix_and_sync.sql
-- ============================================================================

-- 1. spaces tablosuna eksik sütunları ekle (Public Discovery & Custom Links)
alter table public.spaces add column if not exists description text default '';
alter table public.spaces add column if not exists banner_hash text default '';
alter table public.spaces add column if not exists custom_link text default '';

create index if not exists idx_spaces_custom_link on public.spaces(custom_link);
create index if not exists idx_spaces_owner_id on public.spaces(owner_id);

-- 2. roles ve role_members tabloları için tam RLS ve Realtime desteği
create table if not exists public.roles (
    id uuid primary key default gen_random_uuid(),
    space_id uuid not null references public.spaces(id) on delete cascade,
    name text not null,
    color text default '',
    position integer not null default 0,
    permissions bigint not null default 0,
    created_at timestamp with time zone default now()
);

create table if not exists public.role_members (
    role_id uuid not null references public.roles(id) on delete cascade,
    user_id uuid not null references public.users(id) on delete cascade,
    created_at timestamp with time zone default now(),
    primary key (role_id, user_id)
);

create index if not exists idx_roles_space_id on public.roles(space_id);
create index if not exists idx_role_members_user_id on public.role_members(user_id);

-- 3. RLS'i etkinleştir
alter table public.roles enable row level security;
alter table public.role_members enable row level security;
alter table public.spaces enable row level security;
alter table public.channels enable row level security;
alter table public.channel_members enable row level security;
alter table public.friendships enable row level security;
alter table public.messages enable row level security;

-- 4. Temel Anon / Authenticated Okuma/Yazma Politikaları (Kontrol Düzlemi Eşzamanlama)
do $$
begin
    -- spaces
    if not exists (select 1 from pg_policies where tablename = 'spaces' and policyname = 'spaces_public_select') then
        create policy spaces_public_select on public.spaces for select using (true);
    end if;
    if not exists (select 1 from pg_policies where tablename = 'spaces' and policyname = 'spaces_anon_all') then
        create policy spaces_anon_all on public.spaces for all using (true) with check (true);
    end if;

    -- roles
    if not exists (select 1 from pg_policies where tablename = 'roles' and policyname = 'roles_all_access') then
        create policy roles_all_access on public.roles for all using (true) with check (true);
    end if;

    -- role_members
    if not exists (select 1 from pg_policies where tablename = 'role_members' and policyname = 'role_members_all_access') then
        create policy role_members_all_access on public.role_members for all using (true) with check (true);
    end if;

    -- friendships
    if not exists (select 1 from pg_policies where tablename = 'friendships' and policyname = 'friendships_all_access') then
        create policy friendships_all_access on public.friendships for all using (true) with check (true);
    end if;

    -- channel_members
    if not exists (select 1 from pg_policies where tablename = 'channel_members' and policyname = 'channel_members_all_access') then
        create policy channel_members_all_access on public.channel_members for all using (true) with check (true);
    end if;

    -- channels
    if not exists (select 1 from pg_policies where tablename = 'channels' and policyname = 'channels_all_access') then
        create policy channels_all_access on public.channels for all using (true) with check (true);
    end if;
end $$;

-- 5. Supabase Realtime yayınına tüm tabloları ekle
do $$
begin
    alter publication supabase_realtime add table public.messages;
exception when others then null;
end $$;

do $$
begin
    alter publication supabase_realtime add table public.friendships;
exception when others then null;
end $$;

do $$
begin
    alter publication supabase_realtime add table public.channels;
exception when others then null;
end $$;

do $$
begin
    alter publication supabase_realtime add table public.channel_members;
exception when others then null;
end $$;

do $$
begin
    alter publication supabase_realtime add table public.spaces;
exception when others then null;
end $$;

do $$
begin
    alter publication supabase_realtime add table public.memberships;
exception when others then null;
end $$;

do $$
begin
    alter publication supabase_realtime add table public.roles;
exception when others then null;
end $$;

do $$
begin
    alter publication supabase_realtime add table public.role_members;
exception when others then null;
end $$;
