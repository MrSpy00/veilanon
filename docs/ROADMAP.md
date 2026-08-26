# veilanon — Roadmap

Bu belge, kısa vadede implemente edilen özelliklerin ötesinde kalan, mimari ağırlıklı işleri ve "tamamlanma" tanımlarını listeler. İçerik, ürün gereksinim dokümanındaki (PRD) bağlayıcı kararlarla tutarlıdır.

## Tamamlanan (v0.0.1 — Proje Çöp Temizliği, Sohbet Barı Piksel Hizalama & Çoklu Platform Release Senkronizasyonu)

- **Kapsamlı Depo ve Çöp Dosya Temizliği**:
  - Projedeki tüm tek seferlik geçici scriptler (`brand/make_svg.py`, `release-sync-check/`), çift raster logolar (`brand/veilanon-logo.jpg`, `src-tauri/icons/source/veilanon-logo.jpg`), mükerrer Vercel konfigürasyonu (`static/vercel.json`) ve eski/kopya veritabanı scriptleri temizlendi.
- **Sohbet Giriş Barı (MessageInput) Tasarım & Hizalama Devrimi**:
  - Yanıtlanan mesaj, bekleyen dosya ekleri ve link önizleme kartları için dikey istiflenen `.veil-composer-top-stack` mimarisi kuruldu; absolute çakışma ve üst üste binme hatası tamamen giderildi.
  - Textarea metin dolguları ve butonlar (`36px x 36px`) tam dikey merkezde piksel piksel hizalandı.
  - `isMultiline` hesaplaması metin kaydırmasını (`textareaScrollHeight > 42`) kapsayacak şekilde güçlendirildi.
  - `TypingIndicator` boşluk hatası giderilerek mesaj listesi ile input barı arasındaki 20px gereksiz boşluk kaldırıldı.
- **Tüm Platformlar İçin Kurulum Paketleri & GitHub v0.0.1 Güncellemesi**:
  - Windows Setup EXE (`veilanon_0.0.1_x64-setup.exe`), Windows MSI (`veilanon_0.0.1_x64_en-US.msi`), Windows ZIP (`veilanon_0.0.1_x64.zip`), Windows Tar.gz (`veilanon_0.0.1_x64.tar.gz`), Linux AppImage (`veilanon_0.0.1_amd64.AppImage`), Linux DEB (`veilanon_0.0.1_amd64.deb`), Linux RPM (`veilanon-0.0.1-1.x86_64.rpm`), macOS DMG (`veilanon_0.0.1_aarch64.dmg`), macOS App Tar.gz (`veilanon_0.0.1_aarch64.app.tar.gz`, `veilanon.app.tar.gz`) ve doğrulanmış `SHA256SUMS.txt` taze olarak derlenip paketlendi ve GitHub Release `v0.0.1` varlıklarına yüklendi.
- **Test ve Tip Doğrulama**:
  - 192/192 E2E testi, 162/162 Kamera & Efekt testi, SvelteCheck ve Rust derlemesi %100 başarıyla doğrulandı.

## Tamamlanan (v0.0.1 — E2EE Key Sync, Kesintisiz Ses/Görüntü, Topluluk & Çoklu Platform Release Turu)

- **E2EE Mesajlaşma & Anahtar Senkronizasyonu Kusursuzlaştırıldı**:
  - `[Şifreli mesaj — anahtar senkronizasyonu bekleniyor]` hatası tamamen giderildi.
  - Double Ratchet `RatchetState::encrypt_with_key` ile türetilen mesaj anahtarının yerel SQLite'a (`save_message_key`) anında kaydedilmesi sağlandı. Gönderen kendi mesajını yeniden yüklediğinde şifre çözme hatası oluşması engellendi.
  - `decrypt_batch` ve `decrypt_message_content` içinde çok kademeli fallback mimarisi (MLS/Ratchet -> Deterministik Kanal Anahtarı -> db_key) devreye alındı.
  - `sync_messages` içinde sunucu kanalları için varsayılan `is_e2ee` değeri `false` olarak düzeltildi.
- **Ses Kanalları ve DM Arama UX & Mantık Devrimi**:
  - Sunucu ses kanallarında yanlışlıkla gösterilen "aranıyor / kabul bekleniyor" engelleri tamamen kaldırıldı. Sunucu ses kanalları doğrudan, anında ve serbest katılımlı lobi yapısına kavuşturuldu.
  - DM sesli/görüntülü aramalarında odaya anında bağlanılırken karşı tarafa arkaplanda non-blocking çağrı daveti (`call_invite`) ve ringing bildirimi iletildi.
  - Ses kanalındaki katılımcıların anlık görünürlüğü ve avatar halkası senkronizasyonu optimize edildi.
- **Kamera Açma/Kapama ve Cihaz Geçiş Güvenliği ("Kamera Değiştirilemedi" Çözümü)**:
  - LiveKit `room.switchActiveDevice` kapalı track üzerindeyken hata fırlatmasını önlemek için track durum kontrolü eklendi; ayarlar menüsünden kamera seçimi artık hata vermez.
  - `toggleCamera` donanım ve çözünürlük kısıtları (1080p -> 720p -> 480p -> OS Default) için esnek fallback ile güçlendirildi.
