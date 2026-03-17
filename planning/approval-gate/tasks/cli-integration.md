# Task: cli-integration

**ID**: cli-integration
**Status**: pending
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~45 min
**Depends on**: tty-prompt-timeout

---

## Objective

Wire `check_approval` into the CLI execution path so that any module with `annotations.requires_approval: true` passes through the approval gate before execution. Map all `ApprovalError` variants to exit code 46 (`EXIT_APPROVAL_DENIED`).

This task also updates the public API surface in `src/lib.rs` to reflect the removal of `ApprovalTimeoutError` and the new `check_approval` signature.

---

## RED — Write failing tests first

Integration tests live in `tests/approval_integration.rs`. They test the end-to-end flow: approval gate → exit code mapping.

```rust
// tests/approval_integration.rs

use apcore_cli::{ApprovalError, EXIT_APPROVAL_DENIED};
use serde_json::json;

/// Helper: map ApprovalError to exit code (mirrors main.rs logic).
fn exit_code_for(e: &ApprovalError) -> i32 {
    match e {
        ApprovalError::Denied { .. }
        | ApprovalError::NonInteractive { .. }
        | ApprovalError::Timeout { .. } => EXIT_APPROVAL_DENIED,
    }
}

#[tokio::test]
async fn all_approval_errors_map_to_exit_46() {
    let denied = ApprovalError::Denied { module_id: "m".into() };
    let non_interactive = ApprovalError::NonInteractive { module_id: "m".into() };
    let timeout = ApprovalError::Timeout { module_id: "m".into(), seconds: 60 };

    assert_eq!(exit_code_for(&denied), 46);
    assert_eq!(exit_code_for(&non_interactive), 46);
    assert_eq!(exit_code_for(&timeout), 46);
    assert_eq!(EXIT_APPROVAL_DENIED, 46);
}

#[tokio::test]
async fn module_without_requires_approval_skips_gate() {
    // No annotations field → gate must return Ok immediately.
    let module_def = json!({"module_id": "open-module"});
    let result = apcore_cli::approval::check_approval(&module_def, false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn module_with_requires_approval_false_skips_gate() {
    let module_def = json!({
        "module_id": "open-module",
        "annotations": {"requires_approval": false}
    });
    let result = apcore_cli::approval::check_approval(&module_def, false).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn module_with_auto_approve_true_skips_gate() {
    let module_def = json!({
        "module_id": "guarded-module",
        "annotations": {"requires_approval": true}
    });
    let result = apcore_cli::approval::check_approval(&module_def, true).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn module_non_tty_no_bypass_returns_approval_error() {
    // Uses the internal check_approval_with_tty for non-TTY simulation.
    // In the integration context, we verify via the public check_approval
    // but cannot force is_terminal()=false portably here.
    // This test validates the error type returned when running in CI (non-TTY).
    //
    // Note: this test is environment-sensitive. In a real TTY environment it
    // will invoke the interactive prompt. Run with CI=true or in a pipe to
    // guarantee non-TTY behavior:
    //   echo "" | cargo test approval_integration
    //
    // This test is skipped if stdin is a TTY.
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        eprintln!("Skipping non-TTY integration test (stdin is a TTY).");
        return;
    }
    let module_def = json!({
        "module_id": "guarded-module",
        "annotations": {"requires_approval": true}
    });
    let result = apcore_cli::approval::check_approval(&module_def, false).await;
    assert!(matches!(result, Err(ApprovalError::NonInteractive { .. })));
}
```

Run: `cargo test --test approval_integration` — tests will fail (module visibility or missing public `check_approval`).

---

## GREEN — Implement

### 1. Confirm public API in `src/lib.rs`

The `pub use` line must export `check_approval` and `ApprovalError` but NOT `ApprovalTimeoutError` (removed in `error-types` task):

```rust
pub use approval::{check_approval, ApprovalError};
```

### 2. Make `check_approval` pub in `src/approval.rs`

Already declared `pub` in the existing stub. Confirm the signature matches:

```rust
pub async fn check_approval(
    module_def: &serde_json::Value,
    auto_approve: bool,
) -> Result<(), ApprovalError>
```

### 3. Wire into `main.rs` / CLI dispatcher

Locate where module execution is triggered (likely in `src/cli.rs` exec callback or `src/main.rs`). Add the approval check before calling execute:

```rust
// In the exec callback, after collecting module_def and auto_approve flag:
if let Err(e) = check_approval(&module_def, auto_approve).await {
    // All ApprovalError variants → exit 46
    eprintln!("{}", e);  // thiserror Display message
    std::process::exit(EXIT_APPROVAL_DENIED);
}
```

The exact insertion point depends on `src/cli.rs` structure. Find the `build_module_command` exec closure and add the `check_approval` call at the top of module execution, before the actual module invoke.

### 4. Update `src/lib.rs` re-export

Remove `ApprovalTimeoutError` from the `pub use` line. Final line:

```rust
pub use approval::{check_approval, ApprovalError};
```

Run: `cargo test --test approval_integration` — all integration tests must pass.
Run: `cargo test --lib approval` — all unit tests must still pass.

---

## REFACTOR

- Confirm `eprintln!("{}", e)` uses the `thiserror` `Display` impl, not `Debug`.
- Confirm `EXIT_APPROVAL_DENIED` is used (not the literal `46`) for the `std::process::exit` call.
- Run `cargo clippy -- -D warnings` — zero warnings.

---

## Verification

```
cargo test --lib approval
cargo test --test approval_integration
cargo build
cargo clippy -- -D warnings
```

Full acceptance criteria checklist (from plan.md) must all be green.

---

## Files Modified

- `src/approval.rs` — confirm public signature
- `src/lib.rs` — update `pub use approval::{...}` (remove `ApprovalTimeoutError`)
- `src/main.rs` or `src/cli.rs` — wire `check_approval` into module exec path
- `tests/approval_integration.rs` — new integration test file
