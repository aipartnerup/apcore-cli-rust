# Task: tracing-setup

**Feature**: FE-01 Core Dispatcher
**File**: `src/main.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~2h
**Depends on**: nothing
**Required by**: `create-cli-and-main`

---

## Context

The Python implementation uses Python's `logging` module with three-tier log level precedence:

1. `--log-level` CLI flag (runtime override, highest priority)
2. `APCORE_CLI_LOGGING_LEVEL` env var (CLI-specific)
3. `APCORE_LOGGING_LEVEL` env var (global fallback)
4. Default: `WARNING`

The Rust implementation uses `tracing` + `tracing-subscriber` (already in `Cargo.toml`). The equivalent is:

1. `--log-level` flag: reloads the filter at runtime via `tracing_subscriber::reload`.
2. Env vars: read at startup to construct the initial `EnvFilter`.
3. Default: `warn` (tracing uses lowercase level names).

Additionally, the Python implementation silences the `apcore` logger at startup unless the resolved level is INFO or lower. In Rust, this is done with a directive like `apcore=error` in the initial filter, upgraded to the user-requested level when `--log-level DEBUG` or `--log-level INFO` is passed.

This task is extracted separately because the tracing subscriber must be initialised exactly once, before any other code runs, which requires careful placement relative to argument parsing.

---

## RED — Write Failing Tests First

Tracing initialisation is difficult to unit test directly (subscriber can only be set once per process). Use integration tests that verify observable behaviour:

```rust
// tests/test_e2e.rs — tracing-related:

#[test]
fn test_default_log_level_no_debug_output() {
    // With default level (WARNING), DEBUG messages must not appear.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_apcore-cli"))
        .args(&["--extensions-dir", "./tests/fixtures/extensions", "--help"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !stderr.contains("DEBUG") && !stderr.contains("TRACE"),
        "debug output must not appear at default level: {stderr}"
    );
}

#[test]
fn test_apcore_cli_logging_level_env_var() {
    // APCORE_CLI_LOGGING_LEVEL=ERROR should suppress INFO and DEBUG.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_apcore-cli"))
        .env("APCORE_CLI_LOGGING_LEVEL", "ERROR")
        .args(&["--extensions-dir", "./tests/fixtures/extensions", "--help"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!stderr.contains("INFO"), "INFO must be suppressed: {stderr}");
}

#[test]
fn test_apcore_cli_logging_level_overrides_global() {
    // APCORE_CLI_LOGGING_LEVEL takes priority over APCORE_LOGGING_LEVEL.
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_apcore-cli"))
        .env("APCORE_CLI_LOGGING_LEVEL", "ERROR")
        .env("APCORE_LOGGING_LEVEL", "DEBUG")
        .args(&["--extensions-dir", "./tests/fixtures/extensions", "--help"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    // ERROR level: DEBUG messages from APCORE_LOGGING_LEVEL must not appear.
    assert!(!stderr.contains("DEBUG"), "CLI-specific level must win: {stderr}");
}

#[test]
fn test_log_level_flag_accepted() {
    let out = run_apcore(&[
        "--extensions-dir", "./tests/fixtures/extensions",
        "--log-level", "ERROR",
        "--help",
    ]);
    assert_eq!(out.status.code(), Some(0));
}
```

Run `cargo test --test test_e2e tracing` — may pass vacuously if tracing is not emitting; focus on ensuring no unexpected output and that the flag is accepted without error.

---

## GREEN — Implement

Extract `init_tracing()` into a dedicated function in `src/main.rs` (already planned in `create-cli-and-main`). Add runtime reload support for `--log-level`:

```rust
use std::sync::OnceLock;
use tracing_subscriber::{EnvFilter, reload};

// Reload handle stored globally so --log-level can update the filter at runtime.
static TRACING_RELOAD_HANDLE: OnceLock<reload::Handle<EnvFilter, tracing_subscriber::Registry>> =
    OnceLock::new();

pub fn init_tracing() {
    let cli_level = std::env::var("APCORE_CLI_LOGGING_LEVEL")
        .unwrap_or_default()
        .to_lowercase();
    let global_level = std::env::var("APCORE_LOGGING_LEVEL")
        .unwrap_or_default()
        .to_lowercase();

    let level_str = if !cli_level.is_empty() {
        cli_level
    } else if !global_level.is_empty() {
        global_level
    } else {
        "warn".to_string()
    };

    // Silence upstream apcore crate unless user explicitly requests verbose output.
    let filter_str = if level_str == "debug" || level_str == "trace" || level_str == "info" {
        format!("{level_str},apcore={level_str}")
    } else {
        format!("{level_str},apcore=error")
    };

    let filter = EnvFilter::try_new(&filter_str)
        .unwrap_or_else(|_| EnvFilter::new("warn,apcore=error"));

    let (filter_layer, reload_handle) = reload::Layer::new(filter);
    let _ = TRACING_RELOAD_HANDLE.set(reload_handle);

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(tracing_subscriber::fmt::layer().with_target(false))
        .init();
}

/// Apply a runtime log level override from the --log-level flag.
pub fn apply_log_level_flag(level: &str) {
    let level_lower = level.to_lowercase();
    let filter_str = if level_lower == "debug" || level_lower == "trace" || level_lower == "info" {
        format!("{level_lower},apcore={level_lower}")
    } else {
        format!("{level_lower},apcore=error")
    };
    if let Some(handle) = TRACING_RELOAD_HANDLE.get() {
        if let Ok(new_filter) = EnvFilter::try_new(&filter_str) {
            let _ = handle.modify(|f| *f = new_filter);
        }
    }
}
```

Call `apply_log_level_flag` in `main` after parsing `--log-level` from `ArgMatches`:

```rust
if let Some(level) = matches.get_one::<String>("log-level") {
    apply_log_level_flag(level);
}
```

---

## REFACTOR

- Ensure `TRACING_RELOAD_HANDLE` is accessible from `main.rs` only (not pub in lib).
- Validate that `tracing_subscriber` version in `Cargo.toml` supports `reload::Layer` (requires `tracing-subscriber` with `reload` feature). Update `Cargo.toml` if needed:
  ```toml
  tracing-subscriber = { version = "0.3", features = ["env-filter", "reload"] }
  ```
- Run `cargo clippy -- -D warnings`.

---

## Verification

```bash
cargo test --test test_e2e 2>&1 | grep -E "tracing|log_level"

# Manual check — DEBUG output should appear with --log-level DEBUG:
./target/debug/apcore-cli \
    --extensions-dir tests/fixtures/extensions \
    --log-level DEBUG \
    --help 2>&1 | head -20
```
