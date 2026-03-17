# Overview: Security Manager (Rust Port)

**Feature ID**: FE-05
**Status**: planned
**Priority**: P1 (Auth, Encryption, Audit) / P2 (Sandbox)
**Target Language**: Rust 2021

---

## Overview

This feature ports the `security` sub-package from the Python `apcore-cli` implementation to Rust. It provides four security components that are already scaffolded (with `todo!()` stubs) in `src/security/`:

| Component | Rust Module | Spec Requirement |
|---|---|---|
| API key authentication | `src/security/auth.rs` | FR-SEC-001 |
| Encrypted config storage | `src/security/config_encryptor.rs` | FR-SEC-002 |
| Append-only audit logging | `src/security/audit.rs` | FR-SEC-003 |
| Subprocess execution sandbox | `src/security/sandbox.rs` | FR-SEC-004 |

The Rust project already has correct public API shapes, error types, and inline test stubs with `assert!(false, "not implemented")`. This plan fills in all stub bodies and adds the integration test suite.

---

## Scope

**In scope:**
- Implementing all four `todo!()` method bodies in `src/security/`
- Implementing `run_sandbox_subprocess()`, `encode_result()`, `decode_result()` in `src/_sandbox_runner.rs`
- Wiring the `--internal-sandbox-runner` argv intercept in `src/main.rs`
- Creating `tests/test_security.rs` covering T-SEC-01 through T-SEC-18
- Adding `chrono`, `base64`, and `gethostname` to `Cargo.toml`
- Moving `tempfile` from `[dev-dependencies]` to `[dependencies]` (required at runtime by `Sandbox`)

**Out of scope:**
- Changing any public API surface (struct fields, method signatures, error enum variants)
- Integrating security components into the CLI exec flow (that belongs to the `core-dispatcher` feature)
- Adding a `--sandbox` CLI flag (belongs to `cli.rs` / Clap setup)
- Async `AuditLogger` (write is synchronous per spec; sync `BufWriter<File>` is correct)

---

## Technology Stack

| Concern | Crate / API | Version |
|---|---|---|
| OS keyring | `keyring` | 2.x |
| AES-256-GCM | `aes-gcm` | 0.10 |
| PBKDF2 key derivation | `pbkdf2` + `sha2` | 0.12 / 0.10 |
| Nonce generation | `aes_gcm::aead::OsRng` | (bundled) |
| Base64 encoding | `base64` | 0.22 (add to Cargo.toml) |
| Hostname | `gethostname` | 0.4 (add to Cargo.toml) |
| Timestamps | `chrono` with UTC | 0.4 (add to Cargo.toml) |
| JSONL write | `serde_json` + `std::io::BufWriter<File>` | 1.x |
| Subprocess | `tokio::process::Command` | (bundled with tokio) |
| Timeout | `tokio::time::timeout` | (bundled with tokio) |
| Temp directory (sandbox) | `tempfile::TempDir` | 3.x (move to [dependencies]) |
| Error types | `thiserror` | 1.x |

---

## Task Execution Order

| # | Task File | Description | Status |
|---|---|---|---|
| 1 | `tasks/config-encryptor.md` | Implement `ConfigEncryptor`: keyring probe, AES-256-GCM, PBKDF2 | pending |
| 2 | `tasks/audit.md` | Implement `AuditLogger`: JSONL append, salted SHA-256 hash, user fallback | pending |
| 3 | `tasks/sandbox.md` | Implement `Sandbox` + `_sandbox_runner`: subprocess isolation, env whitelist | pending |
| 4 | `tasks/auth.md` | Implement `AuthProvider`: key resolution, Bearer injection, 401/403 mapping | pending |
| 5 | `tasks/integration.md` | Write and pass `tests/test_security.rs` (T-SEC-01 through T-SEC-18) | pending |

Tasks 1, 2, and 3 have no dependencies on each other and can be worked in parallel. Task 4 depends on task 1. Task 5 depends on all four preceding tasks.

---

## Progress

- [ ] `config-encryptor` — Implement ConfigEncryptor + key derivation
- [ ] `audit` — Implement AuditLogger
- [ ] `sandbox` — Implement Sandbox + _sandbox_runner
- [ ] `auth` — Implement AuthProvider
- [ ] `integration` — Write and pass integration tests

---

## Key Design Decisions

| Decision | Rationale |
|---|---|
| Keyring preferred; AES-GCM fallback | Matches Python reference; `_force_aes: bool` field enables test-only bypass |
| Wire format: `nonce[12] ‖ tag[16] ‖ ciphertext` under `enc:base64` prefix | Byte-for-byte compatible with Python reference implementation |
| PBKDF2 salt: `b"apcore-cli-config-v1"`, iterations: 100 000 | Matches Python `hashlib.pbkdf2_hmac` call; machine-specific key via hostname:username |
| User name via `USER` → `LOGNAME` → `"unknown"` | No `whoami` crate needed; safe on all platforms; matches Python fallback chain |
| Salted SHA-256 per invocation for `input_hash` | Prevents cross-invocation correlation; fresh 16-byte random salt from `OsRng` |
| Single binary with `--internal-sandbox-runner` subcommand | Avoids a separate sandbox binary; intercept argv before Clap to avoid unknown-flag error |
| Subprocess env via `.env_clear()` + selective re-add | Only clean way to whitelist with `tokio::process::Command` |
| `log_execution` returns `()`, not `Result` | Write failures are non-fatal by spec; caller must not handle errors |

---

## Reference Documents

| Document | Path |
|---|---|
| Feature spec (FE-05) | `apcore-cli/docs/features/security.md` |
| Python reference: audit | `apcore-cli-python/src/apcore_cli/security/audit.py` |
| Python reference: auth | `apcore-cli-python/src/apcore_cli/security/auth.py` |
| Python reference: config_encryptor | `apcore-cli-python/src/apcore_cli/security/config_encryptor.py` |
| Python reference: sandbox | `apcore-cli-python/src/apcore_cli/security/sandbox.py` |
| Python planning | `apcore-cli-python/planning/security-manager.md` |
| Type mapping spec | `apcore/docs/spec/type-mapping.md` |
| Rust stubs | `apcore-cli-rust/src/security/{audit,auth,config_encryptor,sandbox}.rs` |
| Sandbox runner stub | `apcore-cli-rust/src/_sandbox_runner.rs` |
| Implementation plan | `apcore-cli-rust/planning/security/plan.md` |
