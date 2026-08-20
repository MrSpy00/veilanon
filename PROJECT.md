# Project: VeilAnon v0.0.1

## Architecture
- **Desktop Framework**: Tauri 2 (Rust backend + SvelteKit / Svelte 5 frontend).
- **Backend Architecture (`src-tauri/`)**:
  - `src-tauri/src/lib.rs`: Tauri builder, IPC command registration, background tasks (disappearing messages purge, offline sync, Phoenix WS).
  - `src-tauri/src/crypto/`: Argon2id KDF, DeviceIdentity (Ed25519/X25519), KeyStore v2, Double Ratchet, OpenMLS 0.8 RFC 9420, ChaCha20-Poly1305 file encryption.
  - `src-tauri/src/db/`: SQLite WAL with 11 migrations (0001..0011), column-level cipher encryption, offline queue.
  - `src-tauri/src/commands/`: Domain IPC modules (`auth`, `crypto`, `messages`, `media`, `files`, `spaces`, `social`, `settings`, `logging`, `gifs`, `local_ai`, `mls`, `privacy_tools`).
- **Frontend Architecture (`src/`)**:
  - `src/lib/api/tauri.ts`: Strictly-typed IPC bridge with camelCase <-> snake_case translation.
  - `src/lib/stores/`: Svelte reactive stores (`auth`, `spaces`, `messages`, `media`, `privacyShield`, `streamerMode`, `notifications`, `theme`, `settings`).
  - `src/lib/components/`: Onboarding, Chat, Media/Calling, Spaces, Social, Settings modals, UI design token components.
- **Design Tokens**: `src/app.css` dark/AMOLED theme, presence indicators, responsive breakpoints.

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Tor & Relay Anonymity Check | Query https://check.torproject.org/api/ip and check Tor exit status | M1, M2 | ORIGINAL_REQUEST §1 |
| 2 | IP Leak & Network Diagnostic | Query 1.1.1.1/cdn-cgi/trace and api.ipify.org for public IP & ISP info | M1, M2 | ORIGINAL_REQUEST §1 |
| 3 | Encrypted DoH Test | Probe Cloudflare DoH and Google DoH endpoints for DNS leak/encryption status | M1, M2 | ORIGINAL_REQUEST §1 |
| 4 | k-Anonymity Password Leak Check | Zero-knowledge SHA1 prefix check via https://api.pwnedpasswords.com/range/{5_HEX} | M1, M2 | ORIGINAL_REQUEST §1 |
| 5 | Real-Time Malicious URL Scanner | Inspect links against https://urlhaus-api.abuse.ch/v1/url/ before opening | M1, M2 | ORIGINAL_REQUEST §1 |
| 6 | Multi-Resolver DoH & Tamper Benchmark | Query 5 independent DoH providers (Cloudflare, Google, Quad9, AdGuard, Mullvad) for latency and censorship detection | M1, M2 | ORIGINAL_REQUEST §1 |
| 7 | Deterministic Privacy Avatar Generator | Generate deterministic SVG identicons / privacy avatars offline | M1, M2 | ORIGINAL_REQUEST §1 |
| 8 | Cryptographic Clock Skew Detector | Compare local system time against https://worldtimeapi.org/api/timezone/Etc/UTC | M1, M2 | ORIGINAL_REQUEST §1 |
| 9 | Disappearing Messages Visual Countdown | Countdown timers & flame badges on disappearing messages + selector in chat input | M2 | ORIGINAL_REQUEST §3 |
| 10 | Complete Settings Panels & UX Audit | Audit and wire Audio/Video, Privacy, Permissions, Notifications, Appearance, Diagnostics | M2, M3 | ORIGINAL_REQUEST §2 |
| 11 | Keyboard Navigation & Empty States | Esc close, Arrow keys, shortcuts, responsive layout across all views | M3 | ORIGINAL_REQUEST §2 |
| 12 | Roadmap & Docs Completion | Complete docs/ROADMAP.md and document v0.0.1 release readiness | M3 | ORIGINAL_REQUEST §3, §4 |
| 13 | Backend Rust Test Expansion | Unit tests for crypto/file_enc, crypto/group, and privacy_tools | M1 | Survey Finding |
| 14 | E2E Testing Suite (Tiers 1-4) | Comprehensive requirement-driven opaque-box test suite for all features | E2E Track | ORIGINAL_REQUEST §4 |
| 15 | Adversarial Coverage Hardening | White-box stress testing and edge-case validation (Tier 5) | M4 (Final) | ORIGINAL_REQUEST §4 |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| E2E | E2E Testing Track | Design test runner, test cases (Tiers 1-4), publish TEST_READY.md | None | COMPLETED |
| M1 | Backend Privacy Tools & Tests | Implement `privacy_tools.rs`, register commands in `lib.rs`, add unit tests | None | COMPLETED |
| M2 | Frontend Privacy Tools & Chat Timers | Expose IPC in `tauri.ts`, build Privacy Hub, password check, URL scanner, chat countdown | M1 | COMPLETED |
| M3 | UX Polish, Keyboard Nav & Roadmap Docs | Shortcuts, empty states, diagnostics export, update `docs/ROADMAP.md` | M2 | COMPLETED |
| M4 | Final Milestone & Release Packaging | Pass 100% E2E tests (Tiers 1-4), cross-platform release builds, v0.0.1 tag | M3, E2E | COMPLETED |

