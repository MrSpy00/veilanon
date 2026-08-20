# veilanon — Veri Envanteri (Data Inventory)

Sistemde saklanan **her** veri alanının nerede durduğu, şifre durumu, kimin erişebildiği, saklama süresi ve silinme davranışı. Şema: `supabase/migrations/0001_initial.sql`.

## 1. Sunucu tarafı (Supabase / PostgreSQL — kontrol düzlemi)

| Alan | Tablo | Şifre durumu | Erişim (RLS) | Saklama | Silinme |
| --- | --- | --- | --- | --- | --- |
| `id` | users | düz | kendisi + genel profil | hesap ömrü | hesap silinince cascade |
| `username` | users | düz (citext) | genel (minimal profil) | hesap ömrü | cascade |
| `display_name` | users | düz | genel | hesap ömrü | cascade |
| `avatar_hash` | users | düz (yalnızca hash) | genel | hesap ömrü | cascade |
| `created_at` | users | düz | yalnız kendisi | hesap ömrü | cascade |
| `last_seen_bucket` | users | düz (saatlik kova) | genel | hesap ömrü | cascade |
| `name` / `icon_hash` | spaces | düz | üyeler | mekân ömrü | cascade |
| `invite_code` | spaces/invites | düz | üyeler (RPC ile önizleme) | davet ömrü / döndürülebilir | cascade |
| `channel_type`, `position`, `permission_overrides`, `is_e2ee` | channels | düz | üyeler | kanal ömrü | cascade |
| `ciphertext` | messages | **E2EE (AES-256-GCM)** | üyeler (şifreli halde) | kanal ömrü veya `disappears_at` | üye silinince mesaj kalır; kullanıcı silinince `deleted_at` tombstone |
| `iv` | messages | düz (güvenlikli: GCM nonce) | üyeler | ciphertext ile | aynı |
| `schema_version` | messages | düz | üyeler | ciphertext ile | aynı |
| `client_created_at` / `server_received_at` | messages | düz | üyeler | ciphertext ile | aynı |
| `sender_device_id` | messages | düz (opak kimlik) | üyeler | ciphertext ile | aynı |
| `edited_at` / `deleted_at` / `disappears_at` | messages | düz | üyeler | ciphertext ile | aynı |
| `r2_key` | files | düz (UUID) | yükleyen cihaz sahibi | `expires_at` veya elle | satır silinir; blob temizliği ayrı politika |
| `size_bytes` | files | düz | sahibi | satır ömrü | aynı |
| `content_key_ciphertext` | files | **E2EE** (sarılmış dosya anahtarı) | sahibi | satır ömrü | aynı |
| `public_key` / `signing_public_key` | devices | düz (public) | kendi cihazları | cihaz ömrü | cascade |
| `permissions` / `position` / `is_default` | roles | düz | üyeler | rol ömrü | cascade |
| `role_id, user_id` | role_members | düz | üyeler | atama ömrü | cascade |
| `max_uses`, `used_count`, `expires_at` | invites | düz | üyeler | davet ömrü | cascade |
| `joined_at` | memberships | düz | kendisi (+ RPC roster) | üyelik ömrü | ayrılınca silinir |
| `status`, `last_seen_bucket` | presence | düz (saatlik) | tüm oturum açmışlar | canlı | offline'a düşer; hesap silinince cascade |
| `event_type`, `target_id` | audit_events | düz, **içerik alanı yok** | yalnız service_role | 90 gün (politika) | otomatik temizlik (planlı job) |

## 2. Bulut nesne deposu (R2)

