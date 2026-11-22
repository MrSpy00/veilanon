# Contributing to veilanon

Thanks for taking the time to contribute.

## Development Setup

```bash
npm install
npm run dev
```

For the Rust / Tauri desktop target:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Checks Before Opening a PR

- Run Rust tests (`cargo test --manifest-path src-tauri/Cargo.toml`).
- Run TypeScript / E2E tests (`npm run test:e2e`) when modifying client flows.
- Keep cryptographic code strictly on audited primitives (`OpenMLS`, `Double Ratchet`, `AES-256-GCM`, `Argon2id`) — do not introduce custom ciphers.
- Keep commits small and focused (`feat:`, `fix:`, `docs:`, `test:`).

## Reporting Security Issues

Please report security vulnerabilities privately to `aegissoft0@gmail.com` instead of opening a public issue.
