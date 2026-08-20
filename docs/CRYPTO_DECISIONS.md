# veilanon — Kripto Karar Günlüğü (Crypto Decision Log)

Her karar tek paragrafta: ne seçildi, neden, alternatifler, değişim koşulu. Kural listesi alttadır.

## AD-1: Yalnızca denetlenmiş ilkeller kuralı (KABUL)

veilanon **hiçbir özel (custom) kripto algoritması** içermez. Kullanılacak her şey: AES-256-GCM, X25519, Ed25519, Argon2id, HKDF. Implementasyonlar: Rust'ta `aes-gcm`, `x25519-dalek`, `ed25519-dalek`, `argon2` crate'leri — tamamı RustCrypto/dalek ekosisteminden, topluluk denetiminden geçmiş, CI'da cargo-audit ile izlenir. Kendi yazacağımız tek şey: bu ilkellerin **bağlanması** (key schedule, zarf formatı, versiyonlama) — o da `schema_version` alanıyla yönetilir.

*Reddedilen:* Kimsenin bakmadığı "yeni şifre" fikirleri. Gerekçe: Schneier yasası.

## AD-2: Şifre türetme — Argon2id (KABUL)

Yerel anahtar türetmede Argon2id kullanılır. Parametreler (cihaz profiline göre ölçeklenir): `m=64 MiB, t=3, p=4` (masaüstü) / `m=19 MiB, t=2, p=1` (düşük bellek profili). Salt: 16 bayt `getrandom`. Çıktı: 32 bayt HKDF-SHA256 ile ayrı anahtar malzemesine genişletilir. Argon2id hem GPU hem yan-kanal saldırılarına karşı dengeli olduğu için seçildi. *Reddedilen:* scrypt (hafıza-sertlik dengesi zayıf), PBKDF2 (GPU direnci yok), Argon2d (yan-kanal maruziyeti).

## AD-3: Mesaj zarfı — AES-256-GCM (KABUL)

Zarf formatı: `[schema_version(1B) | iv(12B) | ciphertext+tag]`, taban64 kodlu olarak `messages.ciphertext` + `messages.iv` sütunlarında taşınır. GCM seçildi çünkü AEAD + donanım hızlandırma + RustCrypto denetimi. Nonce (IV) her zarf için benzersizdir — tekrarı, anahtar yeniden türetmeyi tetikler. *Reddedilen:* ChaCha20-Poly1305 (düşük uçlu mobil hedefi olmadığı için gereksiz; yine de `schema_version=2` adayı olarak saklı), CBC+HMAC (EtM kompozisyon hatası riski).

## AD-4: Kimlik ve anahtar anlaşması — X25519 + Ed25519 (KABUL)

