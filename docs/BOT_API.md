# veilanon — Bot API

Botlar, mekân yöneticisinin **açık izniyle** ve **imzalı manifest** ile çalışan, kapsamı daraltılmış otomasyonlardır. Bu belge API sözleşmesini ve güvenlik kurallarını tanımlar.

## 1. Manifest (imzalı)

Her bot, mekâna kurulmadan önce bir manifest yayınlar:

```json
{
  "name": "example-bot",
  "description": "What it does, in plain language",
  "version": "1.0.0",
  "author": "bot-author",
  "permissions": ["messages.read:non-e2ee", "messages.send:non-e2ee"],
  "webhook_url": "https://bots.example.com/hook",
  "events": ["message.created", "member.joined"],
  "public_key": "-----BEGIN PUBLIC KEY-----\n...\n-----END PUBLIC KEY-----"
}
```

Manifest, bot yazarının Ed25519 özel anahtarıyla imzalanır; imza mekâna kurulum sırasında `public_key` ile doğrulanır. **İmzası doğrulanmayan manifest kabul edilmez.** Manifest değişikliği yeniden imza ve yönetici onayı gerektirir.

## 2. Yetki (capability) sistemi

Botlar hiçbir zaman üye değildir; yalnızca manifestte listelenen kapsamla çalışır:

| Yetki | Açıklama | E2EE kanal davranışı |
| --- | --- | --- |
| `messages.read:non-e2ee` | şifresiz kanallardaki mesajları okur | E2EE kanalda ciphertext görür |
| `messages.send:non-e2ee` | şifresiz kanallara yazar | E2EE kanala yazamaz (rıza gerekir) |
| `messages.read:e2ee` | E2EE kanalda düz metin okur | **yalnızca kanal yöneticisinin açık rızasıyla**; rıza kaydı audit'e düşer |
| `messages.send:e2ee` | E2EE kanala yazar | aynı rıza kuralı |
| `members.read` | roster okur | — |
| `channels.manage` | kanal oluşturur/düzenler | — |
| `webhook.emit` | dışarıya olay yayınlar | içerik asla dahil edilmez |

Yetki listesi genişletilebilir; her yeni yetki bu belgede ve manifest şemasında birlikte yayınlanır.

## 3. Tokenlar (kapsamı daraltılmış, dönen)

- Kurulumda bot için **kapsamlı (scoped) token** verilir; token yalnızca onaylanan yetkileri taşır.
- Tokenlar **kısa ömürlüdür ve otomatik döner** (yenileme, webhook imza anahtarından ayrıdır).
- İptal: mekân yöneticisi tek tuşla botu kaldırır; tüm tokenlar anında geçersiz olur.
- Token hiçbir logda görünmez.

## 4. Webhook olayları (versiyonlu + idempotent + imzalı)

Her olay HTTP POST olarak `webhook_url`'e gönderilir:

```
X-Veilanon-Signature: v1=ed25519_hex_signature
X-Veilanon-Event-Id: 550e8400-e29b-41d4-a716-446655440000
X-Veilanon-Event-Version: 1
```

Gövde:

```json
{
  "event_version": 1,
  "event_id": "550e8400-...",
  "type": "message.created",
  "space_id": "...",
  "channel_id": "...",
  "occurred_at": "2026-08-15T12:00:00Z",
  "payload": { "message_id": "...", "sender_public_metadata": { "username": "alice" } }
}
```

Kurallar:

1. **Versiyonlu:** `event_version` her şema değişiminde artar; eski sürümler 6 ay desteklenir.
2. **Idempotent:** Teslim yeniden denemelerinde aynı `event_id` tekrarlanır; bot tekrarları görmezden gelmelidir.
3. **İmzalı:** İmza, manifestteki anahtarla doğrulanır; bot imzayı **mutlaka** kontrol eder.
4. **İçerik kuralı:** `payload` içine **asla** düz metin içerik girmez; E2EE kanal olaylarında yalnızca id + metadata.

## 5. Slash komutları

Botlar `/bot-name command args` biçiminde komut kaydeder. Komut kayıt listesi manifestin parçasıdır. Komut çalıştırmada geçerli olan yetki denetimi webhook ile aynıdır; komut içerikleri de `payload` kurallarına tabidir.

## 6. E2EE kanal kuralları (özet)

1. Varsayılan: bot, E2EE kanalda **ciphertext'i** görür — şifreli kargo, düz metin değil.
2. Düz metin erişimi **kanal yöneticisinin açık rızası** gerektirir; rıza `audit_events`'e `event_type='bot.e2ee_consent'` olarak yazılır.
3. Botun anahtar alması, üye cihazlarına eklenen "bot cihazı" olarak modellenir — bot bu cihazı kendi keychain'inde saklar; sızıntısı üye sızıntısıyla eşdeğerdir (THREAT_MODEL A5).
4. Rıza geri alınırsa kanal anahtarı rotasyonu tetiklenir; bot yeni anahtarı alamaz.

## 7. Uyumluluk denetimi

CI'da manifest şeması doğrulanır; webhook imza doğrulaması referans uygulaması repo'daki `examples/` altındadır (istendiğinde eklenir). Bu belge ile uygulama arasında sapma tespit edilirse uygulama, belgeye uydurulur.

## 8. Örnek kurulum akışı

1. Bot yazarı manifest üretir ve Ed25519 ile imzalar; webhook sunucusunu ayağa kaldırır.
2. Mekân yöneticisi "Bot ekle" sihirbazını açar, manifesti yapıştırır → imza doğrulanır → yetki listesi ekranda düz dille gösterilir.
3. Yönetici onaylar; sistem kapsamlı token üretir ve botun webhook'una `bot.installed` olayı gönderir (imzalı).
4. Bot, olaylara `event_id` üzerinden idempotent yanıt verir; komutlar slash üzerinden kaydedilir.
5. Kaldırma: token iptali + `bot.removed` olayı + audit kaydı. Bot hiçbir iz bırakmaz.

## 9. Hız limitleri ve hata kodları

| Sınır | Değer |
| --- | --- |
| Olay teslim denemesi | 5 (exponential backoff, 1dk–1saat) |
| Webhook yanıt süresi | 10 sn; aşarsa yeniden deneme |
| Komut çalıştırma sıklığı | mekân başına 30/dk |
| Mesaj yazma hızı | yetki başına 60/dk (toplu modlar reddedilir) |

Hata dönüşleri: `4xx` = kalıcı (bot düzeltmeli), `5xx` = geçici (yeniden dene). İmza hatası: `401` + teslim durdurulur.

## 10. Kapsam dışı (v1)

- Botlar arası iletişim (bot-to-bot) — tek atlama kuralı: bot yalnızca mekânla konuşur.
- Para/monetizasyon — botlar ücretsizdir; ücretli bot API'si ayrı karar gerektirir.
- E2EE kanalda rızasız **düz metin** — mimari olarak imkânsızdır; bot ciphertext alır (bölüm 6).