- **Topluluk Katılımı ve Arayüz Gezinimi Onarımı**:
  - `CreateSpace.svelte` içinde `spaceStore.joinPublic` entegre edilerek katılım sonrası doğrudan `#genel` kanalının açılması sağlandı.
  - `spaces_join_public` sunucu kanalları için Supabase `channel_members` ve yerel SQLite kayıtlarını anında oluşturacak şekilde güncellendi.
- **Supabase Realtime ve Canlı Profil Senkronizasyonu**:
  - `get_user_profile` içinde bio, custom status ve banner alanları tek seferde güncellenecek şekilde optimize edildi.
- **Tüm Platformlar İçin Kurulum Paketleri & GitHub v0.0.1 Release Güncellemesi**:
  - Windows MSI (`veilanon_0.0.1_x64_en-US.msi`), Windows Setup EXE (`veilanon_0.0.1_x64-setup.exe`), Windows Zip (`veilanon_0.0.1_x64.zip`), Windows Tar.gz (`veilanon_0.0.1_x64.tar.gz`), macOS DMG (`veilanon_0.0.1_aarch64.dmg`), macOS App Tar.gz (`veilanon_0.0.1_aarch64.app.tar.gz`), Linux DEB (`veilanon_0.0.1_amd64.deb`), Linux AppImage (`veilanon_0.0.1_amd64.AppImage`), Linux RPM (`veilanon-0.0.1-1.x86_64.rpm`) ve doğrulanmış `SHA256SUMS.txt` derlenip GitHub Release `v0.0.1` varlıklarına yüklendi.
  - 113/113 Rust birim testi, Svelte 5 tip denetimi ve üretim derlemeleri %100 başarıyla tamamlandı.

## Tamamlanan (v0.0.1 — önceki doğrulama & release yenileme turu)

- **Kapsamlı kod incelemesi tamamlandı** — kullanıcı bildirilen tüm hataların koddaki durumu doğrulandı:
  - DM isim flicker'ı → `setDmChannels` placeholder/merge koruması (UUID/kendi nick asla gösterilmez).
  - Çevrimdışı arkadaş / "arkadaş bekleniyor" flicker'ı → `safeDoLoad` + loadVersion concurrency kilidi + 45sn presence poll.
  - Topluluk katılımı "local database error" → `spaces_join_public` / `invites_redeem` FK-onarım (spaces satırı eksikse önce insert, sonra membership; açık hata mesajı).
  - Hayalet mesaj bildirimi → `syncChannel` içerik+ek yoksa bildirim engeli.
  - Mesajların kaybolması / sync gelmemesi → optimistic merge + `sync_messages` deleted_at filtresi + clear gate + tombstone pası.
  - "Sohbet geçmişini temizle" → `clear_channel_messages` hem local hem uzak `deleted_at` tombstone; sync gate ile geri gelmez.
  - Kamera açık ama karşıya görünmüyor / ekran paylaşımı siyah → LiveKit track publish teyidi (`syncLocalState` her katılımda) + WebView2 SCREEN_CAPTURE otomatik izni + streamer mode ekran paylaşımı muafiyeti.
  - Profil avatar/banner yavaş sync → `updateIdentity` + `refreshRemoteProfile` + Supabase Realtime ile anlık yayın.
  - Arkaplan parlaklık slider'ı → `setBgBrightness` (0.2–2.5) + `--veil-bg-brightness` / `filter: brightness()` uygulaması.
  - GIF önizleme aç-kapa bozması → `shouldSuppressPreview` (tenor/giphy/.gif) ile link önizleme kartı GIF'ler için tamamen devre dışı.
  - Durum badge / E2EE rozet / bildirim popup'ları → premium glass tasarım; toast stack input bar'ın üstünde (`bottom: 132px`).
  - Ses kanalı üyeleri görünürlüğü → `broadcast_voice_state` + `request_voice_presence` presence yayını.
  - Çağrı bekleniyor durumu → `VideoCall` "Aranıyor…" sahnesi + incoming ring (`call_invite` broadcast + toast + desktop bildirimi).
