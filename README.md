# veilanon

<p align="center">
  <img src="https://veilanon.com/brand/veilanon-logo.svg" alt="veilanon Logo" width="112" height="112" />
</p>

<p align="center">
  <strong>Gizlilik Öncelikli, Yüksek Performanslı, Açık Kaynaklı Masaüstü İletişim Platformu</strong><br>
  <strong>Privacy-First, High-Performance Open-Source Desktop Communication Platform</strong><br>
  <em>Modern topluluklar için sıfır-bilgi (zero-knowledge) ve uçtan uca şifrelemeyle (E2EE) geliştirilmiş Discord alternatifi.</em>
</p>

<p align="center">
  <a href="#turkce"><img src="brand/flags/flag_tr.svg" alt="Türkçe" width="22" height="15" valign="middle" /> <strong>Türkçe</strong></a>
  &nbsp;&nbsp;•&nbsp;&nbsp;
  <a href="#english"><img src="brand/flags/flag_gb.svg" alt="English" width="22" height="15" valign="middle" /> <strong>English</strong></a>
</p>

<p align="center">
  <a href="https://github.com/MrSpy00/veilanon/releases/tag/v0.0.1"><img src="https://img.shields.io/badge/release-v0.0.1-8b5cf6?style=flat-square&logo=github" alt="Release v0.0.1" /></a>
  <a href="https://veilanon.com"><img src="https://img.shields.io/badge/website-veilanon.com-6366f1?style=flat-square" alt="Website" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-AGPL--3.0-10b981?style=flat-square" alt="License" /></a>
  <a href="https://tauri.app"><img src="https://img.shields.io/badge/built%20with-Tauri%202%20%2B%20Rust-f97316?style=flat-square&logo=tauri" alt="Tauri 2" /></a>
  <a href="https://svelte.dev"><img src="https://img.shields.io/badge/frontend-Svelte%205%20%2B%20SvelteKit-ff3e00?style=flat-square&logo=svelte" alt="Svelte 5" /></a>
</p>

---

<a name="turkce"></a>
# <img src="brand/flags/flag_tr.svg" alt="Türkçe" width="28" height="19" valign="middle" /> Türkçe

## ⚡ Genel Bakış

**veilanon**, **Tauri 2**, **Rust** ve **Svelte 5 / SvelteKit** teknolojileri üzerine inşa edilmiş, gizlilik odaklı, çok platformlu bir masaüstü iletişim platformudur. Discord'un modern, akıcı ve zengin kullanıcı deneyimini, Signal seviyesinde sıfır-bilgi (zero-knowledge) matematiksel kriptografik güvencelerle birleştirir:

- **Sıfır-Bilgi Mimarisi (Zero-Knowledge):** Mesaj metinleri, ekler, sesli ve görüntülü görüşme medyaları ağa iletilmeden önce doğrudan yerel cihazınızda şifrelenir.
- **Kör Zarf Yönlendirme (Dumb Envelope Relaying):** Kontrol düzlemi / sunucular yalnızca opak şifreli zarfları yönlendirir; gönderdiğiniz veya söylediğiniz hiçbir veriyi göremez, çözemez ve depolayamaz.
- **Donanımsal İzolasyonlu Yerel Güvenlik:** Yerel mesaj geçmişi ve anahtarlık veritabanı cihazda **AES-256-GCM** ve **Argon2id** anahtar türetimiyle şifrelenerek saklanır.

