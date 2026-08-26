# veilanon — Supabase kurulum rehberi

Bu rehber, control plane'i (kimlik, alan/kanal/rol metadata'sı, şifreli
zarf yönlendirme) canlı Supabase projenize bağlamak için gereken her şeyi
adım adım anlatır.

> Gizlilik ilkesi: Sunucu **asla** düz metin mesaj içeriği, dosya adı veya
> arama metni saklamaz. `messages` tablosu yalnızca istemci tarafında
> AES-256-GCM ile şifrelenmiş zarf (ciphertext + iv) tutar. Bu, şema
> tasarımıyla zorlanır — politikayla değil.

---

## Durum kontrolü (ne yapıldı, ne kaldı)

| Bileşen | Durum |
|---|---|
| `.env` anahtarları (anon + service role JWT) | ✅ Doğrulandı — ikisi de geçerli ve yetkili |
| `supabase/migrations/` (20 migration dosyası) | ✅ **Uygulandı & Doğrulandı** (17 tablo, 80 RLS politikası, 57 RPC, 11 Realtime yayını) |
| `deliver-message` Edge Function | ✅ **Deploy edildi** + auth koruması doğrulandı |
| `livekit-token` Edge Function | ✅ **Deploy edildi** + auth koruması doğrulandı |
| Edge Function secret'ları (LIVEKIT_*) | ✅ `supabase secrets set` ile eklendi |

Canlı doğrulama sonuçları: 17 tablo mevcut, tüm RLS politikaları ve Realtime yayınları aktif, anon INSERT sınırları korunur, fonksiyonlar auth'suz çağrıda `401 UNAUTHORIZED_NO_AUTH_HEADER` döndürür.

> **2026-08-15 ikinci doğrulama turu:** anonim sign-in uçtan uca test edildi
> (GoTrue `signup` → JWT → RLS ile kendi `users` satırı okunabildi →
> test kullanıcısı temizlendi). `deliver-message` ve `livekit-token`
> fonksiyonları canlıda, ikisi de auth'suz çağrıda 401 dönüyor.
>
> **Migration düzeltmesi:** `20260815030002_friendships.sql` ve
> `20260815030003_realtime_publication.sql` aynı tabloları
> `supabase_realtime` yayınına ekliyordu; PG15'te ikinci ekleme
> "already member of publication" hatası verip `db push`'ı durdurabiliyor.
> `03` artık idempotent (`pg_publication_tables` kontrolü) — taze kurulumda
> ve mevcut veritabanında hatasız çalışır. Mevcut canlı projede yayın
> üyeliği zaten eksiksiz olduğundan ek bir işlem gerekmez.
>
> **2026-08-16 senkron düzeltmesi (canlıda doğrulandı):**
> - `best_effort_insert` ve mesaj yükleme artık JSON `null` taşımıyor
>   (Supabase NOT NULL sütunları 400 döndürüyordu) ve plain insert
>   kullanıyor (PostgREST `on_conflict=id` + RLS, `id ≠ auth.uid()`
>   satırlarında 403 veriyordu). Space → membership → channel → message
>   zinciri uçtan uca cloud'a kaydediliyor.
> - Anonim oturum **refresh token'ı kalıcılaştırıldı** (local DB, migration
>   0007): her kilit açmada yeni Supabase kullanıcısı oluşmuyor; aynı
>   kullanıcıya dönülüyor (MAU şişmesi + orphan satırlar çözüldü).
> - `users`/`devices` tablosu silinen anonim kullanıcıların satırlarını
>   cascade ile temizlemez — Supabase Auth kullanıcısı silindiğinde
>   `public.users` satırı da ayrıca silinmelidir.