## Interface Contracts

### Backend `privacy_tools.rs` ↔ Frontend `tauri.ts`
- `check_tor_status() -> Result<TorStatusResult, String>`
  - `TorStatusResult`: `{ is_tor: bool, ip: String }`
- `check_ip_leak() -> Result<IpLeakResult, String>`
  - `IpLeakResult`: `{ ip: String, colo: Option<String>, loc: Option<String>, tls: Option<String>, sni: Option<String>, warp: Option<String>, gateway: Option<String>, rtt_ms: u64 }`
- `check_doh_status() -> Result<DohTestResult, String>`
  - `DohTestResult`: `{ cloudflare_ok: bool, google_ok: bool, latency_cloudflare_ms: u64, latency_google_ms: u64, doh_working: bool }`
- `check_multi_doh_status() -> Result<MultiDohResult, String>`
  - `MultiDohResult`: `{ providers: Vec<DohProviderResult>, fastest_provider: Option<String>, average_latency_ms: u64, censorship_tamper_detected: bool }`
- `check_password_pwned(prefix_5_hex: String) -> Result<Vec<(String, u32)>, String>`
  - Returns matching hash suffixes and occurrence counts for k-anonymity verification.
- `scan_urlhaus(url: String) -> Result<UrlScanResult, String>`
  - `UrlScanResult`: `{ query_status: String, url_status: Option<String>, threat: Option<String>, tags: Vec<String> }`
- `fetch_link_preview(url: String) -> Result<LinkPreviewResult, String>`
  - `LinkPreviewResult`: `{ url: String, title: Option<String>, description: Option<String>, image: Option<String>, site_name: Option<String>, favicon: Option<String>, is_safe: bool }`
- `generate_privacy_avatar(seed: String) -> Result<String, String>`
  - Returns deterministic SVG XML string based on seed.
- `detect_clock_skew() -> Result<ClockSkewResult, String>`
  - `ClockSkewResult`: `{ local_timestamp: i64, server_timestamp: i64, skew_seconds: i64, is_skewed: bool }`

### Chat Disappearing Messages
- `sendMessage(channelId, content, replyToId?, disappearSeconds?)` -> calls Tauri `send_message` with `disappear_seconds`.
- `MessageItem.svelte` renders live countdown when `disappearsAt` is present.

## Code Layout
- `src-tauri/src/commands/privacy_tools.rs`: Backend privacy tools implementation.
- `src-tauri/src/commands/mod.rs` & `src-tauri/src/lib.rs`: Command exports & registration.
- `src-tauri/src/crypto/`: Crypto primitives & unit tests.
- `src/lib/api/tauri.ts`: TypeScript IPC functions.
- `src/lib/components/settings/PrivacySettings.svelte`: Privacy Hub diagnostics UI.
- `src/lib/components/onboarding/CreateIdentity.svelte`: Passphrase k-anonymity check.
- `src/lib/components/ui/ExternalLinkModal.svelte`: URLhaus safety shield.
- `src/lib/components/settings/AboutSettings.svelte`: Privacy coin donation ticker.
- `src/lib/components/chat/MessageInput.svelte`: Disappearing message timer selector.
- `src/lib/components/chat/MessageItem.svelte`: Disappearing message visual countdown badge.
- `docs/ROADMAP.md`: Roadmap documentation.
- `tests/e2e/`: E2E test runner and test cases.