- **Doğrulama kanıtları** — `npm run check` (0 hata, 0 uyarı), frontend üretim build ✓, Tauri Windows release build ✓ (MSI + NSIS), Rust birim testleri 113/113 ✓, Supabase REST bağlantısı ✓ (users tablosu erişilebilir; spaces/channels kullanıcı JWT'siyle RLS üzerinden erişilir — bare anon key'in 401'i beklenendir).
- **Yerel release klasörü yenilendi** — taze Windows `veilanon_0.0.1_x64-setup.exe`, `veilanon_0.0.1_x64_en-US.msi`, `veilanon_0.0.1_x64.zip`, `veilanon_0.0.1_x64.tar.gz` + `SHA256SUMS.txt`. macOS/Linux paketleri (.dmg/.app.tar.gz/.deb/.rpm/.AppImage) CI üzerinde güncellenir.
- **GitHub v0.0.1 release yenilemesi** — main'e push, `build.yml`'in `release` job'unu tetikler; 3 platformda taze derleme yapılır ve tüm bundle'lar `gh release upload --clobber` ile v0.0.1'e yüklenir; `finalize-release` birleşik `SHA256SUMS.txt` üretir.

## Tamamlanan (v0.0.1 — Bildirim Merkezi, Medya, Profil & UI Mükemmelleştirme)

- **Gelişmiş Bildirim Merkezi (Notification Center & Inbox)**:
  - Tümü / Bahsetmeler (@) / İstekler & Davetler / Sistem sekmeli Bildirim Merkezi dropdown'ı (`NotificationCenter.svelte`).
  - Rozet (Unread badge) sayaçları, tek tıkla "Tümünü Oku", bildirim geçmişini yerel saklama (`localStorage`) ve silme.
  - Bildirime tıklayarak doğrudan kanala, DM'ye, arkadaşlık isteğine veya topluluk davetine yönlendirme.
- **Kapsamlı Bildirim & Ses Özelleştirme Ayarları**:
  - `AppSettings` içinde `notification_volume`, `sound_messages`, `sound_mentions`, `sound_friends`, `sound_calls`, `dnd_suppress_notifications`.
  - %0 - %100 arası Genel Bildirim Ses Seviyesi kaydırıcısı.
  - Her bir ses kategorisi için bağımsız toggle ve anlık dinleme ("Dinle" butonu) özelliği.
  - Rahatsız Etmeyin (DND) ve Görünmez durumlarında ses ve masaüstü bildirimlerini otomatik bastırma.
- **Medya, Canlı Cihaz Geçişi & Bas-Konuş (PTT)**:
  - LiveKit `room.switchActiveDevice` ile arama esnasında mikrofon, hoparlör ve kamera değiştirildiğinde anında kesintisiz geçiş.
  - Global `keydown` / `keyup` pencereler arası tuş yakalama ile bas-konuş (Push-to-Talk) basılı tutma / bırakma akışı.
  - Konuşan göstergesi animasyonu (`isSpeaking`), ses seviyesi karıştırıcısı ve ekran paylaşımı çözünürlük/FPS modları.
- **Profil Banner Kırpma & GIF/Emoji Seçici**:
  - 3:1 formatında Banner Kırpma Modalı (`BannerCropModal.svelte`), fareyle sürükleme, tekerlek/kaydırıcı ile yakınlaştırma, avatar önizlemesi ve 1200x400 yüksek çözünürlüklü canvas çıktısı.
  - Tenor & Giphy entegrasyonu, trend ve arama GIF'leri, favoriler ve kategorize edilmiş emoji seçici.
- **Mesajlaşma & Etkileşim UI/UX**:
  - `MessageInput.svelte`: Yukarı ok (↑) ile son kendi mesajını düzenleme, Esc ile yanıt veya emoji seçiciyi iptal etme, dosya sürükle-bırak ve yapıştırma.
  - `MessageItem.svelte`: Hızlı ve özel emoji reaksiyonları, mesaj kopyalama / bağlantı kopyalama, sabitleme (pin), markdown ve medya önizleme.
- **Sosyal, Arkadaşlık & Topluluklar**:
  - Sekmeli Arkadaş Listesi (Çevrimiçi, Tümü, Bekleyen İstekler, Engellenenler, Arkadaş Ekle).
  - Profil kartı, doğrudan DM başlatma, arkadaş ekleme/çıkarma, engelleme ve topluluk oluşturma / davet bağlama.

## Tamamlanan (bu dönem)

- **DM E2EE — Double Ratchet bağlandı**: `send_message`/`load_messages`/`sync_messages`/`search_messages`/`get_pinned_messages` 1:1 DM kanallarında `RatchetState` kullanıyor (dm_sessions + şifreli message-key cache ile geçmiş yeniden okunabilir); root key kimlik DH'sinden türetiliyor (X3DH sadeleştirilmiş); `dm_open` idempotent DM kanalı açıyor.
- **Realtime 400 düzeltmesi**: WS URL'sine `/realtime/v1/websocket` eklendi; kanal join logları + `veilanon:realtime-status`/`veilanon:broadcast` event'leri; typing artık gerçek broadcast ile iletilip 5 sn'de expire ediyor.
- **Kilit açma kilitlenmesi giderildi**: `bind_control_plane` ve `create_identity` içindeki tokio RwLock deadlock'u (read guard varken write alma) düzeltildi; `local_identity` satırı eksikse keychain'den kararlı id ile yeniden kuruluyor; `get_identity_hint` keychain varken "yeni kimlik oluştur" sunmuyor.
- **Kurtarma kodu artık gösteriliyor**: IPC camelCase sözleşmesi (`recoveryCode`) düzeltildi; kimlik oluşturma sonrası kod ekranda çıkıyor; kurtarma ile yeni parola akışı uçtan uca test edildi.
- **Ayarlar gerçekten kaydediliyor** (settings.json persist doğrulandı); Ses & Görüntü açılışta getUserMedia istemiyor (webview izin popup'ı kaldırıldı); cihaz listesi enumerateDevices ile popupsız.
- **Modallar düzeltildi**: `Modal` `open` prop'u AppLayout'a verildi — Alan Oluştur/Davet/Profil/Alan Ayarları/Rol editörü artık açılıyor.
- **Dosya akışı mesajlara bağlandı**: ekle/drag-drop → şifreli yükleme → attachment çipi → indir+çöz; emoji seçici; SVG gönder ikonu.
- **Ana menü**: Arkadaşlar / Direkt Mesajlar / Alanlar sekmeli modern Home; davet koduyla katılım; profil menüsünde ayarlar/çıkış.
- **Sunucu şeması**: channel_members + messages.crypto_meta + nullable channels.space_id + DM RLS + users INSERT policy (supabase db push ile canlıya gönderildi); channels payload'ındaki geçersiz owner_id kaldırıldı.
- **Log sistemi**: `veilanon=debug` varsayılanı, `get_diagnostics` + `get_log_directory` komutları, Hakkında'da tanılama paneli, build.bat locale-bağımsız zaman damgası.

## Tamamlanan (bu tur)

- **IPC sözleşme düzeltmesi** — tüm input/response struct'larında `camelCase` serde; `create_identity`, mesaj, medya, dosya ve sosyal komutlarında kırık argüman eşleşmesi giderildi.
- **Mesaj kripto determinizmi** — `derive_message_key(db_key, message_id)` (HKDF-SHA256); `send_message` artık anahtarı çöpe atmıyor; `load_messages` decrypt edip geçmişi döndürüyor.
- **Mesaj komutları** — `edit_message`, `add_reaction`, `remove_reaction`, `search_messages` gerçek; `get_pinned_messages` decrypt ediyor.
- **Offline queue** — `get_pending_queue` gerçek; `send_message` ağ başarısızsa `enqueue_message` ile kuyruğa alıyor (best-effort).
- **Veri taşınabilirliği** — `export_data` / `import_data`: `VEILANON_EXPORT_1.` şifreli arşiv (AES-256-GCM, db_key).
- **Kimlik** — `update_profile` (görünen ad / avatar hash), `get_identity_hint`, brute-force rate limit.
- **Ayarlar** — kaydetme sözleşmesi düzeltildi (notificationPreview enum, presenceVisibility, full-object save), accent rengi, gerçek cihaz listesi (getUserMedia izni), gürültü/yankı/PTT AppSettings'e taşındı.
- **UI/UX** — premium onboarding (SVG ikon seti, step geçişleri), profil + durum menüsü, modern menü/tooltip, dostane Hakkında linkleri.
- **WebView sertleştirme** — sağ tık, F12/dev tools, `window.open`, webview içi dış link yönlendirmesi engellendi.

## Tamamlanan (bu tur)

- **Gerçek kurtarma akışı** — keychain bundle v2 (master key + çift sarma: parola ve kurtarma kodu); `recover_identity` komutu kurtarma kodu + yeni parola ile kimliği açar ve yeniden sarmalar; v1 bundle'lar geriye dönük okunur. Kurtarma kodu artık parola unutulunca gerçekten çalışıyor (v1 bundle'larda entropy parola sarmasının içinde kilitli olduğu için recovery imkânsızdı).
- **Kilit açma düzeltmesi** — `load_identity` artık `state.identity`'yi dolduruyor (önceden tüm sonraki komutlar `Unauthenticated` dönüyordu); onboarding gate unlock sonrası `recoveryAcknowledged` bayrağını set ediyor — uygulama artık kilit açınca açılıyor.
- **Kimlik koruması** — `create_identity` mevcut kimlik varken `IdentityExists` döndürüyor; onboarding "Yeni Kimlik Oluştur"u gizliyor.
- **WebView popup'sız** — `alert/confirm/prompt` tamamen özel UI'ya (ConfirmDialog/InputDialog) yönlendirildi; autofill/kayıtlı parola görünümü bastırıldı; tüm input'larda `autocomplete="off"`.
- **Kontrol düzlemi bağlantısı** — kimlik oluşturma/yükleme/kurtarma sonrası `sign_in_anonymous` + access token akışı; `bind_control_plane` best-effort.
- **Realtime** — Supabase Realtime WebSocket (Phoenix protocol): postgres_changes/presence/broadcast kanalları, heartbeat, exponential backoff yeniden bağlanma; gelen ciphertext satırları `veilanon:realtime-message` event'i ile UI'a iletilir.
- **Sync** — `sync_messages` komutu: uzak ciphertext satırlarını çeker, sender device → user çözümler, local DB'ye idempotent birleştirir, decrypt edilmiş halde döndürür; frontend realtime event'inde kanalı otomatik senkronlar.
- **Offline queue flush** — arka plan görevi her 20 sn'de kuyruğu Supabase `messages` tablosuna boşaltır (exponential backoff ile), başarılı olanları `sent` işaretler.
- **Dosya akışı** — `upload_file` artık ciphertext'i Supabase Storage'a yüklüyor (`files/{channel}/{id}.bin`), içerik anahtarını db_key ile sarıp yerel `file_metadata` tablosuna yazıyor; `download_file` blob'u indirip çözüyor; `delete_file`/`get_file_info` gerçek.
- **Arkadaşlık / presence / alan ağı** — `friendships` tablosu + RLS; `friends_add`/`friends_accept`/`presence_update` best-effort kontrol düzlemine yazıyor; `users`/`devices` public-key registry okuma politikaları eklendi; `spaces_create` şemayla uyumlu payload + membership kaydı.
- **UI/UX turu** — emoji'ler tamamen SVG ikon setiyle değiştirildi (hash/volume/megaphone/lock/edit/pin/phone-off/camera/check-double…); kanal listesi/sidebar tekilleştirildi (çift "Ayarlar" butonu kaldırıldı); presence `uiStore`'a taşındı; ayarlar nav bölümlü + eşit yükseklikli; tema kartları mini önizlemeli, vurgu rengi swatch'ları modern ring'li; home hero SVG.
- **Log sistemi** — build.bat çıktıları `Tee-Object` ile log dosyasına yazılıyor; frontend console (debug/info/warn/error) dosya loguna köprüleniyor; `log_client_error` PII maskeleme (e-posta) ekledi; uygulama başlangıcında log dizini yazdırılıyor.

## Tamamlanan (bu tur — 2026-08-16 kapsamlı tur)

- **Sonsuz loading kök nedeni giderildi** — `bind_control_plane`/`create_identity` ağ çağrıları 8-10 sn zaman aşımıyla sarıldı (network yazma kilidi asla süresiz tutulmuyor); `list_sessions` kilit edinimi dahil tüm isteği 7 sn'de zaman aşıyor; frontend retry'ı da zaman aşımlı — Oturumlar/Cihazlar ve Topluluk Ayarları artık asla aç kalmıyor.
- **Ekran paylaşımı izni** — WebView2 permission handler artık ekran yakalamayı (SCREEN_CAPTURE/WITH_AUDIO/WITHOUT_AUDIO) otomatik onaylıyor; mikrofon/kamera izinleri uygulama orijinleri için profil düzeyinde kalıcı tanımlandı — izin penceresi hiç görünmüyor.
- **Log klasörü** — `open_log_folder` komutu (explorer/open/xdg-open) eklendi; `revealItemInDir`/file:// fallback'leri artık ikincil.
- **Hakkında bilgileri** — build.rs gerçek derleme tarihi (`VEILANON_BUILD_DATE`) ve `RUSTC_VERSION` üretiyor; "Platform" ve "Rust" satırları dolu.
- **Davet süresi hatası** — `expiresAt` artık unix saniye (i64) gönderiliyor (ISO string serde hatasıyla daveti bozuyordu); davet linki `veilanon.com/invite/{kod}` tam link olarak kopyalanıyor.
- **Ses kanalı katılımcıları** — bağlı ses kanalının altında üyeler listeleniyor; konuşanın avatarında tema renkli halka (`Avatar speaking`), sessizde kırmızı mikrofon ikonu.
- **Oturum satırı** — cihaz adı + "(bu cihaz)" + platform rozeti; `formatRelativeTime` NaN-güvenli ("Invalid Date" yok).
- **Görünüm başlangıcı** — kayıtlı yazı boyutu/kompakt mod/azaltılmış hareket açılışta uygulanıyor.
- **Görüntülü arama düzeni** — paylaşılan ekran + katılımcı ızgarası grid'e çevrildi (sıkışma yok); video overlay `min-height:0`.
- **Menü modernizasyonu** — durum/context/upload menüleri blur + shadow-xl + radius-xl; toggle focus/aktif durumları; emoji seçici genişletildi (fav yıldızı daha büyük).
- **Temizlik** — kullanılmayan `svelte.svg`/`tauri.svg`/`vite.svg` kaldırıldı; sidebar logosu yumuşak köşeli + büyütüldü.

## Tamamlanan (bu tur — moderasyon + altyapı)

- **Kick / ban / timeout — tam stack** — backend komutları: `spaces_kick_member` (sahip veya `kick_members` izni), `spaces_ban_member` (`ban_members`; üyeyi siler + ban listesine ekler, davetle bile geri dönemez), `spaces_unban_member`, `spaces_timeout_member` (`timeout_members`; unix saniye, 0 = kaldır), `spaces_bans_list`. Davet kabulünde ban kontrolü; `send_message` susturma (timeout) kontrolü. Frontend: üye listesinde sağ tık menüsü (profil/DM/arkadaş ekle/sustur/at/yasakla — sahip için), Topluluk Ayarları'nda üye yönetim butonları + yasaklılar listesi (yasağı kaldır).
- **Migration 0009 (local) + 20260817030000_moderation (Supabase, `supabase db push` ile canlıda)** — `banned_members`/`bans` tabloları, `space_members.timeout_until`/`memberships.timeout_until`, owner-only RLS politikaları.
- **Doğrudan indirme (website)** — indirme butonları GitHub API'den son release asset'lerini çekip aracı sayfa olmadan doğrudan dosyaya bağlıyor (`.msi`, `-setup.exe`, `.dmg`, `.AppImage`, `.deb`); release yokken releases/latest fallback'i.
- **Temiz release build** — `veilanon_0.0.1_x64-setup.exe` (NSIS) + `veilanon_0.0.1_x64_en-US.msi` (WiX) üretildi ve doğrulandı.
- **Davet linki formatı** — üretim `veilanon.com/invite?code=KOD` (Vercel'de garantili çalışır); kabul hem `/invite/CODE` hem `?code=` biçimini ayıklar; site davet sayfası her iki biçimi okur.

## Tamamlanan (bu tur — deep link + paylaşım)

- **Deep link protokolü `veilanon://`** — `tauri-plugin-deep-link` + `tauri-plugin-single-instance` (deep-link feature): Windows/Linux'ta CLI argümanıyla gelen URL'ler tek örneğe `deep-link://new-url` event'iyle iletilir; macOS'ta plugin event'i; ilk örnek başlangıcındaki URL kısa gecikmeyle emit edilir (dinleyici hazır olsun). Frontend `onOpenUrl` ile dinler; oturum kapalıysa URL kuyruğa alınır, açılışta işlenir.
- **Desteklenen derin bağlantılar** — `veilanon://invite/CODE` (davet kabulü), `veilanon://u/USERNAME` (profil — yeni `resolve_username` komutu), `veilanon://server/SPACE_ID`, `veilanon://channel/CHANNEL_ID`, `veilanon://message/CH/ID`. Site `https://veilanon.com` aynı yolları paylaşır.
- **Paylaşılabilir linkler** — kanal sağ tık menüsünde "Kanal bağlantısını kopyala" (+ ses kanalı için katıl/ayrıl), Topluluk Ayarları'nda "Topluluk bağlantısını kopyala", profil modalında `veilanon.com/u/<ad>` paylaşımı; mesaj bağlantısı mevcut.
- **Site profil sayfaları** — `https://veilanon.com/u/<kullanıcı>` (Vercel rewrite) "veilanon'da Aç" derin bağlantı butonuyla; davet sayfasında "veilanon'da Aç" birincil buton + kod kopyalama + indirme.

- **v0.0.1 GitHub Release yayında** — CI/CD release otomasyonu ve yerel derleme boru hattı ile 14 adet çoklu platform kurulum paketi ve doğrulama dosyası yayınlandı. Asset'ler: Windows `veilanon_0.0.1_x64-setup.exe` (NSIS) + `veilanon_0.0.1_x64_en-US.msi` (WiX) + `veilanon-setup.exe` + `.zip` + `.tar.gz`, macOS `veilanon_0.0.1_aarch64.dmg` (+ `.app.tar.gz`), Linux `veilanon_0.0.1_amd64.deb` + `veilanon_0.0.1_amd64.AppImage` + `veilanon-0.0.1-1.x86_64.rpm`, `SHA256SUMS.txt` manifesti ve `veilanon-ca.cer` sertifikası.
- **Kamera & Donanım Yaşam Döngüsü İyileştirmesi** — Kamera kapatıldığında / görüşmeden ayrılındığında `MediaStreamTrack.stop()` ile fiziksel web kamerası sensörünün kilitli kalması ve ışığının açık kalması engellendi (`stopLocalTrackOnUnpublish: true`); `ParticipantTile.svelte` içinde Svelte 5 reaktif `$effect` ve polling yeniden deneme mekanizması ile video stream gecikmesiz ve kusursuz olarak `<video>` elemanına bağlandı.

- **Mor vurgu teması** — tüm brand token'ları ve hardcoded mavi (hsl 230) referansları mora (hsl 262) çevrildi; özel hex renk girişi + hazır template swatch'lar + doğal renk seçici eklendi.
- **Sol panel yeniden düzenlendi** — çift profil kaldırıldı (Sidebar artık yalnızca logo + topluluklar); tek kullanıcı paneli (avatar + ad + durum + mikrofon + kulaklık/deafen + ayarlar) kanal listesinin altında; durum noktası klip hatası giderildi.
- **WebView izinleri ve autofill** — mikrofon/kamera izinleri uygulama tarafında otomatik onaylanıyor (WebView2 PermissionRequested handler); "kaydedilmiş bilgiler" autofill ve parola kaydı tamamen kapatıldı; log klasörü açma `revealItemInDir` ile düzeltildi.
- **Toggle düzeltmesi** — `for`/`id` eşleşmesi bozuk olduğu için hiç çalışmayan tüm anahtarlar artık çalışıyor.
- **Kanal adı UUID hatası** — kanal başlangıcı/başlık/ses çubuğu artık UUID yerine gerçek kanal adını gösteriyor (kanal listesi otomatik yükleniyor).
- **Üye listesi** — placeholder yerine gerçek uygulama: çevrimiçi/çevrimdışı gruplu, rol renkleri, tıklayınca profil.
- **Cihaz/session düzeltmeleri** — `DeviceInfo` gerçek cihaz adı + OS döndürüyor; `lastActiveAt` sayısal olduğu için "Invalid date" giderildi; oturum adları gerçek makine adı.
- **Emoji + GIF** — kategorili, aramalı, favorili gelişmiş emoji seçici; Tenor/Giphy üzerinden gerçek GIF arama/trend + favori GIF'ler (anahtarlar Rust tarafında).
- **Dosya + butonu** — Dosya/Fotoğraf/Video/Ses menüsü; gönder tuşu yanındaki emoji/GIF seçici yeniden tasarlandı.
- **Mesaj işlemleri** — hover menüsü + sağ tık context menüsü: kopyala, bağlantı kopyala, yanıtla, tepki ekle (hızlı + özel), sabitle, düzenle, sil; yanıt çubuğu input üzerinde; tepkiler tıklanınca ekle/kaldır.
- **Header 3 buton** — arama (gerçek local arama modalı), sabitlenmiş mesajlar modalı, üye listesi toggle — hepsi çalışıyor.
- **Ses/Görüntü** — video overlay üst üste binme düzeltildi; kamera toggle hata yakalamalı; kulaklık (deafen) eklendi; bas-konuş (tuş yakalama + aramada basılı tut) çalışıyor; mikrofon testi canlı seviye çubuğuyla başlat/durdur.
- **Rol kaydetme** — izin formatı uyuşmazlığı giderildi (string[] ↔ bitfield) — rol oluşturma/güncelleme çalışıyor.
- **Topluluk** — "Alan" terminolojisi "Topluluk" oldu; topluluk oluşturulunca otomatik `#genel` + `Genel` ses kanalı; ayarlar çark ikonu düzeltildi; davet linkleri `veilanon.com/invite/{kod}` formatında kopyalanıyor, URL yapıştırınca kabul ediliyor.
- **Profil** — avatar yükleme/kaldırma (yerel, IPC ile), "Hakkımda" (şifreli saklanır), zengin profil modalı (durum, bio, DM/arkadaşlık/engelle, profil linki).
- **Kullanıcı adı benzersizliği** — kayıtta sunucu kontrolü + Supabase `citext unique` indeksi.
- **Local AI** — Ollama üzerinden gerçek bağlantı (durum kontrolü + test), sahte toggle yok.
- **Discord köprüsü** — OAuth2 stub yerine resmî webhook tabanlı gerçek uygulama: kanal başına webhook, "[köprü]" etiketli yansıtma, gizlilik uyarısı.
- **Türkçe** — varsayılan dil `tr`; eksik çeviriler tamamlandı; boş durum mesajları komik/dostane.

## Tamamlanan (bu tur — kamera efekt sistemi)

- **Efekt Motoru (Effect Engine)** — MediaPipe Face Mesh + Hands + Pose WASM entegrasyonu; Web Worker'da landmark algılama, ana thread'de Canvas 2D render pipeline; 1-euro filtresi ile landmark yumuşatma; 30+ FPS hedefi.
- **15 Hazir Efekt** — Soft Blur Face, Neon Outline, Anime Eyes, Cat Ears + Whiskers, Sunglasses, Face Paint (warrior/butterfly/hearts), Particle Hands, Laser Fingers, Magic Trail, Skeleton Overlay, Energy Aura, Gesture Trigger (peace/thumbsUp/fist), Mirror Face, Glitch Face, Custom (plugin placeholder). Hepsi moduler `Effect` interface'i ile.
- **Plugin Sistemi** — Kullanicilar kendi .js scriptlerini yukleyebilir; sandbox validasyonu (yasakli API'ler: network, filesystem, eval, subprocess); script hash integrity; localStorage registry + Supabase sync; paylasim secenegi (sadece ben / herkes gorson).
- **Gorunurluk Modu** — "Sadece Ben" (local render) / "Herkes" (LiveKit DataChannel ile parametre + landmark sync); piksel asla sunucuya gitmez.
- **Efekt Paneli UI** — Sagdan slide panel; kategori filtreleri (Yuz/El/Vucut/Jest/Ozel); 3 sutunlu grid thumbnail; aktif efekt uzerinde ince glow; parametre slider'lari (renk, yogunluk, boyut); plugin ekle/kaldir yonetimi.
- **Efekt Butonu** — VoiceBar ve VideoCall'da sparkle ikonu ile efekt butonu; kamera kapaliyken tiklandiginda "Efekt kullanmak icin kamerayi acman gerekiyor" uyarisi; kamera acildiginda otomatik efekt restore.
- **EffectsCanvas** — ParticipantTile icinde video uzerine Canvas overlay; sadece aktif efekt ve kamera acikken render eder; inherit border-radius ile tile sekline uyum.

## Sıradaki büyük mimari işler (deferred — kapsam dışı)

Bu işlerin hiçbiri bu dönem implemente edilmedi. Her biri için "tamamlanma" ölçütü ayrıca tanımlanmıştır.

### 1. DM E2EE — Double Ratchet bağlama

- **Durum:** ✅ **Tamamlandı** — 1:1 DM'ler `dm_sessions` üzerinden Double Ratchet ile şifreleniyor; message-key cache ile geçmiş okuma; karşılıklı cihaz senkronu için `crypto_meta` sunucu şemasında.
- **Kalan:** çoklu cihaz X3DH ön-anahtar rotasyonu (oturum kurulduktan sonra ratchet devralır; cihaz silinirse yeni iletiler eski cihazda açılamaz ilkesi için anahtar rotasyon testi).

### 2. Grup E2EE — MLS bağlama

- **Durum:** ✅ **Bağlandı** — E2EE metin/ses kanalları `mls_sessions` (DB anahtarıyla şifreli) üzerinden MLS ile şifreleniyor; `send_message`/`edit_message`/`load_messages`/`sync_messages` MLS yolunu kullanıyor. Üye katılımı: üye kendi KeyPackage'ini üretir → sahibe iletir → sahip Welcome'ı üretenin X25519 anahtarıyla şifreler (yerel + Supabase `mls_welcomes`) → üye Welcome'ı çözüp oturuma katılır.
- **Kalan:** çoklu cihazda welcome teslimatının canlı senkron testi; üye çıkarma akışının UI'a bağlanması (`remove_member` hazır).

### 3. Medya E2EE

- **Durum:** ✅ **Uygulandı (E2EE kanallar için)** — E2EE ses kanalında oda anahtarı MLS export secret'inden türetilir ve yalnızca IPC üzerinden istemciye verilir; LiveKit frame cryptor (E2EEOptions + worker) ile uçtan uca şifreleme aktif; E2EE olmayan kanallarda transport şifreleme etiketli.
- **Kalan:** anahtar dönüşünün üye değişiminde otomatik tetiklenmesi; E2EE olmayan kanallar için medya E2EE (ürün kararı).

### 4. Kaybolan mesaj temizleyici & UI Sayaç Göstergesi

- **Durum:** ✅ **Tamamlandı** — `purge_expired` her 20 sn'de çalışır (lib.rs arka plan döngüsü); `disappears_at` geçmiş mesajlar hem local DB'den hem uzak tablodan tombstone'lanır. `MessageItem.svelte` içinde gerçek zamanlı 1 saniyelik tik sayacı ve renk kodlu süre rozeti (yeşil / sarı / kırmızı) ile `MessageInput.svelte` süre seçicisi tamamen aktiftir.

### 5. Sıfır-Bilgi Güvenlik & Gizlilik Araçları (Zero-Key Public Privacy Hub)

- **Durum:** ✅ **Tamamlandı** — 
  - **Tor & Relay Anonymity Check**: `check_tor_status` ile Tor çıkış düğümü tespiti.
  - **Multi-DoH Benchmark & Sansür Tespiti**: `check_multi_doh_status` ile Cloudflare, Google, Quad9, AdGuard ve Mullvad DoH sağlayıcılarında gerçek zamanlı gecikme ölçümü ve DNS manipülasyon/sansür uyarısı.
  - **k-Anonymity Parola Sızıntı Denetimi**: `check_password_pwned` ile SHA-1 5-karakter prefix modeli üzerinden parola gönderilmeden HIBP sızıntı denetimi.
  - **URLhaus Canlı Tehdit Tarayıcısı**: `scan_urlhaus` ile Abuse.ch veritabanından kötü amaçlı bağlantı ve zararlı yazılım tespiti.
  - **SSRF-Korumalı Güvenli Bağlantı Önizleme**: `fetch_link_preview` ile döngüsel/yerel IP'leri engelleyen OpenGraph meta veri önizleme kartı.
  - **Deterministik SVG Avatar Üreteci**: `generate_privacy_avatar` ile harici ağ isteği olmadan çevrimdışı SVG identicon oluşturma.
  - **Kriptografik Zaman Senkronizasyonu (Clock Skew)**: `detect_clock_skew` ile sistem saati sapması tespiti.

### 5. Dosya dışı kalanlar

- **`sign_message`** — ✅ çalışıyor (`device_identity.sign` — oturum açıkken Ed25519 imzası).
- **`list_sessions`/`revoke_session`** — ✅ gerçek: `devices` tablosundan okur/siler (Supabase).
- **Discord köprüsü** — ✅ webhook tabanlı gerçek uygulama (`discord_set/clear/get_webhook` + `mirror_message`); OAuth2 uygulama akışı için Discord uygulama kimlikleri gerektiğinde eklenir.
- **Local AI** — ✅ Ollama (`/api/tags`, `/api/chat`) üzerinden çalışıyor; gizlilik ayarlarında durum + test.
- **GIF arama** — ✅ Tenor/Giphy (anahtar `.env`'den); anahtar yoksa net hata.
- **kullanıcı arama, kick/ban/timeout** — ürün katmanı işleri; komut imzaları PRD'de tanımlı, IPC yüzeyi hazır.

### 6. Kontrol düzlemi senkronu (2026-08-16)

- **Durum:** ✅ **Düzeltildi** — `best_effort_insert` ve mesaj yükleme artık JSON null taşımıyor (400 hatası) ve plain insert kullanıyor (PostgREST `on_conflict` + RLS 403 tuzağı); space → membership → channel → message zinciri Supabase'e kaydediliyor. Anonim oturum refresh token'ı kalıcılaştırıldı (0007 migration) — her kilit açmada yeni kullanıcı oluşmuyor (MAU şişmesi çözüldü).
- **Kalan:** `R2/Upstash/Qdrant/Discord/Sentry` env anahtarları şu an kullanılmıyor (Supabase Storage + Realtime her şeyi karşılıyor); ilgili servisler eklendiğinde bağlanacak.