Her cihaz bir X25519 (DH) ve bir Ed25519 (imza) çifti üretir; yalnızca **public** anahtarlar `devices` tablosuna yüklenir. Özel anahtarlar OS keychain'de, keychain'den türetilen yerel anahtarla sarılıdır. İki eğri arasında tek anahtar paylaşımı yapılmaz (ağrısız kural: DH imzalamaz, imza DH yapmaz). *Reddedilen:* Tek eğri (Ed25519 ile DH'yi dönüştürme), RSA (boyut + hız), P-256 (uygulama farklılıkları).

## AD-5: Grup kanalları — şimdilik statik paylaşımlı anahtar; MLS aday (KABUL, geçici)

v1 grup E2EE'si: kanal kurucusu rastgele 32 bayt kanal anahtarı üretir, her üye cihazın X25519 public anahtarına **kripto kutulama** (Ecies benzeri, HKDF+AES-GCM) ile sarar. Üye ekleme/çıkarmada anahtar rotasyonu client tarafında yapılır. Bu geçici ve dürüsttür: ileri gizlilik (forward secrecy) yoktur.

*Hedef:* **MLS (openmls)** — endüstri standardı, sürekli grup anahtar anlaşması. **Kapı:** openmls implementasyonu bağımsız güvenlik incelemesinden geçmeden ve IETF MLS RFC'sine uyumlu olduğu doğrulanmadan prod'a alınmaz. Arada DM'ler için çift-yönlü ratchet (Signal çift ratchet deseni) **denetlenmiş bir kütüphaneye** bindiğinde devreye girer; kendi ratchet implementasyonumuzu yazmak yasaktır (AD-1). Aday kütüphane değerlendirme listesi `docs/DATA_INVENTORY.md` değildir; seçim ayrı bir AD ile kapatılır.

## AD-6: Beklemede şifreleme — uygulama katmanı AES-256-GCM; SQLCipher drop-in (KABUL)

Yerel SQLite'da hassas sütunlar (mesaj gövdeleri, taslaklar, kuyruk) uygulama katmanında AES-256-GCM ile şifrelenir; anahtar keychain'den gelir. **Neden bundled SQLite:** Tauri'nin tarayıcı dışı sürecinde sıfır ek kurulumla çalışır, cross-compile sorunu çıkarmaz ve geliştirme makinelerinde (CI dahil) her platformda aynı davranır. SQLCipher (derlenmiş uzantı) gerektirmez. *Değişim koşulu:* Tam disk şifrelemesi isteyen dağıtımlar için SQLCipher, sütun şifrelemesini kaldırmadan dosya düzeyinde ek katman olarak **drop-in** şekilde belgelenmiştir — çünkü SQLCipher page-level şifrelerken uygulama katmanı anahtar rotasyonu ve alan bazlı erişim kontrolünü korur. İkisi birden "çift şifreleme" değildir; ikisi farklı tehditleri örter (dosya kopyası vs. SQL injection/segment dump).

## AD-7: Medya — LiveKit E2EE anahtarı istemci sorumluluğu (KABUL, fazlı)

Medya oturumlarında E2EE anahtarı (`E2EE Manager`) istemci tarafında üretilir; LiveKit'e iletilen çerçeveler istemcide şifrelenir/çözülür. Sunucu (SFU) yalnızca şifreli medya görür. **Faz 1 (şimdi):** DTLS-SRTP taşıma şifrelemesi — SFU içeriği görür; UI rozeti "SFU şifreli (planlanan)" (PRIVACY_SCOPE). **Faz 2:** LiveKit e2ee-kit entegrasyonu, anahtar dağıtımı mekân anahtarı kutulamasıyla (AD-5). *Reddedilen:* Kendi SRTP key yönetimimiz (AD-1 ihlali), sunucu taraflı kayıt şifreleme.

## AD-8: Rastgelelik — işletim sistemi CSPRNG (KABUL)

Tüm anahtar/nonce üretimi OS CSPRNG üzerinden: Rust `getrandom` (Windows BCryptGenRandom, macOS Security, Linux getrandom). Kullanıcıdan entropi toplama kodu yazılmaz. DB tarafında davet kodları için pgcrypto `gen_random_bytes`.

## AD-9: Sürümleme ve esneklik (KABUL)

Her zarf `schema_version` taşır; her şema değişikliği yeni AD gerektirir. Anahtar gücü artırma veya ilkel değiştirme ancak hem eski hem yeni şemayı okuyabilen istemci sürümü yayınlandıktan sonra prod'a girer (çift okuma penceresi).

## Değişim kapısı (her AD için)

1. Yeni ilkel teklifi → AD-1 masasına uygunluk kontrolü.
2. Kütüphane swap → cargo-audit temiz + SBOM diff + bağımsız inceleme (kripto kütüphaneleri için).
3. Parametre değişimi → tehdit modeli güncellemesiyle aynı PR.

## AD-10: İmza kullanımı — Ed25519 (KABUL)

İmzalama gereken her şey Ed25519 kullanır: cihaz kimlik doğrulaması, bot manifest imzaları (BOT_API), webhook olay imzaları, cihaz-doğrulama paketleri. Neden: küçük anahtar (32B), deterministik olmayan güvenli imza, RFC 8032'de sabit, dalek implementasyonu denetimli. *Reddedilen:* ECDSA (nonce tekrarı felaketi — deterministik modda bile), RSA-PSS (boyut + hız), Schnorr (standardizasyon dışı kalmış varyantlar). İmza şeması değişirse bu bir `schema_version` kırılmasıdır (AD-9).

## AD-11: Anahtar türetme zinciri — HKDF-SHA256 (KABUL)

Argon2id çıktısı doğrudan anahtar olarak kullanılmaz; HKDF-SHA256 ile alan-ayrımlı türetme yapılır (`info` alanları: `"veilanon:v1:store"`, `"veilanon:v1:identity"`, `"veilanon:v1:queue"`). Böylece tek giriş malzemesinden bağımsız anahtarlar üretilir; bir alanın anahtarı diğerini açmaz. Kanal anahtarı kutulamasında da (AD-5) geçici DH paylaşımı HKDF'den geçer (`"veilanon:v1:channel-box"`).

## AD-12: Anahtar rotasyonu politikası (KABUL)

| Tetikleyici | Aksiyon | Kapsam |
| --- | --- | --- |
| Üye cihaz kaldırma | kanal anahtarı rotasyonu | yalnızca ilgili kanallar |
| Üye ayrılma / atılma | kanal anahtarı rotasyonu | tüm ortak kanallar |
| Cihaz kaybı şüphesi | kimlik anahtarı değişimi + tüm kanal anahtarlarının yeniden kutulanması | hesap çapında |
| IV tekrarı şüphesi | anahtar rotasyonu + `schema_version` artırımı | ilgili kanal |
| Şema güncellemesi | çift okuma penceresi (AD-9) | global |

Rotasyon her zaman istemci tarafında yapılır; sunucu rotasyon olaylarını yalnızca yeni zarflar olarak görür.

## AD-13: Yedekleme/kurtarma konumu (KABUL — kısıtlı)

Kullanıcı anahtarları şifrelenmiş **yerel yedek** dosyası olarak dışa aktarılabilir (Argon2id ile türetilen anahtarla sarılı). Sunucu tabanlı anahtar yedekleme **yapılmaz** — sunucunun anahtar tutması AD-1'in ruhuna aykırıdır (A1 saldırganına hediye). Kaybolan anahtar = kaybolan tarihçe; bu, gizlilik lehine bilinçli bir trade-off'tur ve kurulum ekranında açıkça söylenir.

## AD-14: Zarf detayları (KABUL)

Mesaj zarfı içinde yalnızca: düz metin, alıcı bilgisi gerektirmez (kanal anahtarı paylaşımlıdır), isteğe bağlı `disappears_at` ipucu ve dosya referansları (`r2_key` + sarılmış dosya anahtarı). Zarfa asla: sunucu zamanı, IP, cihaz kimliği, kullanıcı kimliği yazılmaz — bunlar metadata'dır ve dışarıda (zarftan ayrı) durur; böylece zarf, sunucunun metadata görüşünü artıramaz.

## Açık sorular (bilinçli olarak kapatılmamış)

1. **MLS kapısı:** openmls bağımsız inceleme sonucu beklemede (AD-5). Kapanmadan grup kanalları statik kutulama ile çalışır.
2. **Ratchet kütüphanesi seçimi:** DM çift-ratchet adaptörü için aday kütüphaneler değerlendirmede; denetimli olmayan hiçbir implementasyon içeri girmeyecek.
3. **Padding:** Trafik analizi azaltımı (THREAT_MODEL A2) tasarım aşamasında; `schema_version=2` adaylarından.
4. **Kuantum geçiş:** İmza tarafı için devlet destekli standartlar olgunlaşana dek X25519+Ed25519 kalır; geçiş yolu AD-9 çift okuma penceresiyle planlanır.

## Uygulama crate tablosu (AD-1'in somut karşılığı)

| İlkel | Crate | Denetim notu |
| --- | --- | --- |
| AES-256-GCM | `aes-gcm` (RustCrypto) | RustCrypto AEAD'leri, çok sayıda bağımsız değerlendirmeden geçmiştir |
| X25519 | `x25519-dalek` | curve25519-dalek ekosistemi; formal doğrulama çalışmaları mevcuttur |
| Ed25519 | `ed25519-dalek` | dalek; zerocopy çekirdek, RFC 8032 uyumlu |
| Argon2id | `argon2` (RustCrypto) | PHC referans uyumu testleriyle |
| HKDF | `hkdf` (RustCrypto) | RFC 5869 uyumlu |
| Rastgelelik | `getrandom` | OS CSPRNG ince sarmalayıcı |

Kural: bir crate'in sürümünü yükseltmek bile "kütüphane swap" kapısından geçer (cargo-audit + SBOM diff). `Cargo.lock` kilitlidir; CI `--locked` ile doğrular.

## Anahtar törenleri (key ceremony)

1. **Cihaz kurulumu:** kimlik anahtarları ilk açılışta üretilir; public anahtarlar `devices` tablosuna yazılır. Özel anahtar keychain'e gömülür; hiçbir log/metrik anahtar parçası içermez.
2. **Kanal kurulumu:** kurucu 32 bayt kanal anahtarı üretir → her üye cihaza kutulanır (AD-5). Kanal anahtarı yalnızca kutulu halde taşınır.
3. **Cihaz doğrulama:** yeni cihaz, mevcut cihazın Ed25519 ile imzaladığı onay paketini sunmadan grup anahtarı alamaz (THREAT_MODEL S3).

## Test vektörleri politikası

Her kripto bağlama (key schedule, zarf formatı) için sabit test vektörleri (known-answer test) repo'da `crypto/vectors/` altında tutulur; vektörler RFC'lerden veya referans implementasyonlardan üretilir, elle uydurulmaz. Vektör seti `cargo test` ile her CI'da çalışır.

## Kabul kriterleri (yakınsama)

Bu günlüğe yeni AD eklemek için üç koşul: (1) tehdit modelinde karşılık gelen satır, (2) PRIVACY_SCOPE matrisine etkisi yazılmış, (3) test vektörü veya denetim referansı mevcut. Koşulsuz AD kabul edilmez.
