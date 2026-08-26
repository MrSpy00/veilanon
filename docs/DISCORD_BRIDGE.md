# veilanon — Discord Köprüsü (Bridge)

veilanon, Discord ile **yalnızca politika uyumlu** bir köprü kurar. Amaç: toplulukları taşırken hem kendi güvenlik iddialarımızı hem Discord'un kurallarını ihlal etmemek.

## 1. Politika (bağlayıcı)

1. **Yalnızca OAuth2.** Köprü, Discord'un resmî OAuth2 uygulaması üzerinden çalışır. Kullanıcı adı/şifre girişi ve kullanıcı token'ı (self-bot) **kesinlikle yoktur** — bu Discord ToS ihlalidir ve güvensizdir.
2. **Bot token'ı yalnızca etiketli köprü hesabı içindir.** Köprü, kendi bot kimliğiyle ve `bot` scope'uyla mesaj taşır; taşınan her mesaj kullanıcı arayüzünde **"via veilanon"** etiketi taşır.
3. **Rıza görünürlüğü:** Köprülenen her kanal, her iki tarafta açıkça işaretlenir; kullanıcı hangi tarafın Discord'a geçtiğini bilir.

## 2. Mimari

```
veilanon kanalı ──> Bridge adaptörü (Rust) ──> Discord REST/Gateway
      (şifreli)          (yalnızca açık kanallar)      (bot token'ı)
```

- **Yalnızca şifresiz (`is_e2ee = false`) kanallar köprülenebilir.** E2EE kanalın içeriği istemci dışında hiçbir yerde düz metin olmadığından (PRIVACY_SCOPE) köprülenmesi mimari olarak imkânsızdır; denemek de yasaktır.
- Köprü tek yönlü veya çift yönlü açılabilir; varsayılan **tek yönlü (veilanon → Discord) okuma** modudur.

## 3. İçe aktarma sihirbazı (yerel-only)

- Kullanıcı, Discord'dan **resmî "Export" paketini** (GDPR veri paketi) indirir.
- Sihirbaz paketi **tamamen yerelde** okur; içerik hiçbir sunucuya gönderilmez.
- İçe aktarma hedefi yalnızca **yeni, E2EE işaretli** mekân/kanallardır — taşınan geçmiş anında E2EE'ye girer.
- İşlem sonunda kullanıcıya "paketi sil" hatırlatması yapılır.

## 4. Uyumluluk adaptörünün sınırları

| Özellik | Durum | Not |
| --- | --- | --- |
| Metin mesajları | desteklenir | şifresiz kanallarda |
| Dosyalar | desteklenir | Discord CDN bağlantısı + boyut sınırı; içerik taşınmaz, bağlantı taşınır |
| Ses/görüntü | **desteklenmez** | Discord sesi ile LiveKit arasında köprü güvenli değildir |
| Roller/izinler | kısmi eşleme | rol adları taşınır, yetki bit'leri eşlenmez |
| Tepkiler (reactions) | emoji metnine indirgenir | — |
| Düzenleme/silme | sınırlı yansıtma | webhook idempotency kurallarıyla (BOT_API) |
| Düzenlenmemiş anlık senkron | yok | köprü 5–30 sn aralıklı seyreltme ile çalışır |

## 5. Güvenlik ve denetim

- Köprü kimlik bilgileri (Discord client id/secret) yalnızca `.env` üzerinden, hiçbir zaman repoda değildir (`.env.example` boştur).
- Her köprü olayı `audit_events`'e `event_type='bridge.message_relayed'` olarak yazılır (içerik yok, yalnızca id'ler).
- Discord token'ı yalnızca köprü adaptörü sürecinde bellekte tutulur; diskte saklanmaz.
- Köprünün bot token'ı sızarsa: token iptal edilir, yeni token yayınlanır — kullanıcı anahtarları etkilenmez (E2EE anahtarları Discord'dan bağımsızdır).

## 6. Gelecek

- Opsiyonel self-host köprü servisi (aynı politika, kendi sunucunda).
- Gelişmiş içe aktarma: Discord "Export" paketinin yeni formatlarının takibi.

## 7. Kanal eşleme kuralları

| Discord | veilanon | Kural |
| --- | --- | --- |
| `#genel` (text) | text kanalı | birebir; kategori adı prefix olur: `genel–sohbet` |
| voice kanalı | voice kanalı (boş kabuk) | eşlenir ama akış köprülenmez (bölüm 4) |
| forum kanalı | forum kanalı | gönderi → başlık + ilk mesaj |
| duyuru kanalı | announcement | yalnızca Discord → veilanon yönü |