[aegisSoft](https://www.aegissoft.com.tr/) tarafından geliştirilmektedir. Marka adı her zaman küçük harfle yazılır: `veilanon`.

---

## 🚀 v0.0.1 Sürüm Öne Çıkanları

- **Kusursuz Mesaj Düzenleme:** Düzenleme ikonuna basıldığında açılan satır içi (inline) Discord tarzı editör ile ekli veya metin mesajlarının anlık yerel + sunucu senkronizasyonu.
- **Gelişmiş Video Oynatıcı:** Akıcı ses slider'ı, sessize alma (mute) butonu ve hem native hem de CSS tabanlı tam ekran (fullscreen) modu.
- **Kalıcı Banner ve Profil Senkronizasyonu:** Hesap ve sunucu banner'ları/avatarları uygulama yeniden başlatıldığında veya yeni cihazda oturum açıldığında silinmez; Supabase Realtime ile anlık senkronize kalır.
- **Akıllı Tema ve Vurgu Rengi Senkronizasyonu:** Tema değiştiğinde vurgu rengi otomatik olarak yeni temanın varsayılanına döner; manuel yapılan renk seçimleri ise bir sonraki tema değişimine kadar güvenle korunur.
- **Uçtan Uca Eşitleme (E2EE Sync):** Mesajlar, tepkiler, çevrim içi durumu (presence), arkadaşlıklar, DM'ler, grup sohbetleri, roller ve bildirim badge'leri tüm bağlı cihazlarda anlık tutarlıdır.

---

## 📦 Çok Platformlu Kurulum Paketleri (v0.0.1 İndirme)

Tüm kurulum paketleri dijital olarak imzalanmış ve SHA-256 sağlama toplamları ile doğrulanmıştır. İşletim sisteminize uygun paketi doğrudan [GitHub Releases v0.0.1](https://github.com/MrSpy00/veilanon/releases/tag/v0.0.1) veya [veilanon.com](https://veilanon.com) adresinden indirebilirsiniz:

| İşletim Sistemi | Paket Formatı | Doğrudan İndirme Bağlantısı | Açıklama |
| :--- | :--- | :--- | :--- |
| **Windows** | `.exe` Kurulum | [`veilanon_0.0.1_x64-setup.exe`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_x64-setup.exe) | NSIS Tam Kurulum Sihirbazı (İmzalı) |
| **Windows** | `.msi` Yükleyici | [`veilanon_0.0.1_x64_en-US.msi`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_x64_en-US.msi) | WiX Kurumsal MSI Yükleyici Paketi |
| **macOS** | `.dmg` İmajı | [`veilanon_0.0.1_aarch64.dmg`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_aarch64.dmg) | Apple Silicon (M1/M2/M3/M4) Disk İmajı |
| **macOS** | `.app.tar.gz` | [`veilanon_0.0.1_aarch64.app.tar.gz`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_aarch64.app.tar.gz) | macOS Apple Silicon Uygulama Arşivi |
| **Linux** | `.AppImage` | [`veilanon_0.0.1_amd64.AppImage`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_amd64.AppImage) | Bağımsız, Evrensel Taşınabilir Linux Paketi |
| **Linux** | `.deb` Paketi | [`veilanon_0.0.1_amd64.deb`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_amd64.deb) | Ubuntu / Debian Yerel Kurulum Paketi |
| **Linux** | `.rpm` Paketi | [`veilanon-0.0.1-1.x86_64.rpm`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon-0.0.1-1.x86_64.rpm) | Fedora / RHEL / openSUSE Yerel Paketi |
| **Linux** | `.tar.gz` Arşivi | [`veilanon.app.tar.gz`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon.app.tar.gz) | Genel Linux / Taşınabilir Paket Arşivi |
| **Doğrulama** | Sağlama Özeti | [`SHA256SUMS.txt`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/SHA256SUMS.txt) | SHA-256 Kriptografik Sağlama Özeti |

---

## ✨ Özellik Matrisi

### 🔒 Kriptografi & Gizlilik Motoru
- **Birebir Mesajlar (1:1 DM):** Her mesajda anahtar türeten (`HKDF-SHA256`) ve iletim gizliliği (forward secrecy) sunan **Double Ratchet** mimarisi.
- **Grup Kanalları ve Alanlar:** Ağaç tabanlı anahtar uzlaşması ve KeyPackage / Welcome süreçleriyle Messaging Layer Security (**MLS**, RFC 9420).
- **Yerel Depolama Şifrelemesi:** Uygulama katmanında **AES-256-GCM** ve **Argon2id** parola türetimli gömülü SQLite veritabanı.
- **Şifreli Dosya Kasası:** Sunucunun dosya adlarını, boyutlarını veya içeriklerini asla göremediği istemci taraflı ChaCha20-Poly1305 parça şifreleme.
- **Kaybolan Mesajlar:** 10 saniyeden 1 haftaya kadar görsel geri sayım sayacı, yerel ve uzak otomatik imha (tombstoning) mekanizması.

### 🛡️ Genel Gizlilik & Güvenlik Araçları (Sıfır-Anahtar Merkezi)
- **Tor & Relay Anonimlik Denetleyicisi:** Gerçek zamanlı Tor çıkış düğümü ve relay doğrulama.
- **Çoklu DoH Kıyaslama:** Cloudflare, Google, Quad9, AdGuard ve Mullvad üzerinde 5 yönlü DNS gecikme ve sansür analizi.
- **k-Anonimlik Sızıntı Kontrolü:** Parolayı hiçbir sunucuya göndermeden HaveIBeenPwned veritabanında SHA-1 önek taraması.
- **URLhaus Tehdit Tarayıcısı:** Abuse.ch gerçek zamanlı tehdit akışı üzerinden zararlı yazılım ve oltalama URL kontrolü.
- **Saat Sapma Analizi (Clock Skew):** Kriptografik tekrar saldırılarını engellemek için mikrosaniye düzeyinde NTP senkronizasyon kontrolü.
- **Deterministik Gizlilik Avatarları:** Ağ isteği yapmadan tamamen çevrim dışı üretilen prosedürel profil ikonları.

### 🎙️ Ses, Video & Ekran Paylaşımı
- **LiveKit Seçici İletim Birimi (SFU):** Düşük gecikmeli HD ses, video ve 1080p 60 FPS ekran paylaşımı.
- **MLS Medya Anahtar Türetimi:** E2EE ses kanalları kare şifreleme anahtarlarını doğrudan MLS grup sırlarından türetir.
- **Donanım Yaşam Döngüsü Yönetimi:** Kamera kapatıldığında web kamerası sensörünün veya LED'inin açık kalmasını engelleyen WebRTC donanım yönetimi.
- **Canlı Cihaz Değiştirme:** Mikrofon, hoparlör ve kamera arasında anlık geçiş, ses göstergesi ve Bas-Konuş (PTT) desteği.

### 💬 Sosyal & Topluluk Deneyimi
- **Alanlar, Kanallar ve İzinler:** Metin, ses, forum ve duyuru kanalları; detaylı rol ve yetkilendirme sistemi.
- **Mesaj Düzenleme:** Satır içi Discord tarzı editör, Esc ile iptal, Enter ile kaydetme ve düzenlendi damgası senkronizasyonu.
- **Özel Video Oynatıcı:** Zaman çizelgesinde sarma, hover ses slider'ı, sessize alma ve kararlı tam ekran geçişi.
- **Derin Bağlantı Protokolü (`veilanon://`):** Davetler (`/invite/{kod}`), profiller (`/u/{kullanici}`), kanallar ve mesajlar için evrensel bağlantılar.
- **Zengin Medya & Emojiler:** Tenor/Giphy GIF araması, emoji seçici, 3:1 banner kırpıcı ve animasyonlu durum rozetleri.

---

## 🏛️ Mimari Yapı

veilanon; kullanıcı arayüzü, yerel Rust çekirdeği ve bulut adaptörleri arasında katı bir ayrım ile inşa edilmiştir:

```
┌─────────────────────────────────────────────────────────────────────┐
│  Arayüz Katmanı (SvelteKit / Svelte 5 + Tailwind Değişkenleri)      │
│  Kanallar, Roller, Video Izgarası, Ayarlar, Gizlilik Merkezi        │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ Tauri IPC (invoke/emit) — tip-güvenli
                                │ JS katmanına ham anahtarlar ASLA sızmaz
┌───────────────────────────────▼─────────────────────────────────────┐
│  Yerel Çekirdek (Rust / src-tauri)                                  │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ │
│  │ Kriptografi  │ │ Yerel DB     │ │ Çevrim Dışı  │ │ Adaptörler   │ │
│  │ (OpenMLS,    │ │ (SQLite +    │ │ Kuyruk       │ │ (Supabase,   │ │
│  │  Double      │ │  AES-256-GCM)│ │  (Otomatik   │ │  LiveKit,    │ │
│  │  Ratchet)    │ │              │ │  Yeniden Dene│ │  R2 Storage) │ │
│  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘ │
└───────┬───────────────────┬───────────────────┬──────────────────────┘
        │ Şifreli Zarflar   │ Şifreli Dosyalar  │ Sinyalizasyon & Token
┌───────▼─────────┐ ┌───────▼─────────┐ ┌───────▼──────────────────────┐
│ VERİ DÜZLEMİ    │ │ MEDYA DÜZLEMİ   │ │ KONTROL DÜZLEMİ              │
│ Supabase        │ │ Cloudflare R2   │ │ Supabase Auth + Edge Funcs   │
│ (PostgreSQL,    │ │ (Yalnızca Opak  │ │ (livekit-token, Realtime WS) │
│  Yalnızca Çözüle│ │  Şifreli        │ │  + LiveKit Cloud SFU         │
│  meyen Şifreli  │ │  Dosya          │ │                              │
│  Zarflar)       │ │  Parçaları)     │ │                              │
└─────────────────┘ └─────────────────┘ └──────────────────────────────┘
```

Teknik detaylar için belgeleri inceleyin:

| Belge | Kapsam ve Açıklama |
| :--- | :--- |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Katmanlı tasarım, IPC izolasyonu ve adaptör mimarisi |
| [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) | Tehdit modeli, saldırgan tanımları ve güvenlik güvenceleri |
| [`docs/PRIVACY_SCOPE.md`](docs/PRIVACY_SCOPE.md) | Uçtan uca şifreleme ve metaveri görünürlük matrisi |
| [`docs/CRYPTO_DECISIONS.md`](docs/CRYPTO_DECISIONS.md) | Tercih edilen kriptografik algoritmaların gerekçeleri |
| [`docs/DATA_INVENTORY.md`](docs/DATA_INVENTORY.md) | Saklanan tüm alanların, şifrelemelerin ve saklama sürelerinin envanteri |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Tamamlanan aşamalar, sürüm geçmişi ve yol haritası |
| [`docs/SUPABASE_SETUP.md`](docs/SUPABASE_SETUP.md) | Supabase veritabanı tabloları, RLS ve migrasyon rehberi |
| [`docs/BOT_API.md`](docs/BOT_API.md) | Bot manifestoları ve webhook entegrasyon kuralları |
| [`docs/DISCORD_BRIDGE.md`](docs/DISCORD_BRIDGE.md) | Discord webhook köprüleme kuralları ve gizlilik etiketleri |
| [`TEST_INFRA.md`](TEST_INFRA.md) | Otomatik E2E, Rust birim ve TypeScript test altyapısı |

---

## 🛠️ Geliştirme ve Derleme

### Gereksinimler

- [Rust](https://rustup.rs/) (kararlı 1.80+ toolchain)
- [Node.js](https://nodejs.org/) 22+ ve `npm`
- Platform SDK'ları:
  - **Windows:** Microsoft Visual Studio C++ Build Tools & WebView2
  - **macOS:** Xcode Command Line Tools
  - **Linux:** `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libxdo-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

### Yerel Geliştirme Ortamını Çalıştırma

```bash
# 1. Projeyi klonlayın
git clone https://github.com/MrSpy00/veilanon.git
cd veilanon

# 2. Ön yüz bağımlılıklarını kurun
npm install

# 3. Ortam değişkenlerini yapılandırın
cp .env.example .env

# 4. Geliştirici modunda başlatın
npm run tauri:dev
```

### Testleri Çalıştırma

```bash
# SvelteKit & TypeScript tip denetimi (0 hata, 0 uyarı)
npm run check

# Uçtan uca (E2E) entegrasyon test paketi
npm run test:e2e

# Yerel Rust kriptografik birim testleri (113 test)
cargo test --manifest-path src-tauri/Cargo.toml
```

### Kurulum Paketlerini Derleme

```bash
# Ön yüz üretim derlemesi
npm run build

# Tauri yerel masaüstü kurulum paketlerini derleme
npm run tauri:build
```

---

## 📜 Gizlilik Taahhüdü

veilanon her sürümde yedi temel ilkeye bağlı kalır:

1. **İçeriğiniz yalnızca size aittir:** Mesajlarınız, ekleriniz ve ses/video medyanız cihazınızda şifrelenir. Sunucu yalnızca şifreli veri görür.
2. **Sunucuda sıfır düz metin:** Veritabanında şifrelenmemiş içerik sütunu veya düz metin arama dizini bulunmaz.
3. **Gizli telemetri veya izleme yoktur:** Kullanıcı davranışı izlenmez, analitik toplanmaz.
4. **Varsayılan olarak en az metaveri:** Çevrim içi durumları gruplandırılır, okundu bilgisi ve yazıyor göstergesi isteğe bağlıdır, bağlantı önizlemeleri yalıtılmış ortamda taranır.
5. **Veri taşınabilirliği:** Şifreli SQLite arşivinizi istediğiniz zaman dışa aktarabilir veya geri yükleyebilirsiniz.
6. **Denetlenmiş kriptografi:** Özel kriptografi yazılmaz; yalnızca endüstri standardı, güvenilir kütüphaneler kullanılır (OpenMLS, AES-GCM, Argon2id, Dalek).
7. **Dürüst bilgilendirme:** Nelerin şifrelendiği ve hangi metaverilerin görülebildiği açıkça belgelenir.

<br><br>

---

<a name="english"></a>
# <img src="brand/flags/flag_gb.svg" alt="English" width="28" height="19" valign="middle" /> English

## ⚡ Overview

**veilanon** is a privacy-first, cross-platform desktop communication application built on **Tauri 2**, **Rust**, and **Svelte 5 / SvelteKit**. It combines the rich, modern, fluid user experience of Discord with the zero-knowledge mathematical guarantees of Signal-level cryptography:

- **Zero-Knowledge Architecture:** Message bodies, attachments, and call media are encrypted directly on your local device before they ever touch the network.
- **Dumb Envelope Relaying:** The control plane / server operates solely as an opaque ciphertext envelope router — it can never inspect or decrypt what you send or say.
- **Hardware-Isolated Privacy:** Local message histories and keychains are encrypted at-rest using **AES-256-GCM** and **Argon2id** key derivation.

Developed by [aegisSoft](https://www.aegissoft.com.tr/). Brand name is strictly lowercase: `veilanon`.

---

## 🚀 v0.0.1 Release Highlights

- **Reliable Message Editing:** Inline (Discord-style) editor triggered by the edit icon, supporting caption edits on attachments and text messages with instant local and server synchronization.
- **Refined Video Player:** Smooth hover volume slider, mute toggle, and rock-solid fullscreen mode (supporting both native fullscreen and CSS fallback).
- **Persistent Banner & Profile Sync:** User and space banners/avatars remain intact upon application restarts or fresh logins, staying updated via Supabase Realtime across all devices.
- **Theme-Aware Accent Color:** Switching a theme resets the accent to that theme's default brand color; manual accent overrides are honored until the next theme switch.
- **End-to-End State Synchronization:** Messages, reactions, presence, friendships, DMs, group DMs, roles, and unread badges stay perfectly synchronized across all devices.

---

## 📦 Multi-Platform Release Downloads (v0.0.1)

All release packages are digitally signed and verified with SHA-256 checksum manifests. You can download pre-compiled binaries for your operating system directly from [GitHub Releases v0.0.1](https://github.com/MrSpy00/veilanon/releases/tag/v0.0.1) or from [veilanon.com](https://veilanon.com):

| Platform | Format / Type | Direct Download Link | Description |
| :--- | :--- | :--- | :--- |
| **Windows** | `.exe` Setup | [`veilanon_0.0.1_x64-setup.exe`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_x64-setup.exe) | NSIS Full Setup Wizard (Signed) |
| **Windows** | `.msi` Installer | [`veilanon_0.0.1_x64_en-US.msi`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_x64_en-US.msi) | WiX Enterprise MSI Installer |
| **macOS** | `.dmg` Package | [`veilanon_0.0.1_aarch64.dmg`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_aarch64.dmg) | Apple Silicon (M1/M2/M3/M4) Disk Image |
| **macOS** | `.app.tar.gz` | [`veilanon_0.0.1_aarch64.app.tar.gz`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_aarch64.app.tar.gz) | macOS Apple Silicon Application Bundle Archive |
| **Linux** | `.AppImage` | [`veilanon_0.0.1_amd64.AppImage`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_amd64.AppImage) | Standalone Universal Portable Linux Binary |
| **Linux** | `.deb` Package | [`veilanon_0.0.1_amd64.deb`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_amd64.deb) | Ubuntu / Debian Native Package |
| **Linux** | `.rpm` Package | [`veilanon-0.0.1-1.x86_64.rpm`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon-0.0.1-1.x86_64.rpm) | Fedora / RHEL / openSUSE Native Package |
| **Linux** | `.tar.gz` Archive | [`veilanon.app.tar.gz`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon.app.tar.gz) | Generic Linux Portable Archive |
| **Checksums** | Manifest | [`SHA256SUMS.txt`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/SHA256SUMS.txt) | SHA-256 Cryptographic Hash Manifest |

---

## ✨ Feature Matrix

### 🔒 Cryptography & Privacy Engine
- **Direct Messages (1:1 DM):** Double Ratchet with per-message key derivation (`HKDF-SHA256`) and forward secrecy.
- **Group Channels & Spaces:** Messaging Layer Security (**MLS**, RFC 9420) tree-based key agreement with KeyPackage / Welcome onboarding.
- **At-Rest Storage Encryption:** Embedded SQLite with application-layer **AES-256-GCM** encryption and Argon2id passphrase derivation.
- **Encrypted File Vault:** Client-side ChaCha20-Poly1305 file chunks with zero plaintext filenames, metadata, or server access.
- **Disappearing Messages:** Real-time visual countdown timers (10s to 1 week) with background garbage collection and local/remote tombstoning.

### 🛡️ Public Privacy & Security Tools (Zero-Key Hub)
- **Tor & Relay Anonymity Detector:** Real-time Tor exit node and relay verification.
- **Multi-Resolver DoH Benchmark:** 5-way latency testing across Cloudflare, Google, Quad9, AdGuard, and Mullvad with censorship/tampering detection.
- **k-Anonymity Leak Checker:** Zero-knowledge SHA-1 prefix matching against HaveIBeenPwned database without transmitting passwords.
- **URLhaus Threat Scanner:** Anti-malware and phishing URL analyzer via Abuse.ch real-time threat feed.
- **Clock Skew Analyzer:** Microsecond NTP synchronization verification for cryptographic replay prevention.
- **Deterministic Privacy Avatars:** Offline procedural identicons with zero external network requests.

### 🎙️ Audio, Video & Screen Sharing
- **LiveKit Selective Forwarding Unit (SFU):** Low-latency HD voice, video, and high-framerate screen sharing (1080p 60 FPS).
- **MLS Media Key Derivation:** E2EE voice channels derive frame encryption keys directly from MLS group secrets.
- **Lifecycle & Sensor Management:** Clean WebRTC hardware stream lifecycle management, preventing stuck webcam sensors or active LED indicators when closing camera.
- **Live Device Switching:** Seamless runtime transition for microphones, speakers, and cameras with active volume meters and Push-to-Talk (PTT).

### 💬 Social & Community Experience
- **Spaces, Channels & Permissions:** Text, voice, forum, and announcement channels with granular role-based access control.
- **Message Editing:** Discord-style inline editing with a dedicated editor, Esc-to-cancel and Enter-to-save; edited marker synced across devices in real time.
- **Video Player:** Custom player with progress scrubbing, hover volume slider, mute toggle, and reliable fullscreen (native + CSS fallback).
- **Deep Linking Protocol (`veilanon://`):** Instant universal links for invites (`/invite/{code}`), profiles (`/u/{user}`), spaces, and messages.
- **Rich Media & Emojis:** GIF search via Tenor / Giphy, customizable emoji picker, 3:1 banner cropper, and animated status badges.
- **Theme-Aware Accent:** Switching a theme resets the accent to that theme's brand color; a manual accent override is honored until the next theme change.

---

## 🏛️ Architecture & Documentation

veilanon is structured with strict separation between UI, native Rust core, and external cloud adapters:

```
┌─────────────────────────────────────────────────────────────────────┐
│  UI Layer (SvelteKit / Svelte 5 + Tailwind Tokens)                  │
│  Channels, Roles, Video Grid, Settings, Privacy Hub, Chat Input     │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ Tauri IPC (invoke/emit) — type-safe
                                │ No raw cryptographic keys exposed to JS
┌───────────────────────────────▼─────────────────────────────────────┐
│  Native Core (Rust / src-tauri)                                      │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐ │
│  │ Cryptography │ │ Local Store  │ │ Offline Queue│ │ Adapters     │ │
│  │ (OpenMLS,    │ │ (SQLite +    │ │ (Auto-flush &│ │ (Supabase,   │ │
│  │  Double      │ │  AES-256-GCM)│ │  Retry Loop) │ │  LiveKit,    │ │
│  │  Ratchet)    │ │              │ │              │ │  R2 Storage) │ │
│  └──────────────┘ └──────────────┘ └──────────────┘ └──────────────┘ │
└───────┬───────────────────┬───────────────────┬──────────────────────┘
        │ Encrypted Envelopes│ Encrypted Blobs   │ Signaling / Tokens
┌───────▼─────────┐ ┌───────▼─────────┐ ┌───────▼──────────────────────┐
│ DATA PLANE      │ │ MEDIA PLANE     │ │ CONTROL PLANE                │
│ Supabase        │ │ Cloudflare R2   │ │ Supabase Auth + Edge Funcs   │
│ (PostgreSQL,    │ │ (Opaque File    │ │ (livekit-token, Realtime WS) │
│  Ciphertext Only│ │  Blobs Only)    │ │  + LiveKit Cloud SFU         │
│  Envelopes)     │ │  Chunks)        │ │                              │
└─────────────────┘ └─────────────────┘ └──────────────────────────────┘
```

For complete technical specifications, review the documentation:

| Document | Purpose & Details |
| :--- | :--- |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Layered design, IPC capability isolation, and adapter replaceability |
| [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) | Adversary definitions, trust boundaries, and mitigation guarantees |
| [`docs/PRIVACY_SCOPE.md`](docs/PRIVACY_SCOPE.md) | Granular E2EE vs. metadata visibility matrix |
| [`docs/CRYPTO_DECISIONS.md`](docs/CRYPTO_DECISIONS.md) | Decision log for audited cryptographic primitives |
| [`docs/DATA_INVENTORY.md`](docs/DATA_INVENTORY.md) | Exhaustive inventory of every stored field, encryption, and retention |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | Completed milestones, release history, and architectural backlog |
| [`docs/SUPABASE_SETUP.md`](docs/SUPABASE_SETUP.md) | Production control-plane migrations and deployment guide |
| [`docs/BOT_API.md`](docs/BOT_API.md) | Bot platform manifests and webhook integration standards |
| [`docs/DISCORD_BRIDGE.md`](docs/DISCORD_BRIDGE.md) | Webhook-only Discord bridging rules and privacy labels |
| [`TEST_INFRA.md`](TEST_INFRA.md) | Automated E2E, Rust unit, and TypeScript verification suites |

---

## 🛠️ Development & Building

### Prerequisites

- [Rust](https://rustup.rs/) (stable 1.80+ toolchain)
- [Node.js](https://nodejs.org/) 22+ & `npm`
- Platform SDKs:
  - **Windows:** Microsoft Visual Studio C++ Build Tools & WebView2
  - **macOS:** Xcode Command Line Tools
  - **Linux:** `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libxdo-dev`, `libssl-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`

### Running Locally

```bash
# 1. Clone repository
git clone https://github.com/MrSpy00/veilanon.git
cd veilanon

# 2. Install frontend dependencies
npm install

# 3. Configure environment variables
cp .env.example .env

# 4. Launch Tauri 2 Desktop App in Development Mode
npm run tauri:dev
```

### Running Test Suites

```bash
# SvelteKit & TypeScript diagnostics check (0 errors, 0 warnings)
npm run check

# Comprehensive E2E opaque-box test runner
npm run test:e2e

# Native Rust unit & cryptographic tests (113 tests)
cargo test --manifest-path src-tauri/Cargo.toml
```

### Packaging & Compiling Release Binaries

```bash
# Frontend production build
npm run build

# Tauri full native desktop binary compilation
npm run tauri:build
```

---

## 📜 Privacy Promise

veilanon commits to seven core principles on every release:

1. **Your content belongs only to you:** Message bodies, attachments, and audio/video media are encrypted on your device. The server only sees ciphertext.
2. **Zero plaintext on servers:** The database schema contains no plaintext content columns or unencrypted search indexes.
3. **No hidden telemetry:** No user tracking, analytics, or behavioral surveillance.
4. **Least metadata by default:** Presence is bucketed, read receipts and typing indicators are opt-in, link previews are sandboxed.
5. **Data portability:** Export and import your encrypted SQLite archives at any time without vendor lock-in.
6. **Audited building blocks:** Zero custom cryptographic implementations — only industry-standard, battle-tested libraries (OpenMLS, AES-GCM, Argon2id, Dalek).
7. **Honest claims:** We clearly document the exact boundaries of what is encrypted versus what metadata is visible.

---

## ⚖️ License

Distributed under the **[GNU Affero General Public License v3.0 (AGPL-3.0)](LICENSE)**.

---

## 🌐 Credits & Community

- **Official Website:** [veilanon.com](https://veilanon.com)
- **Company / Studio:** [aegisSoft](https://www.aegissoft.com.tr)
- **Author & Lead Developer:** [MrSpy00](https://github.com/MrSpy00)
- **Support & Donations:** [Buy Me a Coffee](https://buymeacoffee.com/aegissoft)
