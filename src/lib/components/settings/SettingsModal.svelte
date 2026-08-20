<script lang="ts">
  import { tick } from 'svelte';
  import { uiStore } from '$lib/stores/ui';
  import { createFocusTrap, handleEscape } from '$lib/utils/accessibility';
  import Icon, { type IconName } from '$lib/components/ui/Icon.svelte';
  import AccountSettings from './AccountSettings.svelte';
  import PrivacySettings from './PrivacySettings.svelte';
  import AppearanceSettings from './AppearanceSettings.svelte';
  import AudioVideoSettings from './AudioVideoSettings.svelte';
  import NotificationSettings from './NotificationSettings.svelte';
  import SecuritySettings from './SecuritySettings.svelte';
  import StreamerSettings from './StreamerSettings.svelte';
  import AboutSettings from './AboutSettings.svelte';

  const ui = $derived($uiStore);

  const tabs: Array<{ id: string; label: string; icon: IconName; keywords: string[] }> = [
    { id: 'account', label: 'Hesap', icon: 'user', keywords: ['hesap', 'profil', 'avatar', 'banner', 'ad', 'kullanıcı', 'username', 'görünür ad', 'kullanıcı adı', 'fotoğraf', 'bio', 'isim', 'ad soyad'] },
    { id: 'privacy', label: 'Gizlilik & Ağ', icon: 'globe-lock', keywords: ['gizlilik', 'ağ', 'tor', 'vpn', 'proxy', 'dns', 'şifreleme', 'kalkan', 'dm', 'mesajlaşma', 'varlık', 'çevrimiçi', 'okundu', 'bildirim', 'medya', 'indirme', 'bağlantı', 'tor', 'warp', 'wireguard', 'socks', 'dns leak', 'sızıntı', 'şifre', 'anahtar', 'oturum'] },
    { id: 'streamer', label: 'Yayıncı Modu', icon: 'broadcast', keywords: ['yayıncı', 'yayın', 'streamer', 'sansür', 'ekran paylaşımı', 'bulanıklaştırma', 'gizleme', 'canlı', 'live', 'stream', 'mask', 'maskeleme'] },
    { id: 'appearance', label: 'Görünüm', icon: 'sparkle', keywords: ['görünüm', 'tema', 'renk', 'yazı boyutu', 'amoled', 'kompakt', 'animasyon', 'vurgu', 'koyu', 'açık', 'sistem', 'başlatma', 'dark', 'light', 'font', 'boyut', 'rengi', 'accent', 'aksan'] },
    { id: 'audio-video', label: 'Ses & Görüntü', icon: 'mic', keywords: ['ses', 'görüntü', 'mikrofon', 'kamera', 'kulaklık', 'cihaz', 'bas-konuş', 'girişi', 'çıkışı', 'susturma', 'yankı', 'gürültü', 'audio', 'video', 'device', 'input', 'output', 'push to talk', 'ptt'] },
    { id: 'notifications', label: 'Bildirimler', icon: 'bell', keywords: ['bildirim', 'ses', 'masaüstü', 'mesaj', 'bahsetme', 'susturma', 'gece', 'notification', 'sound', 'alert', 'dnd', 'rahatsız etmeyin'] },
    { id: 'security', label: 'Güvenlik', icon: 'shield', keywords: ['güvenlik', 'parola', 'oturum', 'cihaz', 'şifreleme', 'kurtarma', 'otomatik', 'başlangıç', 'security', 'password', 'session', 'device', 'recovery', 'lock', 'kilit'] },
    { id: 'about', label: 'Hakkında', icon: 'info', keywords: ['hakkında', 'sürüm', 'lisans', 'versiyon', 'geliştirici', 'aegissoft', 'github', 'destek', 'about', 'version', 'license', 'developer', 'update', 'güncelleme'] },
  ];

  interface DeepSettingOption {
    tabId: string;
    tabLabel: string;
    tabIcon: IconName;
    title: string;
    description: string;
    keywords: string[];
  }

  const ALL_SUB_SETTINGS: DeepSettingOption[] = [
    // Account
    { tabId: 'account', tabLabel: 'Hesap', tabIcon: 'user', title: 'Görünür İsim & Kullanıcı Adı', description: 'Profilinizde diğer kullanıcılara görünen adınızı ve benzersiz @kullanıcı_adınızı belirleyin.', keywords: ['ad', 'isim', 'username', 'kullanıcı adı', 'display name', 'hesap', 'profil'] },
    { tabId: 'account', tabLabel: 'Hesap', tabIcon: 'user', title: 'Profil Fotoğrafı (Avatar)', description: 'Yerel cihazınızdan avatar yükleyin veya kaldırın.', keywords: ['avatar', 'resim', 'fotoğraf', 'profil resmi', 'görsel', 'foto'] },
    { tabId: 'account', tabLabel: 'Hesap', tabIcon: 'user', title: 'Profil Banner Görseli', description: 'Profil sayfanızın üst kısmında yer alan özel arka plan görselini belirleyin.', keywords: ['banner', 'afiş', 'kapak fotoğrafı', 'arka plan', 'resim'] },
    { tabId: 'account', tabLabel: 'Hesap', tabIcon: 'user', title: 'Hakkımda / Biyografi (Bio)', description: 'Uçtan uca şifreli olarak saklanan kişisel profil biyografiniz.', keywords: ['bio', 'hakkımda', 'biyografi', 'açıklama', 'metin'] },
    
    // Privacy & Network
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Tor Ağı Yönlendirmesi (.onion)', description: 'Tüm ağ trafiğini yerleşik Tor ağı ve vekil sunucu üzerinden gizli yönlendirir.', keywords: ['tor', 'onion', 'proxy', 'socks5', 'anonimlik', 'gizlilik', 'ağ', 'tünel'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'WireGuard / VPN Entegrasyonu', description: 'Özel WireGuard VPN yapılandırması ile tüm giden bağlantıları şifreleyin.', keywords: ['vpn', 'wireguard', 'tünel', 'ip gizleme', 'yapılandırma'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'DNS Sızıntı Koruması (DoH Benchmark)', description: 'Cloudflare, Google, Quad9 ve Mullvad DoH sunucularıyla DNS şifreleme ve sızıntı testi.', keywords: ['dns', 'doh', 'sızıntı', 'leak', 'benchmark', 'resolver'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Ağ ASN & IP Tespiti', description: 'Bağlı olduğunuz ISP operatörünü ve otonom sistem numarasını (ASN) analiz eder.', keywords: ['asn', 'ip', 'isp', 'operatör', 'bağlantı'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Saat Sapması (Clock Skew) Doğrulama', description: 'NTP sunucularıyla sistem saatinizin senkronizasyonunu ve şifreleme toleransını denetler.', keywords: ['saat', 'zaman', 'skew', 'ntp', 'sapma', 'senkron'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Direkt Mesaj (DM) Gizliliği', description: 'Kimlerin size özel mesaj gönderebileceğini belirleyin: Herkes, Ortak Sunucular, Arkadaşlar veya Hiçkimse.', keywords: ['dm', 'direkt mesaj', 'mesaj izni', 'kimler mesaj atabilir', 'özel mesaj', 'izin'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Katılım Tarihini Göster', description: 'Hesap açma tarihinizin diğer kullanıcılar tarafından profilde görülmesini sağlar.', keywords: ['katılım tarihi', 'hesap tarihi', 'join date', 'kayıt tarihi', 'profil'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Okundu Bilgisi (Read Receipts)', description: 'Mesajları okuduğunuzda karşı tarafa iletilen çift tik okundu bildirimi.', keywords: ['okundu', 'çift tik', 'görüldü', 'read receipts', 'tik'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Yazıyor Göstergesi (Typing Indicator)', description: 'Mesaj yazarken kanalda veya DM odasında yazıyor ibaresini gösterir.', keywords: ['yazıyor', 'yazma göstergesi', 'typing', 'yazı'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Otomatik Medya İndirme', description: 'Görsellerin ve ses dosyalarının şifresi çözülüp otomatik olarak yerel diske indirilmesi.', keywords: ['medya indirme', 'otomatik yükle', 'dosya kaydet', 'indirme'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Yerel Yapay Zeka (Ollama / Local AI)', description: 'Tamamen cihazınızda çalışan yerel dil modelleri ile güvenli mesaj analizi ve özetleme.', keywords: ['yapay zeka', 'ai', 'ollama', 'yerel', 'local ai', 'model'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Ekran Paylaşımı Kalkanı', description: 'Ekran paylaşımı sırasında hassas pencereleri ve uygulamaları otomatik olarak gizler.', keywords: ['ekran paylaşımı', 'kalkan', 'shield', 'gizleme', 'screen share'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Bağlantı Önizleme (Link Preview)', description: 'Mesajlardaki web bağlantıları için otomatik metadata önizlemesi oluşturur.', keywords: ['bağlantı', 'link preview', 'önizleme', 'url', 'metadata'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Bildirim Önizleme Gizliliği', description: 'Sistem bildirimlerinde mesaj içeriğinin görünüp görünmeyeceğini ayarlar.', keywords: ['bildirim', 'önizleme', 'notification preview', 'gizlilik', 'içerik'] },
    { tabId: 'privacy', tabLabel: 'Gizlilik & Ağ', tabIcon: 'globe-lock', title: 'Discord Köprüsü (Bridge)', description: 'Discord sunucularınızla webhook tabanlı köprü bağlantısı kurar.', keywords: ['discord', 'köprü', 'bridge', 'webhook', 'entegrasyon'] },

    // Streamer Mode
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Yayıncı Modu (Streamer Mode)', description: 'Ekran paylaşımı ve yayın esnasında hassas kimlik, kullanıcı ID ve bildirimleri otomatik sansürler.', keywords: ['yayıncı modu', 'streamer', 'sansür', 'ekran gizleme', 'live', 'yayın'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Ekran Paylaşımında Otomatik Etkinleştir', description: 'Ekran paylaşımı başlatıldığında yayıncı modunu otomatik olarak aktif eder.', keywords: ['otomatik etkinleştir', 'auto enable', 'ekran paylaşımı', 'screen share'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Ekran Paylaşımı Bitince Otomatik Kapat', description: 'Ekran paylaşımı sona erdiğinde yayıncı modunu otomatik olarak devre dışı bırakır.', keywords: ['otomatik kapat', 'auto disable', 'ekran paylaşımı', 'screen share'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Kullanıcı ID ve Kodlarını Maskele', description: 'Profillerdeki ve mesajlardaki kullanıcı kimliklerini bulanıklaştırarak ifşayı önler.', keywords: ['id maskele', 'gizle', 'bulanıklaştır', 'mask'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'DM İçeriklerini Gizle / Sansürle', description: 'Ekran açıkken gelen özel mesaj bildirimlerini ve sohbet önizlemelerini gizler.', keywords: ['dm sansür', 'özel mesaj gizle', 'bildirim sansür'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Kullanıcı Adlarını Gizle', description: 'Tüm kullanıcı adlarını anonim olarak maskeleyerek gizlilik sağlar.', keywords: ['kullanıcı adı gizle', 'anonimlik', 'username mask'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Zaman Damgalarını Gizle', description: 'Mesajlardaki zaman damgalarını gizleyerek aktivite kalıplarını korur.', keywords: ['zaman damgası', 'timestamp', 'saat', 'gizleme'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Avatarları Bulanıklaştır', description: 'Profil avatarlarını ve görsellerini yayın sırasında bulanıklaştırır.', keywords: ['avatar', 'bulanıklaştırma', 'blur', 'görsel'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Sunucu Adlarını Gizle', description: 'Sol paneldeki sunucu ve kanal adlarını sansürler.', keywords: ['sunucu adı', 'server name', 'gizleme', 'kanal adı'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Mesaj İçeriğini Gizle', description: 'Mesaj metinlerini tamamen gizleyerek yalnızca yapıyı görünür bırakır.', keywords: ['mesaj gizle', 'içerik sansür', 'message hide'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Bahsetmeleri Gizle', description: 'Mesajlardaki @bahsetmeleri sansürleyerek kişi ifşasını önler.', keywords: ['bahsetme', 'mention', 'gizleme', '@'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Kanal Listesini Gizle', description: 'Sol paneldeki kanal listesini tamamen gizler.', keywords: ['kanal listesi', 'channel list', 'gizleme', 'sidebar'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Üye Listesini Gizle', description: 'Sağ paneldeki aktif üye listesini gizler.', keywords: ['üye listesi', 'member list', 'gizleme'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Bildirim Baloncuklarını Gizle', description: 'Sistem bildirim baloncuklarını ve masaüstü açılır pencerelerini sansürler.', keywords: ['bildirim baloncuğu', 'notification badge', 'gizleme'] },
    { tabId: 'streamer', tabLabel: 'Yayıncı Modu', tabIcon: 'broadcast', title: 'Maske Stili', description: 'Sansürleme için kullanılacak görsel maske stilini seçin: bulanık, pikselleme veya solid renk.', keywords: ['maske stili', 'mask style', 'bulanık', 'piksel', 'solid'] },

    // Appearance
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'Hazır Premium Temalar (25 Farklı Koleksiyon)', description: 'Veil Origin, Obsidian Cyan, Royal Indigo, Ruby Noir, Emerald Signal, Aurora Lime ve 25 özel hazır premium tema seçimi.', keywords: ['hazır tema', 'temalar', '25 tema', 'obsidian', 'cyan', 'aurora', 'indigo', 'ruby', 'emerald', 'preset', 'koleksiyon', 'glacier', 'sakura', 'mono', 'paper'] },
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'Kişisel Tema Stüdyosu & CSS Editörü', description: 'Canlı CSS editörü ile kendi renk paletinizi ve tasarım tokenlarınızı özelleştirin.', keywords: ['css editörü', 'kişisel tema', 'özel tema', 'custom css', 'stüdyo', 'theme studio', 'kod', 'şablon'] },
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'Arka Plan Medyası (Görsel / Video)', description: 'Arayüzün arkasında hafif döngülü video veya görsel duvar kağıdı ve opaklık ayarı.', keywords: ['arka plan', 'video', 'görsel', 'resim', 'wallpaper', 'medya', 'opaklık', 'background', 'scrim'] },
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'Yapay Zeka (AI) Tema Sihirbazı', description: 'İstediğiniz temayı tarif edin, yapay zeka sizin için token sözleşmesine tam uyumlu CSS üretsin.', keywords: ['ai tema', 'yapay zeka', 'prompt', 'istem', 'sihirbaz', 'gpt', 'claude', 'gemini'] },
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'Tema İçe & Dışa Aktarma (JSON)', description: 'Kişisel tema yapılandırmanızı sürüm kontrollü .json olarak kaydedin veya içe aktarın.', keywords: ['tema dışa aktar', 'tema içe aktar', 'tema json', 'export theme', 'import theme', 'yedek'] },
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'AMOLED / Saf Siyah Modu', description: 'Koyu gri tonları tamamen #000000 saf siyah ile değiştirerek OLED ekranlarda enerji tasarrufu sağlar.', keywords: ['amoled', 'saf siyah', 'oled', 'karanlık', 'true black', 'siyah'] },
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'Tema Seçimi (Koyu / Açık / Sistem)', description: 'Uygulamanın genel renk paletini koyu, açık veya sistem tercihinize göre ayarlayın.', keywords: ['tema', 'koyu', 'açık', 'sistem', 'dark', 'light', 'theme', 'görünüm'] },
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'Vurgu Rengi (Accent Color)', description: 'Uygulama genelindeki buton, badge ve seçim renk tonunu kişiselleştirin.', keywords: ['vurgu', 'renk', 'accent color', 'tema rengi', 'mor', 'mavi', 'yeşil', 'kırmızı', 'turuncu'] },
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'Kompakt Mesaj Modu', description: 'Mesajlar arası boşlukları ve avatar boyutlarını küçülterek daha fazla içeriği tek ekrana sığdırır.', keywords: ['kompakt', 'dar', 'yoğun', 'compact mode', 'aralık', 'boşluk'] },
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'Yazı Boyutu (Font Size)', description: 'Metin boyutunu 12px ile 20px arasında dinamik olarak ölçekleyin.', keywords: ['yazı boyutu', 'font size', 'metin', 'büyük yazı', 'ölçek', 'font'] },
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'Hareketi Azalt (Reduce Motion)', description: 'Gereksiz animasyonları ve geçiş efektlerini kapatarak performansı artırır.', keywords: ['hareket', 'animasyon', 'reduce motion', 'performans', 'efekt'] },
    { tabId: 'appearance', tabLabel: 'Görünüm', tabIcon: 'sparkle', title: 'Sistem Başlangıcında Otomatik Başlat', description: 'Bilgisayar açıldığında uygulamanın otomatik olarak başlatılmasını sağlar.', keywords: ['otomatik başlat', 'autostart', 'başlangıç', 'sistem açılış', 'boot'] },

    // Audio / Video
    { tabId: 'audio-video', tabLabel: 'Ses & Görüntü', tabIcon: 'mic', title: 'Giriş Cihazı (Mikrofon)', description: 'Sesli görüşmelerde kullanılacak mikrofon aygıtı ve ses giriş seviyesi.', keywords: ['mikrofon', 'giriş cihazı', 'ses aygıtı', 'input device', 'mic', 'ses'] },
    { tabId: 'audio-video', tabLabel: 'Ses & Görüntü', tabIcon: 'mic', title: 'Çıkış Cihazı (Kulaklık / Hoparlör)', description: 'Gelen seslerin çalınacağı hoparlör veya kulaklık aygıtı.', keywords: ['kulaklık', 'hoparlör', 'çıkış', 'output device', 'ses düzeyi', 'vol'] },
    { tabId: 'audio-video', tabLabel: 'Ses & Görüntü', tabIcon: 'mic', title: 'Hoparlör Çıkış Cihazı', description: 'Sesli sohbetlerde seslerin yönlendirileceği hoparlör veya ses çıkış aygıtı.', keywords: ['hoparlör', 'speaker', 'çıkış cihazı', 'output', 'ses'] },
    { tabId: 'audio-video', tabLabel: 'Ses & Görüntü', tabIcon: 'mic', title: 'Kamera Aygıtı & Aynalama', description: 'Görüntülü aramalarda kullanılacak kamera donanımı ve ayna modu tercihi.', keywords: ['kamera', 'video', 'ayna', 'webcam', 'görüntü', 'camera'] },
    { tabId: 'audio-video', tabLabel: 'Ses & Görüntü', tabIcon: 'mic', title: 'Kamera Aynalama (Mirror)', description: 'Kamera görüntüsünü yatay olarak aynalayarak doğal selfie görünümü sağlar.', keywords: ['ayna', 'mirror', 'yansıma', 'kamera aynalama', 'selfie'] },
    { tabId: 'audio-video', tabLabel: 'Ses & Görüntü', tabIcon: 'mic', title: 'Ekran Paylaşımı Kalitesi', description: 'Ekran paylaşımı çözünürlüğünü ve kare hızını ayarlayın (720p/1080p, 30/60 FPS).', keywords: ['ekran paylaşımı', 'çözünürlük', 'kalite', 'kare hızı', 'fps', '1080p', '720p'] },
    { tabId: 'audio-video', tabLabel: 'Ses & Görüntü', tabIcon: 'mic', title: 'Yapay Zeka Gürültü Engelleme', description: 'Arka plan gürültülerini, klavye ve fan seslerini filtreler.', keywords: ['gürültü engelleme', 'noise suppression', 'filtre', 'krisp', 'temizleme'] },
    { tabId: 'audio-video', tabLabel: 'Ses & Görüntü', tabIcon: 'mic', title: 'Yankı Engelleme (Echo Cancellation)', description: 'Hoparlörden mikrofona geri dönen ses döngüsünü engeller.', keywords: ['yankı engelleme', 'echo', 'eko', 'döngü'] },
    { tabId: 'audio-video', tabLabel: 'Ses & Görüntü', tabIcon: 'mic', title: 'Bas-Konuş (Push to Talk)', description: 'Yalnızca belirlenen kısayol tuşuna basıldığında mikrofonu açar.', keywords: ['bas konuş', 'ptt', 'push to talk', 'tuş', 'kısayol'] },

    // Notifications
    { tabId: 'notifications', tabLabel: 'Bildirimler', tabIcon: 'bell', title: 'Masaüstü Bildirimleri', description: 'Uygulama arka plandayken işletim sistemi bildirimlerini gösterir.', keywords: ['masaüstü bildirim', 'desktop notification', 'açılır pencere', 'uyarı'] },
    { tabId: 'notifications', tabLabel: 'Bildirimler', tabIcon: 'bell', title: 'Sadece Bahsetmeler (Mention Only)', description: 'Yalnızca doğrudan bahsetildiğinizde bildirim gösterir.', keywords: ['bahsetme', 'mention only', 'sadece mention', 'bildirim'] },
    { tabId: 'notifications', tabLabel: 'Bildirimler', tabIcon: 'bell', title: 'Sesli Uyarılar (Mesaj & Bahsetme)', description: 'Yeni mesaj veya etiketleme geldiğinde hafif ses efektleri çalar.', keywords: ['sesli uyarı', 'mesaj sesi', 'bahsetme sesi', 'notification sound', 'ses'] },
    { tabId: 'notifications', tabLabel: 'Bildirimler', tabIcon: 'bell', title: 'Rahatsız Etme Modunda Sessize Al', description: 'DND durumundayken bildirim seslerini ve açılır pencereleri bastırır.', keywords: ['dnd', 'rahatsız etme', 'sessiz', 'do not disturb'] },
    { tabId: 'notifications', tabLabel: 'Bildirimler', tabIcon: 'bell', title: 'Mesaj Sesi Seviyesi', description: 'Yeni mesaj bildirimlerini ne kadar sesli alacağınızı ayarlayın.', keywords: ['ses seviyesi', 'volume', 'mesaj sesi', 'sound level'] },
    { tabId: 'notifications', tabLabel: 'Bildirimler', tabIcon: 'bell', title: 'Bahsetme Sesi Seviyesi', description: 'Etiketlendiğinizde duyulacak bildirim sesi seviyesi.', keywords: ['bahsetme sesi', 'mention sound', 'ses seviyesi'] },
    { tabId: 'notifications', tabLabel: 'Bildirimler', tabIcon: 'bell', title: 'Arkadaşlık İsteği Sesi', description: 'Yeni arkadaşlık isteklerinde duyulacak bildirim sesi.', keywords: ['arkadaşlık isteği', 'friend request', 'ses'] },
    { tabId: 'notifications', tabLabel: 'Bildirimler', tabIcon: 'bell', title: 'Arama Bildirimi Sesi', description: 'Gelen ses veya görüntü aramasında duyulacak bildirim sesi.', keywords: ['arama', 'call', 'ses', 'bildirim'] },
    { tabId: 'notifications', tabLabel: 'Bildirimler', tabIcon: 'bell', title: 'Genel Ses Seviyesi', description: 'Tüm bildirim seslerinin genel ses seviyesi ayarı.', keywords: ['ses seviyesi', 'volume', 'genel ses', 'master'] },
    { tabId: 'notifications', tabLabel: 'Bildirimler', tabIcon: 'bell', title: 'Test Bildirimi', description: 'Bildirim ayarlarınızın doğru çalıştığını test etmek için örnek bildirim gönderir.', keywords: ['test', 'deneme', 'örnek bildirim', 'test notification'] },

    // Security
    { tabId: 'security', tabLabel: 'Güvenlik', tabIcon: 'shield', title: 'Parola Değiştirme', description: 'Hesap anahtar kasanızın şifreleme parolasını güncelleyin.', keywords: ['parola', 'şifre değiştir', 'password', 'güvenlik', 'kasa'] },
    { tabId: 'security', tabLabel: 'Güvenlik', tabIcon: 'shield', title: 'Parola Doğrulama', description: 'Hassas işlemler öncesi mevcut parolanızı doğrulamanızı ister.', keywords: ['parola doğrulama', 'verify password', 'şifre doğrulama', 'kimlik doğrulama'] },
    { tabId: 'security', tabLabel: 'Güvenlik', tabIcon: 'shield', title: 'Kurtarma Kodu (Emergency Kit)', description: 'Cihaz kayıplarında hesabınızı geri yüklemek için gerekli 24 kelimelik kurtarma kodu.', keywords: ['kurtarma kodu', 'recovery code', 'yedek', 'emergency', 'kelimeler'] },
    { tabId: 'security', tabLabel: 'Güvenlik', tabIcon: 'shield', title: 'Kurtarma Kodu Doğrulama', description: 'Giriş yaparken veya hassas işlemlerde kurtarma kodunuzun doğrulanmasını ister.', keywords: ['kurtarma doğrulama', 'recovery verify', 'kod doğrulama'] },
    { tabId: 'security', tabLabel: 'Güvenlik', tabIcon: 'shield', title: 'Parola Sızıntısı Kontrolü', description: 'Parolanızın bilinen veri sızıntılarında yer alıp almadığını kontrol eder.', keywords: ['parola sızıntısı', 'password leak', 'haveibeenpwned', 'sızıntı kontrolü'] },
    { tabId: 'security', tabLabel: 'Güvenlik', tabIcon: 'shield', title: 'Aktif Cihazlar & Oturumlar', description: 'Hesabınızın açık olduğu tüm bağlı cihazları görüntüleyin ve oturumlarını sonlandırın.', keywords: ['cihazlar', 'oturumlar', 'sessions', 'bağlı aygıtlar', 'çıkış'] },
    { tabId: 'security', tabLabel: 'Güvenlik', tabIcon: 'shield', title: 'Otomatik Kilit Açma (Auto Unlock)', description: 'Bu cihazda güvenli depolama ile parola sormadan anında giriş yapın.', keywords: ['otomatik giriş', 'auto unlock', 'parola sorma', 'hızlı'] },
    { tabId: 'security', tabLabel: 'Güvenlik', tabIcon: 'shield', title: 'Güvenilir Alanlar', description: 'Otomatik giriş yapılan güvenilir cihazları ve alanları yönetin.', keywords: ['güvenilir alan', 'trusted domain', 'güvenilir cihaz', 'otomatik giriş'] },
    { tabId: 'security', tabLabel: 'Güvenlik', tabIcon: 'shield', title: 'Verileri Dışa Aktar', description: 'Mesaj geçmişinizi ve hesap verilerinizi şifreli olarak dışa aktarın.', keywords: ['dışa aktar', 'export', 'veri aktarım', 'yedekleme'] },
    { tabId: 'security', tabLabel: 'Güvenlik', tabIcon: 'shield', title: 'Verileri İçe Aktar', description: 'Daha önce dışa aktarılmış şifreli veri yedeklerini geri yükleyin.', keywords: ['içe aktar', 'import', 'geri yükle', 'yedek geri yükleme'] },
    { tabId: 'security', tabLabel: 'Güvenlik', tabIcon: 'shield', title: 'Tüm Verileri Temizle', description: 'Yerel depolanan tüm mesaj geçmişini, anahtarları ve önbelleği kalıcı olarak siler.', keywords: ['temizle', 'clear', 'sil', 'veri temizleme', 'sifırla'] },

    // About
    { tabId: 'about', tabLabel: 'Hakkında', tabIcon: 'info', title: 'veilanon Sürümü & Güncellemeler', description: 'Mevcut sürüm detayları, lisans bilgileri ve GitHub güncelleme kontrolü.', keywords: ['sürüm', 'güncelleme', 'versiyon', 'update', 'github', 'hakkında', 'lisans'] },
  ];

  let searchQuery = $state('');

  const deepSearchResults = $derived(
    searchQuery.trim()
      ? (() => {
          const q = searchQuery.toLowerCase().trim();
          const qWords = q.split(/\s+/).filter(w => w.length > 0);
          return ALL_SUB_SETTINGS.filter(item => {
            const text = `${item.title} ${item.description} ${item.keywords.join(' ')} ${item.tabLabel}`.toLowerCase();
            return qWords.every(w => text.includes(w));
          });
        })()
      : []
  );

  const GROUPS: Array<{ label: string; ids: string[] }> = [
    { label: 'Profil', ids: ['account'] },
    { label: 'Tercihler', ids: ['privacy', 'streamer', 'appearance', 'audio-video', 'notifications'] },
    { label: 'Sistem', ids: ['security', 'about'] },
  ];

  let overlayEl = $state<HTMLDivElement | null>(null);
  let cleanupTrap: (() => void) | null = null;
  let cleanupEsc: (() => void) | null = null;

  $effect(() => {
    if (!ui.openModal) return;
    cleanupTrap?.();
    cleanupTrap = null;
    cleanupEsc?.();
    cleanupEsc = null;
    const overlay = overlayEl;
    if (!overlay) return;
    tick().then(() => {
      cleanupTrap?.();
      cleanupTrap = createFocusTrap(overlay);
    });
    cleanupEsc = handleEscape(() => uiStore.closeModal());
    return () => {
      cleanupTrap?.();
      cleanupTrap = null;
      cleanupEsc?.();
      cleanupEsc = null;
    };
  });

  function onOverlayClick(e: MouseEvent) {
    if (e.target === overlayEl) uiStore.closeModal();
  }

  function jumpToSetting(tabId: string) {
    uiStore.setSettingsTab(tabId);
    searchQuery = '';
  }