## 8. Mesaj biçim eşlemesi

| Discord öğesi | veilanon karşılığı |
| --- | --- |
| düz metin | düz metin |
| **kalın** / *italik* / `kod` | birebir markdown |
| emoji (unicode) | birebir |
| özel emoji | `:ad:` olarak bırakılır |
| ek (attachment) | CDN bağlantısı metni + boyut |
| embed | başlık + açıklama düz metnine indirgenir |
| reply | `> alıntı` öneki |
| @mention | kullanıcı adı düz metnine indirgenir |

## 9. Hız sınırı ve başarısızlık modları

- Discord API 429 → exponential backoff; sürekli 429 → köprü duraklar ve mekân yöneticisine bildirir.
- Gateway kopması → otomatik yeniden bağlanma (identify + resume); 3 başarısız deneme → 5 dk soğuma.
- Teslim edilemeyen mesaj → veilanon tarafında `köprü hata` rozetli yerel bildirim; mesaj kaybolmaz (yerel mağazada).
- Kısmi kesinti: tek yön modunda veilanon tarafı etkilenmez; yalnızca röle durur.

## 10. Yayın planı (rollout)

1. **Faz A (şimdi):** İçe aktarma sihirbazı (yerel-only) + tek yönlü metin köprüsü (etiketli).
2. **Faz B:** Çift yönlü metin + dosya bağlantısı + audit olayları tamamlanınca.
3. **Faz C:** Self-host köprü servisi ve topluluk operatörlerine rehber.

Her faz, bu belgenin politika bölümü (bölüm 1) ihlal edilmeden yayınlanır; ihlal eden herhangi bir özellik reddedilir.

## 11. Sorumluluk reddi

veilanon köprüsü Discord'un resmî ürünü değildir; Discord ile bağlantı, Discord'un hizmet şartlarına uygunluk kullanıcının sorumluluğundadır. Köprü yalnızca resmî OAuth2/bot API'lerini kullanır ve hiçbir durumda kullanıcı token'ı işlemez.

## 12. Yapılandırma referansı

```bash
VEILANON_DISCORD_CLIENT_ID=        # Discord OAuth2 application client id
VEILANON_DISCORD_CLIENT_SECRET=    # OAuth2 client secret (yalnızca .env)
```

Köprü adaptörü ayrıca şu sabitleri kullanır (değiştirilmesi yeni karar gerektirir):

| Sabit | Değer | Neden |
| --- | --- | --- |
| Seyreltme aralığı | 5–30 sn | Discord rate-limit uyumu |
| Maks. ileti boyutu | 2000 karakter | Discord API sınırı |
| OAuth2 scope | `identify`, `guilds` (kullanıcı), `bot` (köprü) | yalnızca gereken |
| Yeniden deneme | 5 deneme, exp. backoff | bölüm 9 |

## 13. Kabul testleri (köprü için)

1. Kullanıcı token'ı hiçbir kod yolunda istenmez — statik grep kuralı: `user token`, `self-bot` geçmemelidir.
2. E2EE kanalı köprüleme denemesi reddedilir ve kullanıcıya neden açıklanır.
3. Köprülenen mesaj "via veilanon" etiketi taşır (UI + audit).
4. İçe aktarma sihirbazı çevrimdışı (ağ kapalı) çalışır ve içerik sızdırmaz.
5. Rate-limit testi: sürekli 429'da köprü durur, çökmez.

## 8. Uygulanan yol (2026-08-16): Webhook köprüsü

OAuth2 uygulama kimlikleri gerektirmeden çalışan ilk gerçek uygulama **webhook tabanlıdır**:

- Kanal sahibi Discord'da kendi sunucusunda bir webhook oluşturur; URL yalnızca kendi cihazında saklanır (discord_webhooks — yerel ve Supabase RLS korumalı).
- send_message sonrası mesaj, "[köprü]" etiketiyle webhook'a yansıtılır (mirror_message, best-effort).
- Yalnızca is_e2ee = false kanallar köprülenebilir; UI köprü kurulurken açıkça uyarır.
- OAuth2 akışı (uygulama kimlikleri) eklendiğinde src/lib/api/discord-bridge.ts sözleşmesi üzerinden bağlanır; kullanıcı token'ı hiçbir veilanon sunucusunda saklanmaz.
