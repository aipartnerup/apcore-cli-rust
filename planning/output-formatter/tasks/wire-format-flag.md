# Task: wire-format-flag

**Feature**: FE-08 Output Formatter
**File**: `src/discovery.rs`, `tests/test_output.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~1.5h
**Depends on**: `format-module-list`, `format-module-detail`, `format-exec-result`
**Required by**: (none — final integration task)

---

## Context

This task wires the completed output formatter functions into the `discovery.rs` command handlers and replaces all `assert!(false, "not implemented")` stubs in `tests/test_output.rs` with fully passing assertions.

Currently, `discovery.rs` contains only stub `todo!()` implementations. This task is **not** responsible for fully implementing the discovery command dispatch (that belongs to the `core-dispatcher` feature). Its scope is:

1. Replace the stubbed inline unit tests in `src/output.rs` that still say `assert!(false, "not implemented")`.
2. Replace all `assert!(false, "not implemented")` tests in `tests/test_output.rs` with working assertions using the now-implemented output functions.
3. Confirm that the `lib.rs` re-export of `format_module_list` has the correct updated signature (with `filter_tags: &[&str]`).

This task does **not** need to implement `register_discovery_commands`, `list_command`, or `describe_command` in `discovery.rs` — those are discovery feature scope. It only ensures the output integration tests are green.

---

## RED — Write Failing Tests First (tests/test_output.rs)

The file `tests/test_output.rs` already contains stubs with `assert!(false, "not implemented")`. Replace all of them with working assertions. The RED step is confirming that the stubs currently fail.

Run `cargo test --test test_output` to confirm all existing stubs in that file still fail.

---

## GREEN — Update tests/test_output.rs

Replace the contents of `tests/test_output.rs` with fully implemented tests:

```rust
// apcore-cli — Integration tests for output formatting.
// Protocol spec: FE-08

mod common;

use apcore_cli::output::{format_exec_result, format_module_detail, format_module_list, resolve_format};
use serde_json::json;

// ---------------------------------------------------------------------------
// resolve_format
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_format_explicit_json() {
    assert_eq!(resolve_format(Some("json")), "json");
}

#[test]
fn test_resolve_format_explicit_table() {
    assert_eq!(resolve_format(Some("table")), "table");
}

#[test]
fn test_resolve_format_none_defaults_to_json_in_ci() {
    // In a test runner, stdout is not a TTY, so None → "json".
    // If this assertion fails, the test environment has a TTY attached —
    // which is unusual for CI. Both outcomes are valid; this just documents
    // the expected CI behaviour.
    let fmt = resolve_format(None);
    assert!(
        fmt == "json" || fmt == "table",
        "resolve_format(None) must return 'json' or 'table', got '{}'",
        fmt
    );
}

// ---------------------------------------------------------------------------
// format_module_list
// ---------------------------------------------------------------------------

#[test]
fn test_format_module_list_json_valid() {
    let modules = vec![
        json!({"module_id": "math.add", "description": "Add two numbers", "tags": []}),
        json!({"module_id": "text.upper", "description": "Uppercase", "tags": []}),
    ];
    let output = format_module_list(&modules, "json", &[]);
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("must be valid JSON");
    let arr = parsed.as_array().expect("must be JSON array");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["id"], "math.add");
    assert_eq!(arr[1]["id"], "text.upper");
}

#[test]
fn test_format_module_list_table_has_headers() {
    let modules = vec![
        json!({"module_id": "math.add", "description": "Add two numbers", "tags": []}),
    ];
    let output = format_module_list(&modules, "table", &[]);
    assert!(output.contains("ID"), "table must have ID column header");
    assert!(output.contains("Description"), "table must have Description column header");
}

#[test]
fn test_format_module_list_table_contains_module_id() {
    let modules = vec![
        json!({"module_id": "math.add", "description": "Add two numbers", "tags": []}),
    ];
    let output = format_module_list(&modules, "table", &[]);
    assert!(output.contains("math.add"));
}

#[test]
fn test_format_module_list_table_empty_no_tags() {
    let output = format_module_list(&[], "table", &[]);
    assert_eq!(output.trim(), "No modules found.");
}

#[test]
fn test_format_module_list_table_empty_with_filter_tags() {
    let output = format_module_list(&[], "table", &["math"]);
    assert!(output.contains("No modules found matching tags:"));
    assert!(output.contains("math"));
}

