# E2E Test Infra: VeilAnon v0.0.1

## Test Philosophy
- **Opaque-box, requirement-driven**: Test suite derives strictly from user requirements in `ORIGINAL_REQUEST.md` and specifications in `PROJECT.md`, operating independently of backend/frontend internal implementation details.
- **Methodology**: Systematic 4-Tier verification combining Category-Partition, Boundary Value Analysis (BVA), Pairwise Combinatorial Testing, and Realistic Application Workloads.
- **Progressive Testability**: Verification mechanisms work from simplest primitives (pure deterministic algorithms, zero-key privacy network requests, state stores) up to full multi-user simulated E2E workflows.

## Feature Inventory & Test Coverage Mapping
| # | Feature | Source (Requirement) | Tier 1 (Coverage) | Tier 2 (Boundary) | Tier 3 (Pairwise) | Tier 4 (Scenario) |
|---|---------|---------------------|:-----------------:|:-----------------:|:-----------------:|:-----------------:|
| 1 | Tor & Relay Anonymity Check | ORIGINAL_REQUEST §1 | 5 tests | 5 tests | ✓ | ✓ |
| 2 | IP Leak & Network Diagnostic | ORIGINAL_REQUEST §1 | 5 tests | 5 tests | ✓ | ✓ |
| 3 | Encrypted DoH Test | ORIGINAL_REQUEST §1 | 5 tests | 5 tests | ✓ | ✓ |
| 4 | k-Anonymity Password Leak Check | ORIGINAL_REQUEST §1 | 5 tests | 5 tests | ✓ | ✓ |
| 5 | Real-Time Malicious URL Scanner | ORIGINAL_REQUEST §1 | 5 tests | 5 tests | ✓ | ✓ |
| 6 | Privacy Coin Market & Donation Ticker | ORIGINAL_REQUEST §1 | 5 tests | 5 tests | ✓ | ✓ |
| 7 | Deterministic Privacy Avatar Generator | ORIGINAL_REQUEST §1 | 5 tests | 5 tests | ✓ | ✓ |
| 8 | Cryptographic Clock Skew Detector | ORIGINAL_REQUEST §1 | 5 tests | 5 tests | ✓ | ✓ |
| 9 | Disappearing Messages Visual Countdown | ORIGINAL_REQUEST §3 | 5 tests | 5 tests | ✓ | ✓ |
| 10 | Complete Settings Panels & UX Audit | ORIGINAL_REQUEST §2 | 5 tests | 5 tests | ✓ | ✓ |
| 11 | Keyboard Navigation & Empty States | ORIGINAL_REQUEST §2 | 5 tests | 5 tests | ✓ | ✓ |
| 12 | Roadmap & Docs Completion | ORIGINAL_REQUEST §3, §4 | 5 tests | 5 tests | ✓ | ✓ |
| 13 | Backend Rust Test Expansion | Survey / ORIGINAL_REQUEST §4 | 5 tests | 5 tests | ✓ | ✓ |
| 14 | E2E Testing Suite (Tiers 1-4) | ORIGINAL_REQUEST §4 | 5 tests | 5 tests | ✓ | ✓ |
| 15 | Adversarial Coverage Hardening | ORIGINAL_REQUEST §4 | 5 tests | 5 tests | ✓ | ✓ |

## Test Architecture
- **Directory Layout**:
  - `tests/e2e/runner.js` / `tests/e2e/runner.ts`: Standalone execution harness and reporter.
  - `tests/e2e/tier1-feature-coverage.test.ts`: 75+ feature isolation tests (5 per feature).
  - `tests/e2e/tier2-boundary-corner.test.ts`: 75+ boundary, corner-case, and negative tests (5 per feature).
  - `tests/e2e/tier3-pairwise-combinations.test.ts`: 15+ pairwise cross-feature interaction tests.
  - `tests/e2e/tier4-application-scenarios.test.ts`: 8+ full E2E user lifecycle scenarios.
  - `tests/e2e/harness/`: Mock network providers, crypto utilities, simulated UI state stores, and assertion helpers.
- **Invocation Command**: `npm run test:e2e` or `node tests/e2e/runner.mjs` (or via Vitest `npx vitest run tests/e2e`).
- **Pass/Fail Semantics**:
  - Exit code `0` on 100% pass across all tiers.
  - Structured console summary detailing Tier 1, Tier 2, Tier 3, Tier 4 counts, execution duration, and pass rate.

## Real-World Application Scenarios (Tier 4)
| # | Scenario | Features Exercised | Complexity |
|---|----------|--------------------|------------|
| 1 | Full Zero-Trust Onboarding Journey | Identity generation, deterministic avatar, passphrase k-anonymity check, clock skew validation | High |
| 2 | Privacy Hub Audit & Network Shield | Tor exit check, IP leak check, DoH query verification, ISP mask verification | High |
| 3 | Safe Browsing & Link Inspection | Chat message received with URL, URLhaus real-time threat lookup, ExternalLinkModal safety intercept | Medium |
| 4 | Ephemeral Secure Communication | Channel creation, message composition with 30s disappearing timer, countdown trigger, simulated auto-purge | High |
| 5 | Live Crypto Donation & Settings Customization | Settings modal navigation, About panel XMR/BTC live price retrieval, donation address copy, theme toggle | Medium |
| 6 | Streamer Mode & Privacy Shield Activation | Streamer mode toggle, IP/email obfuscation, media/device permission sanitization, notification suppression | High |
| 7 | Offline Graceful Degradation & Recovery | Network drop simulation, offline queueing of messages, cached privacy tool fallbacks, reconnection sync | High |
| 8 | Complete Keyboard Navigation & Accessibility Walkthrough | Full app navigation via Esc, Arrow keys, shortcuts (Ctrl+K, Ctrl+N, Ctrl+/), empty state validation | Medium |

## Coverage Thresholds
- **Tier 1 (Feature Coverage)**: ≥75 test cases (≥5 tests for each of the 15 features).
- **Tier 2 (Boundary & Corner Cases)**: ≥75 test cases (≥5 boundary/error tests for each of the 15 features).
- **Tier 3 (Cross-Feature Combinations)**: ≥15 test cases (covering major cross-module pairs).
- **Tier 4 (Real-World Application Scenarios)**: ≥8 complex E2E user workflows.
- **Total Minimum Target**: ≥173 test cases with 100% pass rate.