</script>

<div
  class="veil-overlay"
  bind:this={overlayEl}
  role="presentation"
  onclick={onOverlayClick}
>
  <div
    class="veil-modal veil-modal-lg veil-settings-modal"
    role="dialog"
    aria-modal="true"
    aria-label="Ayarlar"
  >
    <div class="veil-settings">
      <nav class="veil-settings-nav" aria-label="Ayarlar sekmeleri">
        <div class="veil-settings-search">
          <Icon name="search" size={14} />
          <input
            type="text"
            class="veil-settings-search-input"
            placeholder="Seçenek veya ayar ara…"
            aria-label="Ayarlarda ara"
            bind:value={searchQuery}
          />
          {#if searchQuery}
            <button
              type="button"
              class="btn-icon"
              style="width: 20px; height: 20px;"
              title="Aramayı temizle"
              onclick={() => (searchQuery = '')}
            >
              <Icon name="x" size={12} />
            </button>
          {/if}
        </div>

        {#each GROUPS as group (group.label)}
          <div class="veil-settings-section-label">{group.label}</div>
          {#each tabs.filter(t => group.ids.includes(t.id)) as tab (tab.id)}
            {@const matchCount = deepSearchResults.filter(r => r.tabId === tab.id).length}
            {@const hasMatches = !searchQuery.trim() || matchCount > 0}
            <button
              class="veil-settings-nav-item"
              class:active={!searchQuery && ui.settingsTab === tab.id}
              class:dimmed={searchQuery.trim() && !hasMatches}
              aria-current={!searchQuery && ui.settingsTab === tab.id ? 'page' : undefined}
              onclick={() => { searchQuery = ''; uiStore.setSettingsTab(tab.id); }}
            >
              <span class="veil-settings-nav-icon" aria-hidden="true"><Icon name={tab.icon} size={18} /></span>
              <span class="veil-settings-nav-label">{tab.label}</span>
              {#if searchQuery.trim() && matchCount > 0}
                <span class="veil-nav-match-pill">{matchCount}</span>
              {/if}
            </button>
          {/each}
        {/each}
      </nav>

      <div class="veil-settings-content" tabindex="-1">
        {#if searchQuery.trim()}
          <div class="veil-settings-search-results">
            <div class="veil-search-res-header">
              <h3>"{searchQuery}" için Arama Sonuçları</h3>
              <span class="badge">{deepSearchResults.length} seçenek bulundu</span>
            </div>

            {#if deepSearchResults.length === 0}
              <div class="veil-search-empty">
                <Icon name="search" size={32} class="text-muted" />
                <p>Aradığınız kelimeye uygun bir ayar veya seçenek bulunamadı.</p>
              </div>
            {:else}
              <div class="veil-search-results-list">
                {#each deepSearchResults as res}
                  <div class="veil-search-result-card">
                    <div class="veil-search-card-meta">
                      <span class="veil-search-tab-pill">
                        <Icon name={res.tabIcon} size={12} />
                        {res.tabLabel}
                      </span>
                      <h4 class="veil-search-card-title">{res.title}</h4>
                      <p class="veil-search-card-desc">{res.description}</p>
                    </div>
                    <button
                      type="button"
                      class="btn btn-secondary btn-sm"
                      onclick={() => jumpToSetting(res.tabId)}
                    >
                      <span>Ayara Git</span>
                      <Icon name="arrow-right" size={14} />
                    </button>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {:else if ui.settingsTab === 'account'}
          <AccountSettings />
        {:else if ui.settingsTab === 'privacy'}
          <PrivacySettings />
        {:else if ui.settingsTab === 'streamer'}
          <StreamerSettings />
        {:else if ui.settingsTab === 'appearance'}
          <AppearanceSettings />
        {:else if ui.settingsTab === 'audio-video'}
          <AudioVideoSettings />
        {:else if ui.settingsTab === 'notifications'}
          <NotificationSettings />
        {:else if ui.settingsTab === 'security'}
          <SecuritySettings />
        {:else}
          <AboutSettings />
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .veil-settings-modal {
    max-width: 920px;
    height: min(700px, 86dvh);
  }
  .veil-settings-modal :global(.veil-modal-body) { display: none; }
  .veil-settings-nav-item {
    height: 40px;
    padding: 0 var(--space-3);
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    border: none;
    background: transparent;
    border-radius: var(--radius-lg);
    cursor: pointer;
    font-size: var(--text-base);
    font-family: var(--font-sans);
    color: var(--veil-text-secondary);
    transition: background var(--t-fast), color var(--t-fast), transform var(--t-fast);
    margin-bottom: 2px;
    position: relative;
    text-align: left;
  }
  .veil-settings-nav-item:hover {
    background: var(--veil-bg-surface);
    color: var(--veil-text-primary);
  }
  .veil-settings-nav-item.dimmed {
    opacity: 0.45;
  }
  .veil-nav-match-pill {
    font-size: 10px;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: var(--radius-full);
    background: var(--veil-brand);
    color: #fff;
  }
  .veil-settings-nav-icon {
    width: 26px;
    height: 26px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border-radius: var(--radius-md);
    background: transparent;
    transition: background var(--t-fast);
  }
  .veil-settings-nav-item.active .veil-settings-nav-icon {
    background: var(--veil-brand);
    color: #fff;
  }
  .veil-settings-nav-label {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .veil-settings-section-label {
    padding-top: var(--space-4);
    padding-bottom: var(--space-2);
  }
  .veil-settings-section-label:first-child { padding-top: 0; }

  .veil-settings-search {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-3);
    margin-bottom: var(--space-2);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-lg);
    color: var(--veil-text-muted);
    transition: border-color var(--t-fast);
  }
  .veil-settings-search:focus-within {
    border-color: var(--veil-brand);
  }
  .veil-settings-search-input {
    flex: 1;
    min-width: 0;
    border: none;
    background: transparent;
    color: var(--veil-text-primary);
    font-size: var(--text-sm);
    font-family: var(--font-sans);
    padding: var(--space-2) 0;
    outline: none;
  }
  .veil-settings-search-input::placeholder {
    color: var(--veil-text-disabled);
  }

  .veil-settings-search-results {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
  }

  .veil-search-res-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding-bottom: var(--space-3);
    border-bottom: 1px solid var(--veil-border-subtle);
  }

  .veil-search-res-header h3 {
    font-size: 1.1rem;
    font-weight: 700;
    color: var(--veil-text-primary);
    margin: 0;
  }

  .veil-search-results-list {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  .veil-search-result-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    padding: var(--space-4);
    background: var(--veil-bg-surface);
    border: 1px solid var(--veil-border-subtle);
    border-radius: var(--radius-xl);
    transition: all 0.15s ease;
  }

  .veil-search-result-card:hover {
    border-color: var(--veil-brand-border, rgba(88, 101, 242, 0.4));
    background: var(--veil-bg-elevated);
  }

  .veil-search-card-meta {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .veil-search-tab-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    font-weight: 700;
    color: var(--veil-brand);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .veil-search-card-title {
    font-size: 14px;
    font-weight: 700;
    color: var(--veil-text-primary);
    margin: 0;
  }

  .veil-search-card-desc {
    font-size: 12px;
    color: var(--veil-text-secondary);
    line-height: 1.4;
    margin: 0;
  }

  .veil-search-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: var(--space-8);
    text-align: center;
    gap: var(--space-3);
    color: var(--veil-text-muted);
  }
</style>