> Migration iki dosyaya bölünmüştür: `20260815030000_initial.sql` (şema +
> tablolar) ve `20260815030001_functions.sql` (fonksiyonlar + RLS policy'leri).
> Nedeni: SQL fonksiyonları oluşturma anında tablo varlığını doğrular,
> policy'ler de fonksiyonu gerektirir — sıralama bu bağımlılığı çözer.

---


---

## Adım 0 - Anonymous oturum açmayı etkinleştir (ZORUNLU)

Uygulama kontrol düzlemine **anonim oturum** ile bağlanır (sign_in_anonymous);
bu olmadan kullanıcı/kimlik kaydı RLS nedeniyle reddedilir ve uygulama
yalnızca local-first modda çalışır (çok kullanıcılı senkron olmaz).

1. Supabase Dashboard → **Authentication → Sign In / Up**.
2. **Anonymous sign-ins** bölümünü **Enable** yap.
3. Uygulama logunda şunu görünce adım tamamdır:

`
INFO veilanon ... Realtime connected
INFO veilanon ... Realtime channel joined: realtime:postgres_changes
INFO veilanon ... Control plane bound
`

Control plane bind skipped görüyorsan bu adım atlanmış demektir.

> **Captcha hakkında (önemli):** Anonymous sign-in **açık kalmalıdır** — uygulama
> her kimlik oluşturma/bağlanmada anonim oturum açar (desktop istemcisi
> hCaptcha çözümü yapmaz). Panel "Enable captcha" öneriyor ancak **captcha'yı
> zorunlu yapmayın**: GoTrue captcha token'ı isterse desktop istemcisinin
> `signup` çağrısı (`gotrue_meta_security: {}`) reddedilir ve kimlik
> oluşturma kırılır. Kötüye kullanım koruması zaten uygulamada mevcuttur:
> hatalı parola denemeleri için 5-deneme kilidi + Argon2id maliyeti, RLS ile
> veri izolasyonu. MAU maliyeti konusunda: her kimlik bir anonim kullanıcı
> üretir; MAU hedefin üzerine çıkarsa eski/anlamsız anonim hesapları
> Dashboard → Authentication → Users'tan temizleyebilirsin.

> Auth → URL Configuration'da Site URL ve Redirect URLs alanlarının dolu
> olduğundan da emin olun (ör. http://localhost:1420).
## Adım 1 — Veritabanı şemasını uygula

### Seçenek A: SQL Editor (önerilen — hızlı)

1. Supabase Dashboard → projen (`<your-project-ref>`) → **SQL Editor**.
2. `supabase/migrations/20260815030000_initial.sql` içeriğini kopyala-yapıştır → **Run**.
3. `supabase/migrations/20260815030001_functions.sql` içeriğini kopyala-yapıştır → **Run**.

### Seçenek B: Supabase CLI (`db push`)

```bash
supabase init                        # supabase/config.toml üretir (varsa atlanır)
supabase link --project-ref <your-project-ref>
supabase db push                     # DB parolasını sorar (dashboard > Settings > Database)
```

> Not: `supabase init` proje **kökünde** çalıştırılmalıdır (supabase/ içinde
> değil) — aksi halde iç içe bir supabase/supabase klasörü oluşur.

### Doğrulama (CLI sonrası / SQL Editor sonrası)

```bash
# Service role anahtarıyla tablo görünür olmalı (sunucu-taraflı User-Agent ile):
curl -s -H "apikey: $VEILANON_SUPABASE_SERVICE_ROLE_KEY" \
     -H "Authorization: Bearer $VEILANON_SUPABASE_SERVICE_ROLE_KEY" \
     "https://<your-project-ref>.supabase.co/rest/v1/users?select=count"
# Anon anahtarla aynı sorgu RLS nedeniyle BOŞ dönmelidir (güvenlik kanıtı):
curl -s -H "apikey: $VEILANON_SUPABASE_ANON_KEY" \
     -H "Authorization: Bearer $VEILANON_SUPABASE_ANON_KEY" \
     "https://<your-project-ref>.supabase.co/rest/v1/users?select=count"
```

---

## Adım 2 — Edge Function ortam değişkenleri

Dashboard → **Edge Functions** → sağ üst **Manage Secrets** → aşağıdakileri ekle:

| Anahtar | Değer |
|---|---|
| `SUPABASE_URL` | `https://<your-project-ref>.supabase.co` |
| `SUPABASE_ANON_KEY` | `.env` içindeki `VEILANON_SUPABASE_ANON_KEY` |
| `LIVEKIT_API_KEY` | `.env` içindeki `VEILANON_LIVEKIT_API_KEY` |
| `LIVEKIT_API_SECRET` | `.env` içindeki `VEILANON_LIVEKIT_API_SECRET` |
| `LIVEKIT_URL` | `.env` içindeki `VEILANON_LIVEKIT_URL` |

> Sırlar asla depoya yazılmaz; yalnızca Supabase platformunda saklanır.

---

## Adım 3 — Edge Function'ları deploy et

```bash
supabase functions deploy deliver-message
supabase functions deploy livekit-token
```

### Doğrulama

```bash
# Deploy edilmemişse 404, edilmişse 401/405 döner:
curl -s -o /dev/null -w "%{http_code}" -X POST \
     "https://<your-project-ref>.supabase.co/functions/v1/deliver-message"
```

---

## Güvenlik notları (önemli)

1. **Service role anahtarı tarayıcıdan kullanılamaz.** Supabase'in yeni
   koruması `sb_secret_*` anahtarını browser-context isteklerinde reddeder
   ("Forbidden use of secret API key in browser"). veilanon bu anahtarı
   **yalnızca Rust çekirdeğinde** (sunucu-taraflı, `Authorization: Bearer`
   + backend User-Agent) kullanır — bu korumayla uyumludur ve frontend'e
   asla sızmaz (doğrulandı: release binary'de ve frontend bundle'da hiçbir
   gerçek anahtar yok).
2. **RLS her sorguyu üyelikle filtreler.** `is_space_member()` SECURITY
   DEFINER + `search_path` sabitli; kullanıcı yalnızca üyesi olduğu alanın
   kanallarını ve mesajlarını görebilir.
3. **`messages` şemasında düz metin sütunu yoktur.** Content, filename ve
   search index sunucuda yoktur; şema düzeyinde imkânsızdır.
4. **Denetim günlüğü yalnızca service role'a açıktır.** `audit_events` içerik
   taşımaz, yalnızca olay tipi + hedef kimliği.
5. **Davet kabulü yalnızca `accept_invite()` ile yapılır** — atomik doğrulama
   + kullanım sayacı + üyelik ataması tek fonksiyonda.

---

## Yerel geliştirme vs canlı

| Mod | Şema | Fonksiyonlar | Not |
|---|---|---|---|
| Yerel (şu an) | gerekmez | gerekmez | Tüm veri cihazda, E2EE korumalı |
| Canlı | Adım 1 | Adım 3 | Çoklu cihaz, alanlar, presence |

İki mod arasında geçiş yapmak için yalnızca `.env` yeterlidir; uygulama
kodunda değişiklik gerekmez.
