-- Oturum listesi: kullanıcı yalnızca kendi cihaz kayıtlarını okuyabilir.
-- (Uygulama oturumlar/cihazlar ekranında devices tablosunu sorgular.)
create policy "read own devices"
  on public.devices for select
  using (user_id = auth.uid());
