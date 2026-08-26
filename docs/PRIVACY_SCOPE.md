# veilanon — Gizlilik Kapsamı (Privacy Scope)

Bu belge "E2EE" etiketinin neyi kapsadığını, neyi kapsamadığını alan alan tanımlar. Amaç: pazarlama iddiası değil, denetlenebilir bir söz.

## 1. Gizlilik matrisi

| Alan | Sunucunun gördüğü | Şifreleme | Etiket |
| --- | --- | --- | --- |
| Mesaj içeriği | yok (yalnızca ciphertext) | AES-256-GCM, istemcide | **İçerik E2EE** |
| Mesaj kimden / hangi kanala | evet (`sender_device_id`, `channel_id`) | — | Metadata |
| Mesaj zamanı | evet (`server_received_at`, `client_created_at`) | — | Metadata (saat kovası uygulanmaz, zamanlama görelidir) |
| Mesaj boyutu | evet (ciphertext uzunluğu) | — | Metadata |
| Dosya içeriği | yok (şifreli blob) | AES-256-GCM, istemcide | **İçerik E2EE** |
| Dosya adı | yok (R2 anahtarı UUID'dir) | — | **İçerik E2EE** |
| Dosya boyutu | evet (`size_bytes`) | — | Metadata |
| Kullanıcı adı / görünen ad / avatar | evet (minimal profil) | — | Genel metadata |
| E-posta / telefon | yalnızca Supabase Auth (operatör de görür) | — | Kimlik |
| Çevrimiçi durum | evet (saatlik kova) | — | Metadata, saatlik |
| Üyelikler / roller | evet | — | Metadata |
| Ses/görüntü içeriği | SFU'da şifresiz (DTLS-SRTP) | LiveKit E2EE planlandı | **Şu an: taşıma şifreli, SFU'ya açık** |
| Davet kodları | evet | — | Metadata |

## 2. Etiketler (kullanıcı arayüzünde gösterilir)

- **İçerik E2EE:** Yalnızca üye cihazları okuyabilir. Kanallarda bu rozet gösterilir.
- **Metadata görünür:** Sunucu; kim/ne zaman/nerede bilgisini bilir. Profil sayfasında dürüstçe belirtilir.
- **Yerel şifreli:** Veri yalnızca bu cihazda şifreli saklanır (yerel mağaza).
- **SFU şifreli (planlanan):** Ses/görüntü oturumlarında LiveKit E2EE henüz aktif değilse bu rozet gösterilir.

## 3. Varsayılanlar tablosu

| Ayar | Varsayılan | Neden |
| --- | --- | --- |
| Yeni kanal türü | `text`, `is_e2ee = true` | Gizlilik öncelikli varsayılan |
| Varlık (presence) | Açık, saatlik kova | Topluluk deneyimi vs. izlenebilirlik dengesi |
| Son görülme | Yalnız saat kovası (dakika yok) | Hassas zamanlama ifşasını sınırla |
| Profil görünürlüğü | Kullanıcı adı + görünen ad + avatar | Minimal profil fonksiyonu `get_public_profile` yalnızca bunları döndürür |
| Hata izleme (Sentry) | Kapalı (opt-in) | İçerik asla gönderilmez; opt-in'de bile içerik alanları strip edilir |
| E-posta bildirimleri | Kapalı | Veri minimizasyonu |
| Kaybolma (disappears_at) | Kapalı | Kullanıcı kanal bazında açar |

## 4. Uygulamadaki karşılıklar (denetim noktaları)

1. `supabase/migrations/0001_initial.sql` — `messages` tablosunda düz metin sütunu yoktur; `files` tablosunda ad sütunu yoktur.
2. `deliver-message` Edge fonksiyonu gövdeyi loglamaz; yalnızca id + uzunluk yazar.
3. `get_public_profile()` fonksiyonu beyaz listeli sütunlar döndürür; `users` tablosu RLS ile kendi satırına kilitlidir.
4. `presence.last_seen_bucket` saatlik dilimdir (epoch/3600).
5. `audit_events` tablosunda içerik alanı yoktur.

## 5. Ağ Gizliliği, Tor ve Proxy Kapsamı

- **Uygulama İçi Tor SOCKS5h & Proxy Modu:** Veilanon, yerleşik ayarlar üzerinden tüm REST, dosya transferi ve tanılama çağrılarını yerel Tor SOCKS5h (`socks5h://127.0.0.1:9050` / `9150`) veya özel bir SOCKS/HTTP proxy üzerinden geçirebilir. `socks5h` kullanıldığında alan adları proxy tarafında çözümlenir (DNS sızıntısı engellenir).
- **Kesin Gizlilik Modu (Fail-Closed / Kill-Switch):** Tor/Proxy bağlantısı kesildiğinde uygulamanın doğrudan açık internete (clear-net) çıkış yapmasını engelleyerek gerçek IP sızıntılarını önler.
- **Sistem Seviyesi VPN:** Veilanon'un Tor/Proxy modu uygulama kapsamlıdır. Bilgisayarın diğer yazılımlarının tüm trafiğini korumak için sistem seviyesinde WireGuard/OpenVPN istemcisi gereklidir; Veilanon profil yapılandırma rehberliği sunar.
- **Trafik analizi direnci (padding):** planlanmıştır, v1'de yoktur (THREAT_MODEL A2).
- **Anonimlik ve Meta Veri:** Tor kullanımı IP karartması sağlar; ancak hesap bazlı E2EE şifreleme ve Supabase oturum yapısı gereği kanallar ve hesap kimlikleri protokol seviyesinde ayrıştırılır.

## 6. Denetlenebilirlik

Bu matris, kod ve şemayla birebir eşleşmelidir. Her sürümde SBOM + şema diff'i yayınlanır; meraklısı `DATA_INVENTORY.md` ile bu matrisi karşılaştırabilir.

## 7. Özellik bazında veri akışı

### Mesaj gönderme
Kullanıcının yazdığı düz metin **yalnızca** istemci belleğinde ve yerel şifreli mağazada bulunur. Kablodan ve sunucudan geçen tek şey: `channel_id`, `sender_device_id` (opak), `ciphertext`, `iv`, `schema_version`, zaman damgaları. Sunucunun bu akışta düz metin görme şansı mimari olarak sıfırdır.

### Dosya yükleme
Dosya istemcide şifrelenir → R2'ye rastgele UUID anahtarla yüklenir → `files` tablosuna `r2_key` + `size_bytes` + `content_key_ciphertext` yazılır. Dosya adı sunucuya **hiçbir noktada** ulaşmaz; ad bilgisi yalnızca mesaj zarfının içinde (E2EE) taşınır.

### Sesli/görüntülü oturum
Faz 1: akış DTLS-SRTP ile taşımada şifrelidir; SFU içeriği çözebilir. Faz 2 (LiveKit e2ee-kit): içerik istemcide şifrelenir, SFU yalnızca şifreli paketleri yönlendirir. Aradaki fark kullanıcıya rozet ile gösterilir (bkz. bölüm 2).

### Varlık (presence)
İstemci her durum değişiminde `status` + saatlik `last_seen_bucket` yazar. Dakika hassasiyeti bilinçli olarak yoktur; "10:42'de çevrimiçiydi" bilgisi sunucuda oluşamaz.

## 8. Rozet uygulama notları

- Kanal başlığında `is_e2ee` rozeti şemadaki sütundan gelir — istemci başka kaynak icat etmez.
- "SFU şifreli (planlanan)" rozeti LiveKit E2EE manager'ının varlığına göre değişir; sessizce düşmez.
- Profil sayfasında "Metadata görünür" bilgi kutusu; kullanıcıyı iddiasız bilgilendirir.

## 9. Sık sorulanlar

**S: Sunucu neden mesajları hiç okuyamıyor, ya operatör kötü niyetliyse?**
E2EE anahtarları yalnızca üye cihazlarında üretilir ve keychain'de tutulur; sunucuya hiçbir anahtar malzemesi gitmez. Operatör isterse şifreli zarfları görebilir, çözemez (A1).

**S: Metadata neden şifrelenmiyor?**
Röle mimarisi, mesajı doğru kanala teslim etmek için kanal/zaman bilgisini bilmek zorundadır. Şifreli metadata güvenli biçimde sorgulanamaz (arama/filtreleme kırılır). Bu, merkezi mimarinin açık ve kabul edilmiş bedelidir.

**S: Kaybolan mesajlar (disappears_at) sunucuda gerçekten siliniyor mu?**
`disappears_at` bir istemci sözleşmesidir: istemciler süre dolunca yerelde siler ve sunucuya tombstone (`deleted_at`) yazar. Sunucudaki ciphertext'i kullanıcı silme isteği ile `null` edilebilir; tam fiziksel temizlik operatör politikasıdır (DATA_INVENTORY bölüm 5).

**S: E2EE'nin bağımsız denetimi var mı?**
Kendi algoritma yazmıyoruz (AD-1); kullandığımız kütüphaneler topluluk denetimlidir. Bağımsız tam denetim (pentest) planlanmış olup sonucu yayınlanacaktır.

## 10. Değişiklik süreci

Bu matriste herhangi bir değişiklik (örn. yeni metadata alanı) şunları gerektirir: aynı PR'da `DATA_INVENTORY.md` güncellemesi, `0001_initial.sql` veya yeni migration, ve THREAT_MODEL tablosunda karşılık satırı. "Unuttuk" kabul edilebilir bir açıklama değildir.

## 11. Belge sürümü

| Sürüm | Tarih | Not |
| --- | --- | --- |
| 1.0 | 2026-08 | v1 matrisi; SFU Faz 1 durumu |

## 12. Özet cümle (kullanıcıya söylenen)

> "veilanon içeriklerini şifreler; sunucu ne mesajlarını ne dosyalarını okuyabilir. Sunucu yalnızca kimin, hangi kanalda, ne zaman yazdığını bilir — bu metadata görünürlüğünü saatlik kovalama ve minimal profillerle sınırlandırıyoruz."
