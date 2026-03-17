# Task: bypass-logic

**ID**: bypass-logic
**Status**: pending
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~45 min
**Depends on**: annotation-extraction

---

## Objective

Implement the early-return and bypass paths inside `check_approval`:

1. If `get_requires_approval(module_def)` is `false` → return `Ok(())` immediately (no prompt, no logging).
2. If `auto_approve == true` → log INFO "Approval bypassed via --yes flag for module '{id}'." → return `Ok(())`.
3. If `env::var("APCORE_CLI_AUTO_APPROVE") == Ok("1")` → log INFO "Approval bypassed via APCORE_CLI_AUTO_APPROVE for module '{id}'." → return `Ok(())`.
4. If `APCORE_CLI_AUTO_APPROVE` is set to any value other than `"1"` or `""` → log WARN "APCORE_CLI_AUTO_APPROVE is set to '{val}', expected '1'. Ignoring." → fall through (do not bypass).

Bypass priority: `--yes` flag beats env var (evaluated first). The env var `""` is silently ignored (not a warning). Only exact `"1"` activates bypass.

At the end of this task, `check_approval` still calls `todo!()` for the TTY/prompt paths. Those are handled in subsequent tasks.

---

## RED — Write failing tests first

These tests do not require a real TTY or async runtime — they exercise only the synchronous bypass logic. Use `tokio::test` since `check_approval` is `async fn`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn module(requires: bool) -> serde_json::Value {
        json!({
            "module_id": "test-module",
            "annotations": { "requires_approval": requires }
        })
    }

    // Skip cases (requires_approval != true)

    #[tokio::test]
    async fn skip_when_requires_approval_false() {
        let result = check_approval(&json!({"annotations": {"requires_approval": false}}), false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn skip_when_no_annotations() {
        let result = check_approval(&json!({}), false).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn skip_when_requires_approval_string_true() {
        let result = check_approval(&json!({"annotations": {"requires_approval": "true"}}), false).await;
        assert!(result.is_ok());
    }

    // Bypass: --yes flag

    #[tokio::test]
    async fn bypass_auto_approve_true() {
        let result = check_approval(&module(true), true).await;
        assert!(result.is_ok(), "auto_approve=true must bypass");
    }

    // Bypass: env var

    #[tokio::test]
    async fn bypass_env_var_one() {
        // Isolate env var — use std::env::set_var carefully in tests.
        // Note: if running tests in parallel, prefer serial or a mutex.
        unsafe { std::env::set_var("APCORE_CLI_AUTO_APPROVE", "1") };
        let result = check_approval(&module(true), false).await;
        unsafe { std::env::remove_var("APCORE_CLI_AUTO_APPROVE") };
        assert!(result.is_ok(), "APCORE_CLI_AUTO_APPROVE=1 must bypass");
    }

    // Priority: --yes beats env var

    #[tokio::test]
    async fn yes_flag_priority_over_env_var() {
        unsafe { std::env::set_var("APCORE_CLI_AUTO_APPROVE", "1") };
        let result = check_approval(&module(true), true).await;
        unsafe { std::env::remove_var("APCORE_CLI_AUTO_APPROVE") };
        // Both set; result is Ok because --yes is checked first.
        assert!(result.is_ok());
        // The tracing log is NOT directly assertable in unit tests without a
        // subscriber mock. Log verification is deferred to integration tests.
    }

    // Env var set to non-"1" value — must NOT bypass (falls through to TTY check)
    // This test is intentionally left as a compile-check only; the TTY path
    // is implemented in the non-tty-rejection task. Add full assertion there.
    #[tokio::test]
    #[should_panic] // Still calls todo!() in TTY path
    async fn env_var_not_one_does_not_bypass() {
        unsafe { std::env::set_var("APCORE_CLI_AUTO_APPROVE", "true") };
        let _ = check_approval(&module(true), false).await;
        unsafe { std::env::remove_var("APCORE_CLI_AUTO_APPROVE") };
    }
}
```

Run: `cargo test --lib approval::tests` — compilation should succeed; bypass tests will fail (function is still `todo!()`).

---

## GREEN — Implement

Replace the stub `check_approval` body:

```rust
pub async fn check_approval(
    module_def: &serde_json::Value,
    auto_approve: bool,
) -> Result<(), ApprovalError> {
    if !get_requires_approval(module_def) {
        return Ok(());
    }

    let module_id = get_module_id(module_def);

    // Bypass: --yes flag (highest priority)
    if auto_approve {
        tracing::info!(
            "Approval bypassed via --yes flag for module '{}'.",
            module_id
        );
        return Ok(());
    }

    // Bypass: APCORE_CLI_AUTO_APPROVE env var
    match std::env::var("APCORE_CLI_AUTO_APPROVE").as_deref() {
        Ok("1") => {
            tracing::info!(
                "Approval bypassed via APCORE_CLI_AUTO_APPROVE for module '{}'.",
                module_id
            );
            return Ok(());
        }
        Ok("") | Err(_) => {
            // Not set or empty — fall through silently.
        }
        Ok(val) => {
            tracing::warn!(
                "APCORE_CLI_AUTO_APPROVE is set to '{}', expected '1'. Ignoring.",
                val
            );
        }
    }

    // TTY check and prompt — implemented in subsequent tasks.
    todo!("TTY check and prompt")
}
```

Run: `cargo test --lib approval::tests` — all bypass tests must pass; TTY tests still panic via `todo!()`.

---

## REFACTOR

- Confirm `std::env::var` returns `Err` when the variable is unset (not `Ok("")`) — this is correct Rust behavior.
- The `as_deref()` call converts `Result<String, _>` to `Result<&str, _>`, enabling pattern matching on string literals.

---

## Verification

```
cargo test --lib approval::tests::skip_when_requires_approval_false
cargo test --lib approval::tests::skip_when_no_annotations
cargo test --lib approval::tests::skip_when_requires_approval_string_true
cargo test --lib approval::tests::bypass_auto_approve_true
cargo test --lib approval::tests::bypass_env_var_one
cargo test --lib approval::tests::yes_flag_priority_over_env_var
```

---

## Files Modified

- `src/approval.rs` — replace `check_approval` stub body with bypass logic
