# Overview: Config Resolver (Rust Port)

**Feature ID**: FE-07
**Status**: planned
**Priority**: P0
**Target Language**: Rust 2021

---

## Overview

This feature ports the `ConfigResolver` component from the Python `apcore-cli` implementation to Rust. The resolver provides a 4-tier configuration precedence hierarchy used throughout `apcore-cli`: CLI flag > environment variable > YAML config file > built-in default. It is a foundational, dependency-free module that must be complete before any feature that reads user-configurable values can be finalized.

The Rust project already has a scaffolded `src/config.rs` with the correct struct definition, public API shape, and inline `#[cfg(test)]` stubs, as well as a partial integration test file at `tests/test_config.rs`. This feature plan fills in the three `todo!()` method bodies and completes all integration test assertions.

---

## Scope

**In scope:**
- Implementing `ConfigResolver::resolve` (4-tier precedence logic)
- Implementing `ConfigResolver::load_config_file` (YAML reading, malformed-file handling)
- Implementing `ConfigResolver::flatten_dict` and private `flatten_yaml_value` helper
- Correcting the `DEFAULTS` constant (`logging.level` value and adding `cli.auto_approve`)
- Replacing all `assert!(false, "not implemented")` stubs in `tests/test_config.rs`
- Covering all 9 spec test cases (T-CFG-01 through T-CFG-09)

**Out of scope:**
- Changing the public API surface of `ConfigResolver` (struct fields, method signatures)
- Adding new configuration keys beyond the five defined in the feature spec
- Integration with `main.rs` or CLI flag parsing (those are separate features)
- Async I/O (config file reading is synchronous by design)

---

## Technology Stack

| Concern | Crate / Feature | Version |
|---|---|---|
| YAML parsing | `serde_yaml` | 0.9 (already in `Cargo.toml`) |
| Struct serialization | `serde` with `derive` feature | 1.x |
| JSON value (public `flatten_dict`) | `serde_json` | 1.x |
| Warning emission | `tracing` (`warn!` macro) | 0.1 |
| Temp files in tests | `tempfile` | 3.x (already in `[dev-dependencies]`) |
| Async runtime | `tokio` | 1.x (not used by this feature; config resolution is sync) |
| Error handling | `thiserror`, `anyhow` | 1.x (not used by this feature; errors are absorbed into `Option`) |

---

## Task Execution Order

| # | Task File | Description | Status |
|---|---|---|---|
| 1 | `tasks/models.md` | Audit and finalise defaults, types, and public API surface | pending |
| 2 | `tasks/resolver.md` | Implement `resolve`, `load_config_file`, and `flatten_dict` | pending |
| 3 | `tasks/tests.md` | Complete integration test assertions in `tests/test_config.rs` | pending |

Tasks must be executed in order: `models` → `resolver` → `tests`.

---

## Progress

- [ ] `models` — Audit DEFAULTS and types
- [ ] `resolver` — Implement all three stubbed methods
- [ ] `tests` — Complete all integration test assertions

---

## Reference Documents

| Document | Path |
|---|---|
| Feature spec (FE-07) | `apcore-cli/docs/features/config-resolver.md` |
| Python reference implementation | `apcore-cli-python/src/apcore_cli/config.py` |
| Python planning doc | `apcore-cli-python/planning/config-resolver.md` |
| Type mapping spec | `apcore/docs/spec/type-mapping.md` |
| Rust target module | `apcore-cli-rust/src/config.rs` |
| Rust integration tests | `apcore-cli-rust/tests/test_config.rs` |
| Shared test helpers | `apcore-cli-rust/tests/common/mod.rs` |
| Implementation plan | `apcore-cli-rust/planning/config-resolver/plan.md` |