#[test]
fn test_format_module_list_json_empty() {
    let output = format_module_list(&[], "json", &[]);
    assert_eq!(output.trim(), "[]");
}

// ---------------------------------------------------------------------------
// format_module_detail
// ---------------------------------------------------------------------------

#[test]
fn test_format_module_detail_json() {
    let module = json!({
        "module_id": "math.add",
        "description": "Add two numbers",
        "input_schema": {"type": "object"},
        "tags": ["math"]
    });
    let output = format_module_detail(&module, "json");
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("must be valid JSON");
    assert_eq!(parsed["id"], "math.add");
    assert_eq!(parsed["description"], "Add two numbers");
    assert!(parsed.get("input_schema").is_some());
}

#[test]
fn test_format_module_detail_json_no_null_fields() {
    let module = json!({
        "module_id": "a.b",
        "description": "desc",
    });
    let output = format_module_detail(&module, "json");
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("input_schema").is_none());
    assert!(parsed.get("output_schema").is_none());
    assert!(parsed.get("tags").is_none());
}

#[test]
fn test_format_module_detail_table_description() {
    let module = json!({
        "module_id": "math.add",
        "description": "Add two numbers",
    });
    let output = format_module_detail(&module, "table");
    assert!(output.contains("Add two numbers"));
    assert!(output.contains("math.add"));
}

// ---------------------------------------------------------------------------
// format_exec_result
// ---------------------------------------------------------------------------

#[test]
fn test_format_exec_result_json() {
    let result = json!({"sum": 42});
    let output = format_exec_result(&result, "json");
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("must be valid JSON");
    assert_eq!(parsed["sum"], 42);
}

#[test]
fn test_format_exec_result_table() {
    let result = json!({"sum": 42});
    let output = format_exec_result(&result, "table");
    assert!(output.contains("sum"), "table must contain key 'sum'");
    assert!(output.contains("42"), "table must contain value '42'");
}

#[test]
fn test_format_exec_result_null() {
    let output = format_exec_result(&serde_json::Value::Null, "json");
    assert_eq!(output, "");
}

#[test]
fn test_format_exec_result_string() {
    let result = json!("hello");
    let output = format_exec_result(&result, "json");
    assert_eq!(output, "hello");
}

#[test]
fn test_format_exec_result_array() {
    let result = json!([1, 2, 3]);
    let output = format_exec_result(&result, "json");
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("must be valid JSON");
    assert!(parsed.is_array());
}
```

---

## GREEN — Update lib.rs export (if needed)

If `format_module_list` signature changed (added `filter_tags` parameter), verify the `lib.rs` re-export still compiles:

```rust
// In src/lib.rs — this line should already be present:
pub use output::{format_exec_result, format_module_detail, format_module_list, resolve_format};
```

No change to the re-export line is needed since it re-exports the function by name. Call sites that call `format_module_list` with two arguments will need to be updated to pass `&[]` as the third argument.

---

## GREEN — Remove remaining stubs in src/output.rs inline tests

The inline `#[cfg(test)]` block in `src/output.rs` still contains five stubs from the original scaffolding that say `assert!(false, "not implemented")`:

- `test_resolve_format_explicit_json` — replaced by `test_resolve_format_explicit_json_tty` in `resolve-format-and-truncate`
- `test_resolve_format_explicit_table` — replaced by `test_resolve_format_explicit_table_non_tty`
- `test_format_module_list_json` — replaced by full tests in `format-module-list`
- `test_format_module_list_table` — replaced by full tests in `format-module-list`
- `test_format_module_detail_json` — replaced by full tests in `format-module-detail`
- `test_format_exec_result_json` — replaced by full tests in `format-exec-result`
- `test_format_exec_result_table` — replaced by full tests in `format-exec-result`

Delete these stub functions from the inline test block. They are superseded by the more detailed tests added in the earlier tasks.

---

## REFACTOR

- Run the full test suite: `cargo test 2>&1` and confirm zero failures in the output module.
- Run `cargo clippy -- -D warnings` on the full crate.
- Confirm `tests/test_output.rs` has zero `assert!(false, ...)` calls: `grep -n "assert!(false" tests/test_output.rs` should return nothing.

---

## Verification

```bash
cargo test --test test_output 2>&1
# Expected: all tests pass, 0 fail.

cargo test 2>&1 | grep -E "^test result"
# Expected: test result: ok. N passed; 0 failed.
```
