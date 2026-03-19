# CLAUDE.md — apcore-cli-rust

## Build & Test

- `make check` — runs fmt-check + clippy + all tests. **Must pass before considering any task complete.**
- `make build` — compile release binary to `.bin/`
- `cargo fmt --all` — format all code. **Run after every code change.**
- `cargo clippy --all-targets --all-features -- -D warnings` — zero warnings required.
- `cargo test --all-features` — run all tests.

## Code Style

- All code must pass `cargo fmt --check` (rustfmt default: 100-column line width).
- Break long function signatures, macro calls, and chained method calls across multiple lines to stay within 100 columns.
- Follow existing naming patterns:
  - `_with_limit` / `_with_*` suffix for configurable-parameter variants that delegate from a simpler public API.
  - Public wrapper functions (e.g., `build_module_command`) delegate to `_with_limit` variants with default constants.
- `#[allow(dead_code)]` only when the field is intentionally kept for API symmetry.
- Error types use `thiserror::Error` derive.
- Tests live in `#[cfg(test)] mod tests` within each source file, plus integration tests in `tests/`.

## Project Conventions

- Spec repo (single source of truth): `../apcore-cli/docs/`
- Python reference implementation: `../apcore-cli-python/`
- All values in `ConfigResolver::DEFAULTS` are `&str` (not typed) — callers parse as needed.
- Exit codes are `pub const` in `lib.rs`, matching the protocol spec.
- `apdev-rs check-chars` is part of `make check` — no non-ASCII characters in source files.

## Environment

- Rust edition: 2021
- MSRV: 1.75+
- Async runtime: tokio
