# veilanon — Mimari

Bu belge veilanon'un katmanlı mimarisini, güven sınırlarını ve bileşen değiştirilebilirliğini tanımlar. Hedef okuyucu: katkıda bulunacak geliştiriciler ve kendi sunucusunu çalıştıracak topluluk yöneticileri.

## 1. Katmanlı diyagram

```
┌─────────────────────────────────────────────────────────────────────┐
│  UI Katmanı (SvelteKit / Svelte 5)                                  │
│  Kanallar, roller, profil, ayarlar, köprü sihirbazı (yerel)         │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ Tauri IPC (invoke/emit) — yalnızca
                                │ serileştirilmiş komutlar; DOM'dan
                                │ anahtara erişim YOK
┌───────────────────────────────▼─────────────────────────────────────┐
│  Çekirdek (Rust)                                                     │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ │
│  │ Kripto       │ │ Yerel Mağaza │ │ Çevrimdışı   │ │ Adaptör      │ │
│  │ (AEAD, X25519│ │ (SQLite +    │ │ Kuyruk       │ │ Katmanı      │ │
│  │ Ed25519,     │ │ AES-256-GCM) │ │ (çıktı sırası│ │ (trait'ler)  │ │
│  │ Argon2id)    │ │              │ │ korunur)     │ │              │ │
│  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘ │
└───────┬───────────────────┬───────────────────┬──────────────────────┘
        │ şifreli zarf      │ şifreli blob      │ sinyalleme
┌───────▼─────────┐ ┌───────▼─────────┐ ┌───────▼──────────────────────┐
│ VERİ DÜZLEMİ    │ │ MEDYA DÜZLEMİ   │ │ KONTROL DÜZLEMİ               │
│ Supabase        │ │ R2 + Upstash    │ │ Supabase Auth + Edge Fn'ler   │
│ (Postgres,      │ │ (blob + varlık) │ │ (deliver-message,             │
│  yalnızca       │ │                 │ │  livekit-token) + LiveKit     │
│  şifreli metin) │ │                 │ │                               │
└─────────────────┘ └─────────────────┘ └───────────────────────────────┘
```

## 2. Düzlemler ve güven sınırları

### Kontrol düzlemi (Control plane)
- **Ne yapar:** Auth (Supabase Auth), üyelik doğrulama, davetler, varlık (presence) kovası, LiveKit token düzenleme, `deliver-message` ile şifreli zarf alımı.
- **Güven varsayımı:** Düzlem **güvenilmezdir**. Yalnızca şifreli zarfları ve metadata'yı görür. Düzlemi işleten operatör (aegisSoft veya self-host eden topluluk) içerik düzeyinde **hiçbir şey** okuyamaz.
- **Sınır:** İstemci → düzlem geçişi daima AES-256-GCM zarfıdır; düzlem → istemci geçişi daima imzalı (Ed25519) metadata'dır.

### Veri düzlemi (Data plane)
- Mesaj gövdeleri (ciphertext) Postgres'te; dosya blob'ları R2'de (istemci tarafında şifreli) tutulur. R2 anahtarları anlamsız UUID'lerdir — dosya adı asla saklanmaz.

### Medya düzlemi (Media plane)
- Ses/görüntü/ekran paylaşımı LiveKit SFU üzerinden akar. Tokenlar `livekit-token` Edge fonksiyonu ile HS256 imzalanır ve 6 saat geçerlidir; token alabilmek için mekân üyeliği şarttır. LiveKit E2EE anahtar yönetimi için bkz. `CRYPTO_DECISIONS.md`.

## 3. Güven sınırları (trust boundaries) — özet tablo

| Sınır | Geçen veri | Şifreleme | Kim doğrular? |
| --- | --- | --- | --- |
| UI (WebView) → Rust çekirdek | komutlar, olaylar | süreç içi IPC | Tauri `invoke` allowlist'i (capabilities) |
| Rust → yerel SQLite | düz metin mesajlar | AES-256-GCM (uygulama katmanı) | anahtar yalnız Rust'ta |
| Rust → Supabase | zarf (ciphertext+iv) | AES-256-GCM | — (sunucu çözemez) |
| Rust → R2 | şifreli blob | AES-256-GCM | — (sunucu çözemez) |
| Rust → LiveKit | medya akışı | DTLS-SRTP (standart) | LiveKit tokeni |
| Edge Fn → Postgres | JWT claim'leri | — | RLS politikaları |

## 4. IPC yüzeyi

Tauri 2 capabilities dosyası (dokunulmaz kural gereği sınırlı) yalnızca şu komutları açığa çıkarır:

