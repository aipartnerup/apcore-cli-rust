# Task: validate-module-id

**Feature**: FE-01 Core Dispatcher
**File**: `src/cli.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~2h
**Depends on**: nothing
**Required by**: `collect-input`, `lazy-module-group-skeleton`

---

## Context

`validate_module_id` is the first guard called before any registry lookup. It must reject bad module IDs with exit code 2 and an exact error message. The function signature is already stubbed in `src/cli.rs` and skeleton tests exist in both `src/cli.rs` (unit) and `tests/test_cli.rs` (integration).

The existing stub uses a loose pattern `^[a-z_][a-z0-9_.]*$` that is too permissive (allows consecutive dots, trailing dots, leading dot). The correct pattern from the spec is:

```
^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$
```

This pattern must be enforced without adding the `regex` crate. A hand-written validator is preferred to keep the dependency surface small.

---

## RED — Write Failing Tests First

The tests already exist as stubs in `tests/test_cli.rs`. Remove the `assert!(false, "not implemented")` placeholders and fill in proper assertions. The unit tests in `src/cli.rs` are already complete and should also pass.

**`tests/test_cli.rs`** — update:

```rust
#[test]
fn test_validate_module_id_valid_ids() {
    for id in ["math.add", "text.summarize", "a", "a.b.c", "my_mod.sub"] {
        assert!(validate_module_id(id).is_ok(), "expected ok for '{id}'");
    }
}

#[test]
fn test_validate_module_id_too_long() {
    let long_id = "a".repeat(129);
    assert!(validate_module_id(&long_id).is_err());
}

#[test]
fn test_validate_module_id_invalid_formats() {
    for id in [
        "INVALID!ID",   // uppercase + special char
        "123abc",       // starts with digit
        ".leading.dot", // leading dot
        "a..b",         // consecutive dots
        "a.",           // trailing dot
        "a.B",          // uppercase segment
        "a b",          // space
        "",             // empty string
    ] {
        assert!(
            validate_module_id(id).is_err(),
            "expected error for '{id}'"
        );
    }
}

#[test]
fn test_validate_module_id_max_length_ok() {
    // Exactly 128 chars: "a" repeated 128 times
    let max_id = "a".repeat(128);
    assert!(validate_module_id(&max_id).is_ok());
}

#[test]
fn test_validate_module_id_error_message_contains_id() {
    let bad_id = "BAD_ID!";
    let err = validate_module_id(bad_id).unwrap_err();
    assert!(
        err.to_string().contains(bad_id),
        "error message must contain the invalid id"
    );
}
```

Run `cargo test validate_module_id` — all tests should fail (function panics with `todo!`).

---

## GREEN — Implement

Replace the `todo!` in `src/cli.rs`'s `validate_module_id` and fix the `MODULE_ID_PATTERN` constant.

```rust
// Correct pattern constant — update the existing constant:
const MODULE_ID_MAX_LEN: usize = 128;

/// Validate a module identifier.
///
/// Rules:
/// - Maximum 128 characters
/// - Matches `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$`
/// - No leading/trailing dots, no consecutive dots
///
/// # Errors
/// Returns `CliError::InvalidModuleId` (exit code 2) on any violation.
pub fn validate_module_id(module_id: &str) -> Result<(), CliError> {
    if module_id.len() > MODULE_ID_MAX_LEN {
        return Err(CliError::InvalidModuleId(format!(
            "Invalid module ID format: '{module_id}'. Maximum length is 128 characters."
        )));
    }
    if !is_valid_module_id(module_id) {
        return Err(CliError::InvalidModuleId(format!(
            "Invalid module ID format: '{module_id}'."
        )));
    }
    Ok(())
}

/// Hand-written validator matching `^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)*$`.
///
/// Does not require the `regex` crate.
fn is_valid_module_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Split on '.' and validate each segment.
    let segments: Vec<&str> = s.split('.').collect();
    for segment in &segments {
        if segment.is_empty() {
            return false; // leading dot, trailing dot, or consecutive dots
        }
        let mut chars = segment.chars();
        // First char must be a lowercase letter.
        match chars.next() {
            Some(c) if c.is_ascii_lowercase() => {}
            _ => return false,
        }
        // Remaining chars: lowercase letter, digit, or underscore.
        for c in chars {
            if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '_' {
                return false;
            }
        }
    }
    true
}
```

Run `cargo test validate_module_id` — all tests should pass.

---

## REFACTOR

- Delete the now-unused `MODULE_ID_PATTERN` `&str` constant from `cli.rs` (it was never compiled into a `Regex`).
- Ensure `is_valid_module_id` is `#[inline]` if the validator is in the hot path (optional, cosmetic).
- Confirm `cargo clippy -- -D warnings` is clean.

---

## Verification

```bash
cargo test validate_module_id 2>&1
# Expected: test result: ok. N passed; 0 failed
```

Exit-code behaviour is exercised in `tests/test_e2e.rs` (T-DISP-04). The unit tests here confirm the `Result` contract; the e2e test confirms the `process::exit(2)` path in the dispatcher.
