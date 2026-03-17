# Task: annotation-extraction

**ID**: annotation-extraction
**Status**: pending
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~45 min
**Depends on**: error-types

---

## Objective

Implement two private helper functions that extract data from the `module_def` JSON value:

1. `get_requires_approval(module_def: &serde_json::Value) -> bool` — returns `true` only when `module_def["annotations"]["requires_approval"]` is exactly `Value::Bool(true)`. All other values (string `"true"`, integer `1`, `null`, absent key) return `false`.
2. `get_approval_message(module_def: &serde_json::Value, module_id: &str) -> String` — returns `module_def["annotations"]["approval_message"]` if it is a non-empty string, otherwise the default: `"Module '{module_id}' requires approval to execute."`.
3. `get_module_id(module_def: &serde_json::Value) -> String` — returns `module_def["module_id"]` or `module_def["canonical_id"]` if either is a string, otherwise `"unknown"`.

These helpers encode the strict-boolean rule from FR-03-01 and the Python lesson: `"true"` string is NOT truthy.

---

## RED — Write failing tests first

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // get_requires_approval

    #[test]
    fn requires_approval_true_returns_true() {
        let v = json!({"annotations": {"requires_approval": true}});
        assert!(get_requires_approval(&v));
    }

    #[test]
    fn requires_approval_false_returns_false() {
        let v = json!({"annotations": {"requires_approval": false}});
        assert!(!get_requires_approval(&v));
    }

    #[test]
    fn requires_approval_string_true_returns_false() {
        // Key decision: string "true" is NOT truthy — strict bool check only.
        let v = json!({"annotations": {"requires_approval": "true"}});
        assert!(!get_requires_approval(&v));
    }

    #[test]
    fn requires_approval_int_one_returns_false() {
        let v = json!({"annotations": {"requires_approval": 1}});
        assert!(!get_requires_approval(&v));
    }

    #[test]
    fn requires_approval_null_returns_false() {
        let v = json!({"annotations": {"requires_approval": null}});
        assert!(!get_requires_approval(&v));
    }

    #[test]
    fn requires_approval_absent_returns_false() {
        let v = json!({"annotations": {}});
        assert!(!get_requires_approval(&v));
    }

    #[test]
    fn requires_approval_no_annotations_returns_false() {
        let v = json!({});
        assert!(!get_requires_approval(&v));
    }

    #[test]
    fn requires_approval_annotations_null_returns_false() {
        let v = json!({"annotations": null});
        assert!(!get_requires_approval(&v));
    }

    // get_approval_message

    #[test]
    fn approval_message_custom() {
        let v = json!({"annotations": {"approval_message": "Please confirm."}});
        assert_eq!(get_approval_message(&v, "mod-x"), "Please confirm.");
    }

    #[test]
    fn approval_message_default_when_absent() {
        let v = json!({"annotations": {}});
        assert_eq!(
            get_approval_message(&v, "mod-x"),
            "Module 'mod-x' requires approval to execute."
        );
    }

    #[test]
    fn approval_message_default_when_not_string() {
        let v = json!({"annotations": {"approval_message": 42}});
        assert_eq!(
            get_approval_message(&v, "mod-x"),
            "Module 'mod-x' requires approval to execute."
        );
    }

    // get_module_id

    #[test]
    fn module_id_from_module_id_field() {
        let v = json!({"module_id": "my-module"});
        assert_eq!(get_module_id(&v), "my-module");
    }

    #[test]
    fn module_id_from_canonical_id_field() {
        let v = json!({"canonical_id": "canon-module"});
        assert_eq!(get_module_id(&v), "canon-module");
    }

    #[test]
    fn module_id_unknown_when_absent() {
        let v = json!({});
        assert_eq!(get_module_id(&v), "unknown");
    }
}
```

Run: `cargo test --lib approval::tests` — tests will fail to compile (functions don't exist yet).

---

## GREEN — Implement

Add to `src/approval.rs` (private helpers, before `check_approval`):

```rust
fn get_requires_approval(module_def: &serde_json::Value) -> bool {
    module_def
        .get("annotations")
        .and_then(|a| a.get("requires_approval"))
        .and_then(|v| v.as_bool())
        == Some(true)
}

fn get_approval_message(module_def: &serde_json::Value, module_id: &str) -> String {
    module_def
        .get("annotations")
        .and_then(|a| a.get("approval_message"))
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("Module '{module_id}' requires approval to execute."))
}

fn get_module_id(module_def: &serde_json::Value) -> String {
    module_def
        .get("module_id")
        .or_else(|| module_def.get("canonical_id"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}
```

Run: `cargo test --lib approval::tests` — all tests must pass.

---

## REFACTOR

- Confirm that `as_bool()` on a `Value::Bool(true)` returns `Some(true)` and on `Value::String("true")` returns `None` (this is correct serde_json behavior — the test suite validates it).
- No further refactoring expected.

---

## Verification

```
cargo test --lib approval::tests
cargo clippy -- -D warnings
```

---

## Files Modified

- `src/approval.rs` — add three private helper functions