- `crypto_*`: anahtar türetme (Argon2id), şifrele/çöz, imzala/doğrula — ham anahtar **asla** JS'e dönmez.
- `store_*`: yerel mağazaya okuma/yazma (şifreleme Rust tarafında saydam).
- `queue_*`: çevrimdışı kuyruk yönetimi.
- `sync_*`: zarf indirme/teslim (adaptör üzerinden).
- `bridge_*`: Discord köprüsü (yalnızca OAuth2 oturumu).

WebView içinde çalışan JS'in kriptografik anahtar erişimi yoktur; XSS riski "anahtar çalınamaz" seviyesine indirilir (içerik yine de okunabilir, bkz. THREAT_MODEL).

## 5. Bileşen değiştirilebilirliği

Tüm dış servisler Rust tarafında trait arkasındadır. Her adaptörün ücretsiz katman (free tier) eşleniği vardır:

| Bileşen | Varsayılan | Değiştirme yolu | Ücretsiz katman eşleniği |
| --- | --- | --- | --- |
| Control plane | Supabase | `ControlPlane` trait | Supabase free tier; yerel yalnız mod (degrade) |
| Postgres | Supabase DB | aynı trait üzerinden self-host Postgres 15 | — (Supabase free) |
| Auth | Supabase Auth | OIDC adaptörü (Keycloak vb.) | Supabase free (50k MAU) |
| Blob | Cloudflare R2 | `BlobStore` trait (S3 uyumlu) | R2 free (10 GB) |
| Realtime/varlık | Supabase Realtime | Upstash Redis pub/sub | Upstash free |
| Medya | LiveKit Cloud | self-host LiveKit OSS | LiveKit free (50 GB) |
| Hata izleme | Sentry | kapalı / kendi istatistik | Sentry free |
| Vektör (ops.) | Qdrant Cloud | self-host Qdrant | Qdrant free |

Adaptör değiştirmek `src-tauri/src/adapters/` içindeki trait implementasyonunu değiştirmektir; üst katmanlar etkilenmez. Bu belgeye göre **infra yalnızca adaptörlerdir**, çekirdek koddan bağımsız yaşar.

## 6. Yerel düzen (on-disk)

```
~/.local/share/veilanon/   (Windows: %APPDATA%\veilanon)
├── store.db            ← SQLite (şifreli sütunlar AES-256-GCM)
├── keys/               ← yerel kimlik (X25519 + Ed25519, OS keychain'de)
└── queue/              ← çevrimdışı kuyruk (şifreli zarflar)
```

Yerel anahtar OS keychain'de saklanır; SQLite anahtarı keychain'den türetilir. Disk şifreleme alternatifi olarak SQLCipher drop-in seçeneği `CRYPTO_DECISIONS.md` AD-6'da belgelenmiştir.

## 7. Başlatma sırası (boot sequence)

1. Rust çekirdek başlar; keychain'de kimlik arar → yoksa üretir (X25519 + Ed25519).
2. Yerel mağaza açılır; anahtar Argon2id ile türetilir (AD-2), sütunlar çözülmeye hazır hale gelir.
3. Adaptör katmanı `.env`'den yapılandırmayı okur; Supabase yoksa **yerel-only** moda düşer (ağ kapalı uyarısı).
4. Kontrol düzlemi erişilebilirse: auth restore, cihaz kaydı (varsa), mekân/kanal/rol çekimi.
5. Çevrimdışı kuyruk boşaltılır — sıra korunarak (`queue_*`).

## 8. Mesaj akışı (mutlu yol)

```
A cihazı                              Sunucu                         B cihazı
─────────                            ───────                        ─────────
1. AES-256-GCM ile şifrele
2. deliver-message Edge Fn ────────► 3. RLS: üyelik kontrolü
                                     4. messages tablosuna zarf
5. Realtime olayı (yalnız id) ◄────  6. olay fanout (id + kanal)
7. zarfı çek, yerelde çöz, doğrula
```

Sunucu 2–6 arasında yalnızca zarfı taşır; çözme yalnızca 1 ve 7'dedir.

## 9. Hata ve kesinti felsefesi

- Sunucu hataları istemcide **asla** düz metni açığa çıkarmaz; hata mesajları yalnızca id/kanal referansı taşır.
- Kesinti: kuyrukta birikir; önce-sonra sırası istemci saatine göre korunur (`client_created_at`).
- Yerel mağaza her zaman yazılabilir; ağ yalnızca "senkron fırsatı"dır (yerel-first ilkesi).

## 10. Telemetri sınırları

- Hata raporları (Sentry) opt-in'dir; içerik alanları gönderim öncesi strip edilir.
- Anonim istatistik yoktur; PRIVACY_SCOPE matrisindeki metadata dışında sunucuya bir şey gitmez.

## 11. Adaptör sürümleme

Adaptör trait'leri semver taşır; kırıcı değişiklik yeni trait sürümü + geçiş sürümü demektir. `.env` anahtarları `VEILANON_` önekiyle sürümlenir; bilinmeyen anahtar uyarı üretir, çökmez.
