# Approval Gate — Feature Overview

**Feature ID**: FE-03
**Status**: planned
**Language**: Rust 2021
**Module**: `src/approval.rs`
**Exit code for all denial/timeout/non-TTY outcomes**: 46 (`EXIT_APPROVAL_DENIED`)

---

## What It Does

The Approval Gate is a TTY-aware Human-in-the-Loop (HITL) middleware. Before a module executes, `check_approval` inspects the module's `annotations.requires_approval` field. If it is exactly `true` (strict boolean — the string `"true"` and integer `1` are NOT accepted), the gate:

1. Checks bypass mechanisms in priority order (`--yes` flag, then `APCORE_CLI_AUTO_APPROVE=1`).
2. Rejects non-interactive callers (no TTY, no bypass) with exit 46 and an informative stderr message.
3. Prompts interactive users with a 60-second timed `[y/N]` confirmation.

User must type `y` or `yes` (case-insensitive). Any other input — including pressing Enter (default deny) — exits with code 46.

---

## Key Rust Differences vs Python

| Concern | Python | Rust |
|---------|--------|------|
| Timeout mechanism | `signal.SIGALRM` (Unix) / `threading.Timer` (Windows) | `tokio::select!` racing `spawn_blocking` + `sleep` |
| TTY detection | `sys.stdin.isatty()` | `std::io::stdin().is_terminal()` (stable 1.70) |
| Error type | `ApprovalTimeoutError(Exception)` + `sys.exit(46)` | `ApprovalError` enum (thiserror); caller does `process::exit(46)` |
| Platform code | Separate `_prompt_unix` / `_prompt_windows` | None — tokio + IsTerminal are cross-platform |

---

## Error Variants

```rust
pub enum ApprovalError {
    Denied { module_id: String },          // user typed n/N/Enter
    NonInteractive { module_id: String },  // no TTY, no bypass
    Timeout { module_id: String, seconds: u64 },  // 60s elapsed
}
```

All three map to exit code 46.

---

## Tasks

| ID | Title | Estimate | Status |
|----|-------|----------|--------|
| `error-types` | Define `ApprovalError` enum with thiserror | ~30 min | pending |
| `annotation-extraction` | Strict bool check + message/ID helpers | ~45 min | pending |
| `bypass-logic` | `--yes` flag and `APCORE_CLI_AUTO_APPROVE` bypass paths | ~45 min | pending |
| `non-tty-rejection` | TTY detection and `NonInteractive` error | ~30 min | pending |
| `tty-prompt-timeout` | `tokio::select!` prompt with 60s timeout | ~1.5 hr | pending |
| `cli-integration` | Wire into exec path; map error → exit 46 | ~45 min | pending |

**Execution order**: error-types → annotation-extraction → bypass-logic → non-tty-rejection → tty-prompt-timeout → cli-integration

**Total estimate**: ~4.5 hours

---

## Bypass Rules (unchanged from Python)

| Bypass | Condition | Priority |
|--------|-----------|----------|
| `--yes` CLI flag | `auto_approve == true` | 1 (highest) |
| `APCORE_CLI_AUTO_APPROVE=1` | exact string `"1"` | 2 |
| Invalid env value | any value other than `""` and `"1"` | WARN logged, no bypass |

---

## Acceptance Criteria Summary

- All 13 test cases from the feature spec (T-APPR-01 through T-APPR-13) pass.
- `cargo test --lib approval` passes with zero failures.
- `cargo test --test approval_integration` passes.
- `cargo clippy -- -D warnings` produces zero warnings.
- `cargo build` succeeds.

See `plan.md` for full acceptance criteria checklist.