| Nesne | İçerik | Şifre | Erişim | Saklama |
| --- | --- | --- | --- | --- |
| Blob (`r2_key`) | dosya içeriği | **AES-256-GCM (istemcide)** | yükleyen cihaz sahibi (indirme URL'si kısa ömürlü) | `files.expires_at` veya operatör politikası |
| Avatar (ops.) | görsel | public okuma | genel | profil ömrü |

## 3. Yerel (istemci)

| Veri | Konum | Şifre | Saklama | Silinme |
| --- | --- | --- | --- | --- |
| Mesaj geçmişi | `store.db` | **AES-256-GCM (sütun)** | kullanıcı ayarı (varsayılan: sınırsız yerel) | hesap çıkışında anahtar silinir → veri erişilemez; dosya opsiyonel imha |
| Taslaklar | `store.db` | AES-256-GCM | kullanıcı silene dek | aynı |
| Çevrimdışı kuyruk | `queue/` | AES-256-GCM | gönderilene dek | teslim sonrası silinir |
| Kimlik anahtarları | OS keychain | keychain şifrelemesi | cihaz ömrü | cihaz kaldırılınca |
| Oturum token'ı | keychain | keychain | token ömrü | çıkışta silinir |
| Önbellek (avatar vb.) | OS cache | — | LRU | otomatik |

## 4. Erişimciler (aktör matrisi)

| Aktör | Görebildiği |
| --- | --- |
| Üye (kendi) | kendi profili, üyesi olduğu mekânlar, şifreli zarflar, roster, kendi cihazları |
| Üye (başkası) | minimal profil (4 alan), saatlik varlık |
| Bot | manifest kapsamı kadar (BOT_API.md); E2EE kanalda rızasız ciphertext |
| Operatör (service_role) | tablo metadata'sı + ciphertext; düz metin yok |
| Anon | hiçbir şey (RLS) |

## 5. İlkeler

- **Minimizasyon:** Yukarıdaki listede olmayan alan eklenmez; eklenecekse bu belge aynı PR'da güncellenir.
- **İçerik-dışı kural:** Düz metin içerik veya dosya adı hiçbir tabloda, logda, metrikte yoktur (PRIVACY_SCOPE denetim noktası 1).
- **Silme davranışı:** FK `on delete` kuralları şemada sabittir; tombstone (`deleted_at`) yalnızca mesajlarda kullanılır, içerik üzerine yazılmaz.

## 6. Yerel SQLite şeması (şifreli sütunlar `*` ile)

| Tablo | Alanlar | Şifre | Not |
| --- | --- | --- | --- |
| `local_messages` | id, channel_id, `body*`, `iv*`, client_created_at, flags | AES-256-GCM | sunucu zarfının yerel düz metin karşılığı |
| `local_files` | id, path* (şifreli), r2_key, size, key_ciphertext | AES-256-GCM | yolun şifreli olması diskten ad sızmasını önler |
| `drafts` | channel_id, `body*`, updated_at | AES-256-GCM | taslak içerikleri de E2EE disiplininde |
| `outbox` | id, `envelope*`, retry_count, next_attempt_at | AES-256-GCM | çevrimdışı kuyruk; teslim sonrası silinir |
| `device_keys` | — | keychain referansı | ham anahtar SQLite'da asla durmaz |
| `space_cache` | space/channel/role kopyaları | düz | sunucudaki düz metadata'nın aynısı |

## 7. R2 yaşam döngüsü

- Yükleme: presigned URL yerine istemci doğrudan S3 uyumlu yükleme yapar (R2), `r2_key` sonradan `files` tablosuna kaydedilir.
- İndirme: kısa ömürlü (15 dk) imzalı URL; URL'de dosya adı yoktur.
- Silme: `files` satırı silinince blob operatörün zamanlanmış job'u ile temizlenir (varsayılan: 30 gün bekletme, geri dönüş penceresi).
- Avatar blob'ları ayrı önekte (`avatars/`) tutulur ve `avatar_hash` ile isimlendirilir — ad yok, yalnızca hash.

## 8. Yedekleme politikası

| Veri | Yedek | Şifre |
| --- | --- | --- |
| Postgres | operatör günlük yedeği (PITR önerilir) | yedek yalnızca zarf/metadata içerir |
| R2 | opsiyonel replikasyon | bloblar zaten şifreli |
| Yerel mağaza | kullanıcıya bırakılır (AD-13 anahtar yedek dosyası) | kullanıcı şifresiyle sarılı |

Yedekten geri dönüşte içerik kaybı olmaz; anahtar kaybında içerik kalıcı olarak kapanır (tasarlanmış davranış).

## 9. Alanlar ↔ belgeler eşleme

- Şema → `supabase/migrations/0001_initial.sql`
- Matris → `docs/PRIVACY_SCOPE.md` (bölüm 1)
- Anahtar yönetimi → `docs/CRYPTO_DECISIONS.md` (AD-11, AD-12, AD-13)
- Silme davranışı → FK kuralları bu dosya bölüm 1 + THREAT_MODEL bölüm 5

## 10. Denetim hilesi (self-check)

Her sürümde: `grep -ri "plaintext\|plain_text\|content" supabase/migrations/` komutu düz metin sütunu aramak için kullanılır; `messages` ve `files` tablolarında içerik alanı bulunması **yayın engelleyici** hatadır.
