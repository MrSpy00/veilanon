-- MLS grup E2EE: sahibin ürettiği Welcome zarfı (üyenin DH anahtarıyla şifreli).
-- Yalnızca envelope (ciphertext) saklanır — sunucu anahtar veya düz metin görmez.
create table if not exists public.mls_welcomes (
  channel_id uuid not null,
  user_id    uuid not null references public.users (id) on delete cascade,
  envelope   text not null,
  created_at timestamptz not null default now(),
  primary key (channel_id, user_id)
);

alter table public.mls_welcomes enable row level security;

-- Kullanıcı yalnızca kendi welcome'ını okuyabilir (user_id = auth.uid()).
create policy "read own welcomes"
  on public.mls_welcomes for select
  using (user_id = auth.uid());

-- Kanal sahibi welcome yazabilir (spaces sahipliği üzerinden).
create policy "owner inserts welcomes"
  on public.mls_welcomes for insert
  with check (
    exists (
      select 1
      from public.channels ch
      join public.spaces s on s.id = ch.space_id
      where ch.id = channel_id and s.owner_id = auth.uid()
    )
  );

-- Discord köprüsü webhook'ları (yalnızca meta veri, mesaj içeriği değil).
create table if not exists public.discord_webhooks (
  channel_id  uuid primary key references public.channels (id) on delete cascade,
  webhook_url text not null,
  created_at  timestamptz not null default now()
);

alter table public.discord_webhooks enable row level security;

create policy "owner manages webhooks"
  on public.discord_webhooks for all
  using (
    exists (
      select 1
      from public.channels ch
      join public.spaces s on s.id = ch.space_id
      where ch.id = channel_id and s.owner_id = auth.uid()
    )
  )
  with check (
    exists (
      select 1
      from public.channels ch
      join public.spaces s on s.id = ch.space_id
      where ch.id = channel_id and s.owner_id = auth.uid()
    )
  );
