# veilanon

<p align="center">
  <img src="https://veilanon.com/brand/veilanon-logo.svg" alt="veilanon Logo" width="108" height="108" />
</p>

<p align="center">
  <strong>Privacy-First, High-Performance Open-Source Desktop Communication Platform.</strong><br>
  <em>An uncompromising, end-to-end encrypted Discord alternative engineered for modern communities.</em>
</p>

<p align="center">
  <a href="https://github.com/MrSpy00/veilanon/releases/tag/v0.0.1"><img src="https://img.shields.io/badge/release-v0.0.1-8b5cf6?style=flat-square&logo=github" alt="Release v0.0.1" /></a>
  <a href="https://veilanon.com"><img src="https://img.shields.io/badge/website-veilanon.com-6366f1?style=flat-square" alt="Website" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-AGPL--3.0-10b981?style=flat-square" alt="License" /></a>
  <a href="https://tauri.app"><img src="https://img.shields.io/badge/built%20with-Tauri%202%20%2B%20Rust-f97316?style=flat-square&logo=tauri" alt="Tauri 2" /></a>
  <a href="https://svelte.dev"><img src="https://img.shields.io/badge/frontend-Svelte%205%20%2B%20SvelteKit-ff3e00?style=flat-square&logo=svelte" alt="Svelte 5" /></a>
</p>

---

## ⚡ Overview

**veilanon** is a privacy-first, cross-platform desktop communication application built on **Tauri 2**, **Rust**, and **Svelte 5 / SvelteKit**. It combines the rich, modern, fluid user experience of Discord with the zero-knowledge mathematical guarantees of Signal-level cryptography:

- **Zero-Knowledge Architecture:** Message bodies, attachments, and call media are encrypted directly on your local device before they ever touch the network.
- **Dumb Envelope Relaying:** The control plane / server operates solely as an opaque ciphertext envelope router — it can never inspect or decrypt what you send or say.
- **Hardware-Isolated Privacy:** Local message histories and keychains are encrypted at-rest using **AES-256-GCM** and **Argon2id** key derivation.

Developed by [aegisSoft](https://www.aegissoft.com.tr/). Brand name is strictly lowercase: `veilanon`.

---

## 📦 Multi-Platform Release Downloads (v0.0.1)

All release packages are digitally signed and verified with SHA-256 checksum manifests. You can download pre-compiled binaries for your operating system directly from [GitHub Releases v0.0.1](https://github.com/MrSpy00/veilanon/releases/tag/v0.0.1) or from [veilanon.com](https://veilanon.com):

| Platform | Format / Type | Direct Download Link | Description |
| :--- | :--- | :--- | :--- |
| **Windows** | `.exe` Setup | [`veilanon_0.0.1_x64-setup.exe`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_x64-setup.exe) | NSIS Full Setup Wizard (Authenticode Signed) |
| **Windows** | `.msi` Installer | [`veilanon_0.0.1_x64_en-US.msi`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_x64_en-US.msi) | WiX Enterprise MSI Installer |
| **Windows** | `.zip` Portable | [`veilanon_0.0.1_x64.zip`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_x64.zip) | Portable Zero-Install Windows Archive |
| **Windows** | `.tar.gz` Portable | [`veilanon_0.0.1_x64.tar.gz`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_x64.tar.gz) | Compressed Windows Portable Archive |
| **macOS** | `.dmg` Package | [`veilanon_0.0.1_aarch64.dmg`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_aarch64.dmg) | Apple Silicon (M1/M2/M3/M4) Disk Image |
| **macOS** | `.app.tar.gz` | [`veilanon.app.tar.gz`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon.app.tar.gz) | macOS Universal Application Bundle Archive |
| **Linux** | `.AppImage` | [`veilanon_0.0.1_amd64.AppImage`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_amd64.AppImage) | Standalone Universal Linux Binary |
| **Linux** | `.deb` Package | [`veilanon_0.0.1_amd64.deb`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon_0.0.1_amd64.deb) | Ubuntu / Debian Native Package |
| **Linux** | `.rpm` Package | [`veilanon-0.0.1-1.x86_64.rpm`](https://github.com/MrSpy00/veilanon/releases/download/v0.0.1/veilanon-0.0.1-1.x86_64.rpm) | Fedora / RHEL / openSUSE Native Package |
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
- **Lifecycle & Sensor Management:** Clean WebRTC hardware stream lifecycle management (`stopLocalTrackOnUnpublish: true`), preventing stuck webcam sensors or active LED indicators when closing camera.
- **Live Device Switching:** Seamless runtime transition for microphones, speakers, and cameras with active volume meters and Push-to-Talk (PTT).

### 💬 Social & Community Experience
- **Spaces, Channels & Permissions:** Text, voice, forum, and announcement channels with granular role-based access control.
- **Deep Linking Protocol (`veilanon://`):** Instant universal links for invites (`/invite/{code}`), profiles (`/u/{user}`), spaces, and messages.
- **Rich Media & Emojis:** GIF search via Tenor / Giphy, customizable emoji picker, 3:1 banner cropper, and animated status badges.
- **Moderation Tools:** Complete kick, ban, unban, and temporary timeout enforcement.

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
# Fill in VEILANON_SUPABASE_URL and VEILANON_SUPABASE_ANON_KEY if testing cloud sync

# 4. Launch Tauri 2 Desktop App in Development Mode
npm run tauri:dev
```

### Running Test Suites

```bash
# SvelteKit & TypeScript diagnostics check (0 errors, 0 warnings)
npm run check

# Comprehensive E2E opaque-box test runner (173/173 tests)
npm run test:e2e

# Native Rust unit & cryptographic tests (82/82 tests)
cargo test --manifest-path src-tauri/Cargo.toml
```

### Packaging & Compiling Release Binaries

```bash
# Frontend production build
npm run build

# Tauri full native desktop binary compilation
npm run tauri:build

# Multi-platform packaging and signing utility (PowerShell)
pwsh ./scripts/package-release.ps1 -Version "0.0.1"
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
