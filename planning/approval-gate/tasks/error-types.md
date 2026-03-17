# Task: error-types

**ID**: error-types
**Status**: pending
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~30 min
**Depends on**: (none)

---

## Objective

Replace the existing `ApprovalError` skeleton in `src/approval.rs` with a complete, production-ready `thiserror`-derived enum. All three error variants (`Denied`, `NonInteractive`, `Timeout`) must carry the `module_id` field and produce correctly formatted `Display` messages. The `ApprovalTimeoutError` type alias must be removed — it was a placeholder; callers use `ApprovalError` directly.

---

## RED — Write failing tests first

Add to `src/approval.rs` inside `#[cfg(test)] mod tests`:

```rust
#[test]
fn error_denied_display() {
    let e = ApprovalError::Denied { module_id: "my-module".into() };
    assert_eq!(e.to_string(), "approval denied for module 'my-module'");
}

#[test]
fn error_non_interactive_display() {
    let e = ApprovalError::NonInteractive { module_id: "my-module".into() };
    assert_eq!(
        e.to_string(),
        "no interactive terminal available for module 'my-module'"
    );
}

#[test]
fn error_timeout_display() {
    let e = ApprovalError::Timeout { module_id: "my-module".into(), seconds: 60 };
    assert_eq!(
        e.to_string(),
        "approval timed out after 60s for module 'my-module'"
    );
}

#[test]
fn error_variants_are_debug() {
    // Ensures the derive(Debug) is present and compiles.
    let d = format!("{:?}", ApprovalError::Denied { module_id: "x".into() });
    assert!(d.contains("Denied"));
}
```

Run: `cargo test --lib approval::tests` — expect compilation failure or test failure.

---

## GREEN — Implement

Replace the `ApprovalError` enum and remove `ApprovalTimeoutError` alias in `src/approval.rs`:

```rust
#[derive(Debug, Error)]
pub enum ApprovalError {
    #[error("approval denied for module '{module_id}'")]
    Denied { module_id: String },

    #[error("no interactive terminal available for module '{module_id}'")]
    NonInteractive { module_id: String },

    #[error("approval timed out after {seconds}s for module '{module_id}'")]
    Timeout { module_id: String, seconds: u64 },
}
```

Update `src/lib.rs` re-export: remove `ApprovalTimeoutError` from the `pub use` line (it no longer exists).

Run: `cargo test --lib approval::tests` — all four tests must pass.

---

## REFACTOR

- Verify error messages exactly match the strings expected by the acceptance criteria (plan.md).
- Confirm `cargo clippy -- -D warnings` produces no warnings for this module.

---

## Verification

```
cargo test --lib approval::tests::error_denied_display
cargo test --lib approval::tests::error_non_interactive_display
cargo test --lib approval::tests::error_timeout_display
cargo test --lib approval::tests::error_variants_are_debug
cargo build 2>&1 | grep -c "^error" || true  # must be 0
```

---

## Files Modified

- `src/approval.rs` — replace `ApprovalError`, remove `ApprovalTimeoutError` alias
- `src/lib.rs` — update `pub use approval::{...}` to remove `ApprovalTimeoutError`
