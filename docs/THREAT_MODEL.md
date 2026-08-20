# veilanon — Tehdit Modeli

Bu belge, sistemin neyi koruduğunu, kimden koruduğunu ve korumaların nerede bittiğini dürüstçe tanımlar. "Sunucu hiçbir şey görmez" iddiasında bulunmuyoruz; **içerik** E2EE'dir, **metadata** değildir.

## 1. Varlıklar (Assets)

| Varlık | Nerede | Hassasiyet | Koruma |
| --- | --- | --- | --- |
| Mesaj içeriği | İstemci belleği, yerel SQLite, zarf olarak sunucu | Çok yüksek | AES-256-GCM (taşıma + beklemede) |
| Dosya içerikleri & adları | İstemci, R2 blob'ları | Çok yüksek | AES-256-GCM blob; ad hiç saklanmaz |
| Özel anahtarlar (X25519, Ed25519) | Yalnız istemci (OS keychain) | Kritik | Keychain + Argon2id türetme |
| Metadata (kim, hangi kanal, ne zaman, boyut) | Supabase | Orta | Saatlik kovalama, minimal profil |
| Kimlik (hesap) | Supabase Auth | Yüksek | Standart auth; 2FA önerilir |
| Davet kodları | Supabase | Orta | RLS üye-görünür; RPC ile önizleme |
| Roster / roller | Supabase | Düşük-Orta | Üyeye açık (bilinçli tasarım) |

## 2. Saldırgan tablosu

