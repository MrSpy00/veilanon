-- ============================================================================
-- veilanon test verisi temizleme komut dosyası (Clean Test Data)
-- ----------------------------------------------------------------------------
-- Bu script şemayı, tabloları, RLS politikalarını ve tetikleyicileri KORUR;
-- yalnızca şu ana kadar oluşan test kayıtlarını (arkadaşlıklar, mesajlar,
-- kanallar, topluluklar, davetler, cihazlar) temizler.
-- ============================================================================

-- Bağımlı kayıtları sırayla temizle (Foreign Key güvenli)
truncate table messages cascade;
truncate table channel_members cascade;
truncate table friendships cascade;
truncate table files cascade;
truncate table role_members cascade;
truncate table roles cascade;
truncate table invites cascade;
truncate table channels cascade;
truncate table memberships cascade;
truncate table spaces cascade;
truncate table devices cascade;

-- İsteğe bağlı: test kullanıcılarını da sıfırlamak isterseniz yorumu kaldırabilirsiniz:
-- truncate table users cascade;
-- delete from auth.users;
