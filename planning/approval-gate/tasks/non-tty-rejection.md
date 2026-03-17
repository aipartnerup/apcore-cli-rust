# Task: non-tty-rejection

**ID**: non-tty-rejection
**Status**: pending
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~30 min
**Depends on**: bypass-logic

---

## Objective

Implement the non-TTY detection path in `check_approval`. When `std::io::stdin().is_terminal()` returns `false` and no bypass is active, the function must:

1. Write the error message to stderr:
   `"Error: Module '{id}' requires approval but no interactive terminal is available. Use --yes or set APCORE_CLI_AUTO_APPROVE=1 to bypass."`
2. Log `tracing::error!("Non-interactive environment, no bypass provided for module '{id}'.")`.
3. Return `Err(ApprovalError::NonInteractive { module_id })`.

The caller in `main.rs` maps this to `std::process::exit(46)`.

### TTY Detection

```rust
use std::io::IsTerminal;
let is_tty = std::io::stdin().is_terminal();
```

`std::io::IsTerminal` is stable since Rust 1.70. No platform-specific code is required.

### Testing Strategy

Direct unit tests cannot force `stdin().is_terminal()` to return `false` portably. Use a thin injectable abstraction: introduce a `fn check_approval_with_tty(module_def, auto_approve, is_tty: bool)` internal helper that accepts `is_tty` as a parameter. `check_approval` calls it with `std::io::stdin().is_terminal()`. Tests call `check_approval_with_tty` directly with `is_tty = false`.

---

## RED — Write failing tests first

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn module_requiring_approval() -> serde_json::Value {
        json!({
            "module_id": "test-module",
            "annotations": { "requires_approval": true }
        })
    }

    // Non-TTY rejection

    #[tokio::test]
    async fn non_tty_no_bypass_returns_non_interactive_error() {
        let result = check_approval_with_tty(&module_requiring_approval(), false, false).await;
        match result {
            Err(ApprovalError::NonInteractive { module_id }) => {
                assert_eq!(module_id, "test-module");
            }
            other => panic!("expected NonInteractive error, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn non_tty_with_yes_flag_bypasses_before_tty_check() {
        // --yes bypass is evaluated before TTY check, so non-TTY + auto_approve=true succeeds.
        let result = check_approval_with_tty(&module_requiring_approval(), true, false).await;
        assert!(result.is_ok(), "auto_approve bypasses TTY check");
    }

    #[tokio::test]
    async fn non_tty_with_env_var_bypasses_before_tty_check() {
        unsafe { std::env::set_var("APCORE_CLI_AUTO_APPROVE", "1") };
        let result = check_approval_with_tty(&module_requiring_approval(), false, false).await;
        unsafe { std::env::remove_var("APCORE_CLI_AUTO_APPROVE") };
        assert!(result.is_ok(), "env var bypass happens before TTY check");
    }

    #[tokio::test]
    async fn non_tty_env_var_not_one_returns_non_interactive() {
        unsafe { std::env::set_var("APCORE_CLI_AUTO_APPROVE", "true") };
        let result = check_approval_with_tty(&module_requiring_approval(), false, false).await;
        unsafe { std::env::remove_var("APCORE_CLI_AUTO_APPROVE") };
        // env var "true" is not a valid bypass; non-TTY path fires.
        assert!(matches!(result, Err(ApprovalError::NonInteractive { .. })));
    }
}
```

Run: `cargo test --lib approval::tests` — tests will fail to compile (`check_approval_with_tty` does not exist yet).

---

## GREEN — Implement

Refactor `check_approval` to delegate to an internal async function that accepts `is_tty`:

```rust
pub async fn check_approval(
    module_def: &serde_json::Value,
    auto_approve: bool,
) -> Result<(), ApprovalError> {
    use std::io::IsTerminal;
    let is_tty = std::io::stdin().is_terminal();
    check_approval_with_tty(module_def, auto_approve, is_tty).await
}

// Internal: accepts is_tty for testability.
async fn check_approval_with_tty(
    module_def: &serde_json::Value,
    auto_approve: bool,
    is_tty: bool,
) -> Result<(), ApprovalError> {
    if !get_requires_approval(module_def) {
        return Ok(());
    }

    let module_id = get_module_id(module_def);

    // Bypass: --yes flag
    if auto_approve {
        tracing::info!("Approval bypassed via --yes flag for module '{}'.", module_id);
        return Ok(());
    }

    // Bypass: APCORE_CLI_AUTO_APPROVE
    match std::env::var("APCORE_CLI_AUTO_APPROVE").as_deref() {
        Ok("1") => {
            tracing::info!(
                "Approval bypassed via APCORE_CLI_AUTO_APPROVE for module '{}'.",
                module_id
            );
            return Ok(());
        }
        Ok("") | Err(_) => {}
        Ok(val) => {
            tracing::warn!(
                "APCORE_CLI_AUTO_APPROVE is set to '{}', expected '1'. Ignoring.",
                val
            );
        }
    }

    // Non-TTY rejection
    if !is_tty {
        eprintln!(
            "Error: Module '{}' requires approval but no interactive terminal is available. \
             Use --yes or set APCORE_CLI_AUTO_APPROVE=1 to bypass.",
            module_id
        );
        tracing::error!(
            "Non-interactive environment, no bypass provided for module '{}'.",
            module_id
        );
        return Err(ApprovalError::NonInteractive { module_id });
    }

    // TTY prompt — implemented in tty-prompt-timeout task.
    todo!("TTY prompt with timeout")
}
```

Run: `cargo test --lib approval::tests` — non-TTY tests must pass.

---

## REFACTOR

- Confirm that bypasses (auto_approve, env var) short-circuit before the `is_tty` check. Tests `non_tty_with_yes_flag_bypasses_before_tty_check` and `non_tty_with_env_var_bypasses_before_tty_check` verify this explicitly.
- Confirm the stderr message matches T-APPR-04 verbatim.

---

## Verification

```
cargo test --lib approval::tests::non_tty_no_bypass_returns_non_interactive_error
cargo test --lib approval::tests::non_tty_with_yes_flag_bypasses_before_tty_check
cargo test --lib approval::tests::non_tty_with_env_var_bypasses_before_tty_check
cargo test --lib approval::tests::non_tty_env_var_not_one_returns_non_interactive
```

---

## Files Modified

- `src/approval.rs` — refactor `check_approval` into `check_approval` + `check_approval_with_tty`; add non-TTY rejection