| # | Saldırgan | Yüzey | Etki | Azaltma | Kalıntı risk |
| --- | --- | --- | --- | --- | --- |
| A1 | Meraklı operatör (sunucuyu işleten) | DB dökümü, loglar, Edge Fn'ler | İçerik okuyamaz; metadata'yı okur | E2EE zarflar; log'lara içerik yazılmaz (deliver-message kuralı); RLS | Operatör metadata'yı satabilir/analiz edebilir (kabul edilen risk, PRIVACY_SCOPE) |
| A2 | Ağ dinleyicisi (MITM) | TLS yoksa, kafe Wi-Fi | İçerik + metadata | Her yerde TLS; zarf zaten E2EE | Metadata (paket boyutu/zamanlama) analizi mümkün; padding v2 planı |
| A3 | Hesap ele geçirme | Şifre sıfırlama, phishing, token hırsızlığı | Kurbanın görebildiği her şey; yeni cihaz ekleme | 2FA önerisi; cihaz doğrulama uyarısı; yeni cihaz güven yok sayımı | Cihaz listesi sunucu tarafında doğrulanır; ele geçirilen hesaba yeni cihaz eklendiyse geçmiş anahtarlar yoktur (forward secrecy yalnızca çift yönlü ratchet'te) |
| A4 | Kötü niyetli üye | Aynı mekândaki içerik | Kanal içeriğini sızdırır (meşru üyedir) | E2EE grup anahtarı rotasyonu (üye çıkışında yeniden anahtarlama); roller | Ekran görüntüsü/ekran fotoğrafı engellenemez — bilinçli insan sorunu |
| A5 | Kötü niyetli eklenti/bot | Bot API, webhook'lar | Bot izni ölçeğinde içerik | İmzalı manifest, kapsamlı yetki sistemi, kapsamı daraltılmış dönen tokenlar | Bot, E2EE kanallarda ciphertext görür (rıza yoksa); rıza akışı BOT_API'de |
| A6 | Tedarik zinciri | npm / crates.io bağımlılıkları | Uzaktan kod çalıştırma | Kilitli bağımlılıklar, CI'da cargo-audit + npm audit, gitleaks, SBOM yayını | Denetlenmemiş transitive bağımlılık riski standarttır; SBOM ile izlenir |
| A7 | Kesinti/veri kaybı (outage) | Sunucu, R2 | Hizmet durur; geçmiş kaybolabilir | Yerel-first mağaza (geçmiş yerelde yaşar), çevrimdışı kuyruk | Sunucu yedekleri operatör sorumluluğunda |
| A8 | Ağ gözlemcisi & Kötü niyetli çıkış düğümü (Tor/Proxy) | Yerel ISP, sansür duvarı, kötü niyetli Tor çıkış düğümü | IP tespiti, DNS sızıntısı, bağlantı engelleme | SOCKS5h ile Tor tarafında DNS çözümleme, TLS v1.3 zorunluluğu, E2EE zarflar, Fail-Closed (kesin mod) | Çıkış düğümü hedefin Supabase olduğunu görebilir; içerik ve anahtarları asla çözemez |

## 3. Korumalar (mapping)

- **İçerik gizliliği:** A1, A2 → E2EE (AES-256-GCM, X25519, Argon2id). Sunucuda düz metin tutan tek satır kod bile ret nedenidir.
- **Kimlik doğrulama:** A3 → Supabase Auth + 2FA önerisi + cihaz güven modeli (yeni cihaz = yeni anahtar çifti).
- **Yetkilendirme:** A3, A4, A5 → RLS politikaları (tüm tablolar), üyelik kapıları Edge Fn'lerde yeniden doğrulanır (çift kapı ilkesi).
- **Bütünlük:** A6 → bağımlılık kilitleri, imzalı manifest, webhook HMAC imzaları.
- **Süreklilik:** A7 → yerel-first; sunucu yalnızca röledir.

## 4. Kabul edilen (bilinçli) riskler

1. **Metadata görünürlüğü:** Sunucu; kullanıcı→kanal→zaman→boyut grafını bilir. Azaltma: saat kovalama, padding planı. Bu, merkezi röle mimarisinin bedelidir.
2. **Grup anahtarı yönetimi:** Mevcut aşamada kanal anahtarları üye çıkışında rotasyon bekler; MLS (openmls) aday olarak `CRYPTO_DECISIONS.md` AD-5'te değerlendirilmiştir.
3. **Direct üyelik ekleme (R-1):** `memberships` tablosundaki `insert own` politikası spec gereğidir; davet doğrulaması `accept_invite()` RPC'sindedir. İstemciler bu RPC'yi kullanmalıdır; kötüye kullanım yalnızca kendi kullanıcısını mekâna ekler (içerik yine E2EE'dir, yani etki: davetsiz "üyelik" metadata'sı).
4. **Ekran görüntüsü:** Herhangi bir E2EE sistemde olduğu gibi, görüntüleyen sızdırabilir.
5. **Silinen hesap:** `on delete cascade` ile zarf satırları silinir; blob temizliği operatör politikasıdır.

## 5. Doğrulama & bakım

- CI: `security-audit.yml` her push/PR'da cargo-audit + npm audit + gitleaks çalıştırır.
- SBOM: her sürümde `anchore/sbom-action` SPDX çıktısı yayınlanır.
- Tehdit modeli her büyük mimari değişiklikte güncellenir; bu dosya tek doğruluk kaynağıdır.

## 6. Bileşen bazında STRIDE taraması

| Bileşen | S | T | R | I | D | E | Not |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Edge Fn (deliver-message) | zarf sahteciliği | — | üyelik kapısı | zarf bütünlüğü (GCM tag) | boyut limiti ile DoS sınırı | token yetki yükseltme yok (claim yalnız `sub`) | — |
| Postgres | operatör okuması | — | RLS | FK cascade | sorgu limitleri | — | — |
| R2 | blob okuma | — | imzalı kısa ömürlü URL | istemci AEAD tag'ı | boyut kotası | — | — |
| LiveKit SFU | medya dinleme (Faz 1) | — | token HS256 | DTLS-SRTP | oda kotası | — | Faz 2: E2EE |
| Yerel SQLite | disk kopyası | — | — | AEAD | — | — | anahtar keychain'de |
| Tauri IPC | yetkisiz komut | — | capabilities allowlist | — | — | — | — |
| Bot webhook | sahte olay | — | — | Ed25519 imza | — | — | BOT_API |
| npm/crates | kötü paket | — | — | — | — | — | SBOM + audit CI |

## 7. Senaryo yürüyüşleri

**S1 — DB dökümü sızarsa:** Saldırgan (A1) tüm `messages` satırlarını alır; yalnızca AES-256-GCM zarfları görür. Anahtarlar sunucuda olmadığından içerik açılamaz. Kayıp: metadata grafı. Yanıt: kullanıcıları bilgilendir, dökümün metadata içerdiğini açıkça söyle.

**S2 — Edge Fn kodu değiştirilirse:** Saldırgan `deliver-message`'i değiştirip düz metin loglamaya çalışabilir. Ama istemci yalnızca zarf gönderir — fonksiyonun görebileceği düz metin yoktur. Kötü niyetli fonksiyon en fazla zarf kaybına veya metadata toplamaya yol açar; bu nedenle edge dağıtımına erişim operatörün en kritik yüzeyidir (A1).

**S3 — Sahte cihaz kaydı:** Ele geçirilen hesap (A3) yeni cihaz ekler ve grup anahtarını ister. Azaltma: grup anahtarı yeni cihaza yalnızca mevcut bir cihazın imzalı "cihaz doğrulama" paketiyle verilir (device-2-device verify). Kalıntı: doğrulama mekanizması v1'de "diğer cihazdan onay" istemi olarak başlar, otomatik değildir.

**S4 — Tedarik zinciri:** Kritik crate (örn. aes-gcm) ele geçirilir ve Cargo.lock sabitlenmiş sürüme kötü kod girmez — kilit, sürüm sabitlemesi sağlar; yeni sürüme geçiş yalnızca audit + SBOM diff ile olur (A6).

## 8. Yanıt planı (özet)

1. **İçerik sızıntısı şüphesi:** önce `PRIVACY_SCOPE` matrisiyle karşılaştır; içerik sızıntısı mimari olarak yalnızca istemci tarafından olabilir → sürüm denetimi + istemci düzeltmesi.
2. **Metadata sızıntısı:** döküm kapsamını belirle, etkilenen kullanıcıları saatlik kova hassasiyetini açıklayarak bilgilendir.
3. **Bot/adaptör ihlali:** token iptali + anahtar rotasyonu + audit kontrolü.
4. Her olay `audit_events`'e yazılır ve postmortem `docs/` altında yayınlanır.

## 9. İletişim

Güvenlik bulguları için: önce özel kanal (repo güvenlik sekmesi / aegisSoft iletişim), kamuya açık issue **açılmadan**. Koordineli ifşa (coordinated disclosure) politikası: 90 gün.

## 10. Kapsam notları

- Bu model v1 masaüstü istemcisini ve kontrol düzlemini kapsar; mobil istemci çıkarsa model ayrıca güncellenir.
- Self-host senaryosunda A1 saldırganı operatörün kendisidir; aynı tablo geçerlidir (içerik yine kapalıdır).
- Fiziksel cihaz hırsızlığı (evil maid) kapsam dışıdır; varsayım: OS keychain + disk şifreleme kullanıcı sorumluluğu.

## 11. Güvenlik değişmezleri (invariants — kod inceleme kontrol listesi)

1. Sunucuya giden her bayt ya AEAD şifrelidir ya da açıkça listelenmiş metadata'dır (PRIVACY_SCOPE).
2. `messages` ve `files` tablolarında düz metin sütunu yoktur (yayın engelleyici kural).
3. Edge fonksiyonları gövdeyi loglamaz (deliver-message sözleşmesi).
4. Özel anahtar malzemesi WebView/JS'e geçmez (IPC allowlist).
5. Tüm tablolar RLS açıktır; yeni tablo = zorunlu RLS + bu dosyada satır.
6. Bağımlılık değişikliği audit + SBOM diff olmadan merge edilmez.

Her PR'da bu 6 madde otomatik olmayan (insan) inceleme kontrol listesidir; `security-audit.yml` CI ise otomatik katmandır.

## 12. Model sürümü

| Sürüm | Tarih | Değişiklik |
| --- | --- | --- |
| 1.0 | 2026-08 | İlk model; v1 kapsamı, 7 saldırgan, STRIDE taraması |

Güncellemeler yukarıdaki tabloya satır ekler; eski sürümler silinmez.
