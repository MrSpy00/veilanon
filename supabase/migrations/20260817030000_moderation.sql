-- Moderation: kick / ban / timeout (yalnızca meta veri — mesaj içeriği asla).
-- Ban listesi sahibin yönetiminde; üyeler kendi ban durumunu bile görmez.

create table if not exists public.bans (
  space_id   uuid not null references public.spaces (id) on delete cascade,
  user_id    uuid not null references public.users (id) on delete cascade,
  banned_by  uuid not null references public.users (id),
  reason     text,
  created_at timestamptz not null default now(),
  primary key (space_id, user_id)
);

alter table public.bans enable row level security;

create policy "owner manages bans"
  on public.bans for all
  using (
    exists (
      select 1 from public.spaces s
      where s.id = space_id and s.owner_id = auth.uid()
    )
  )
  with check (
    exists (
      select 1 from public.spaces s
      where s.id = space_id and s.owner_id = auth.uid()
    )
  );

-- Geçici susturma (süre dolana kadar mesaj gönderilemez).
alter table public.memberships add column if not exists timeout_until timestamptz;

-- Kick/ban için sahip üyelikleri yönetebilir (üye ekleme/çıkarma).
create policy "owner manages memberships"
  on public.memberships for all
  using (
    exists (
      select 1 from public.spaces s
      where s.id = space_id and s.owner_id = auth.uid()
    )
  )
  with check (
    exists (
      select 1 from public.spaces s
      where s.id = space_id and s.owner_id = auth.uid()
    )
  );
