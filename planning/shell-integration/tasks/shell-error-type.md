# Task: shell-error-type

**Feature**: FE-06 Shell Integration
**File**: `src/shell.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~0.5h
**Depends on**: none
**Required by**: `completion-command`, `man-page-generator`

---

## Context

Before any shell command logic can be implemented, we need a typed error enum and a constant for the set of known built-in command names. The `ShellError` type allows command handlers to return `Result` instead of calling `std::process::exit` directly, keeping them testable. `KNOWN_BUILTINS` is the static list consulted by `cmd_man` when the named subcommand is not found in the live clap `Command` tree.

This task produces:

- `ShellError` (thiserror-derived) with one variant: `UnknownCommand(String)`.
- `KNOWN_BUILTINS: &[&str]` listing `"exec"`, `"list"`, `"describe"`, `"completion"`, `"man"`.

These are the only deliverables. No command wiring or clap integration is done here.

---

## RED — Write Failing Tests First

Add to the `#[cfg(test)]` block in `src/shell.rs`:

```rust
    #[test]
    fn test_shell_error_unknown_command_message() {
        let err = ShellError::UnknownCommand("bogus".to_string());
        assert_eq!(err.to_string(), "unknown command 'bogus'");
    }

    #[test]
    fn test_known_builtins_contains_required_commands() {
        for cmd in &["exec", "list", "describe", "completion", "man"] {
            assert!(
                KNOWN_BUILTINS.contains(cmd),
                "KNOWN_BUILTINS must contain '{cmd}'"
            );
        }
    }

    #[test]
    fn test_known_builtins_has_expected_count() {
        assert_eq!(KNOWN_BUILTINS.len(), 5);
    }
```

Run `cargo test test_shell_error test_known_builtins` — all three fail because `ShellError` and `KNOWN_BUILTINS` do not exist yet.

---

## GREEN — Implement

Replace the placeholder content in `src/shell.rs` with the error type and constant. Keep the existing function stubs so the file still compiles.

```rust
use thiserror::Error;

/// Errors produced by shell integration commands.
#[derive(Debug, Error)]
pub enum ShellError {
    #[error("unknown command '{0}'")]
    UnknownCommand(String),
}

/// The fixed set of built-in CLI command names.
///
/// `cmd_man` consults this list when the requested command name is not found
/// among the live clap subcommands, so that built-in commands that have not
/// yet been wired still produce a man page stub rather than an "unknown
/// command" error.
pub const KNOWN_BUILTINS: &[&str] = &["exec", "list", "describe", "completion", "man"];
```

Run `cargo test test_shell_error test_known_builtins` — all three pass.

---

## REFACTOR

- Confirm `ShellError` derives `Debug` (required for `Result` debug formatting in tests).
- Run `cargo clippy -- -D warnings` on `src/shell.rs`; fix any warnings.
- No structural changes expected at this stage.

---

## Verification

```bash
cargo test test_shell_error test_known_builtins 2>&1
# Expected: 3 tests pass, 0 fail.

cargo clippy -- -D warnings 2>&1
# Expected: no warnings in src/shell.rs.
```
