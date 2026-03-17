# Task: format-exec-result

**Feature**: FE-08 Output Formatter
**File**: `src/output.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~1.5h
**Depends on**: `resolve-format-and-truncate`
**Required by**: `wire-format-flag`

---

## Context

`format_exec_result` formats the output of a module execution. The Python version accepts `Any` and handles dict, list, str, None, and other types using `isinstance` checks and `json.dumps(result, default=str)`. The Rust version works on `serde_json::Value` directly, using a `match` on the enum variant.

**Python → Rust variant mapping:**

| Python type | `serde_json::Value` variant | Behaviour |
|-------------|----------------------------|-----------|
| `dict` | `Value::Object` | JSON (both modes), or key-value `comfy-table` in `"table"` mode |
| `list` | `Value::Array` | `serde_json::to_string_pretty` always |
| `str` | `Value::String` | Print raw string value (no JSON quotes) |
| `None` | `Value::Null` | Return empty string (caller prints nothing) |
| other (int, float, bool) | `Value::Number`, `Value::Bool` | `.to_string()` |

**Difference from Python**: The Python `format_exec_result` calls `resolve_format(format)` internally to handle `format=None`. The Rust version accepts an already-resolved `&str` format. Callers must call `resolve_format` first if they have an `Option<&str>`. This avoids lifetime complications and keeps the function pure.

The existing stub signature `format_exec_result(result: &Value, format: &str) -> String` is correct and does not change.

---

## RED — Write Failing Tests First

Add to the `#[cfg(test)]` block in `src/output.rs`:

```rust
    // --- format_exec_result ---

    #[test]
    fn test_format_exec_result_null_returns_empty() {
        let output = format_exec_result(&Value::Null, "json");
        assert_eq!(output, "", "Null result must produce empty string");
    }

    #[test]
    fn test_format_exec_result_string_plain() {
        let result = json!("hello world");
        let output = format_exec_result(&result, "json");
        assert_eq!(output, "hello world");
    }

    #[test]
    fn test_format_exec_result_string_table_mode_also_plain() {
        // Strings are always printed raw, regardless of format.
        let result = json!("hello");
        let output = format_exec_result(&result, "table");
        assert_eq!(output, "hello");
    }

    #[test]
    fn test_format_exec_result_object_json_mode() {
        let result = json!({"sum": 42, "status": "ok"});
        let output = format_exec_result(&result, "json");
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("must be valid JSON");
        assert_eq!(parsed["sum"], 42);
        assert_eq!(parsed["status"], "ok");
    }

    #[test]
    fn test_format_exec_result_object_table_mode() {
        let result = json!({"key": "value", "count": 3});
        let output = format_exec_result(&result, "table");
        // Table must contain both keys and their values.
        assert!(output.contains("key"), "table must contain 'key'");
        assert!(output.contains("value"), "table must contain 'value'");
        assert!(output.contains("count"), "table must contain 'count'");
        assert!(output.contains('3'), "table must contain '3'");
    }

    #[test]
    fn test_format_exec_result_array_is_json() {
        let result = json!([1, 2, 3]);
        let output = format_exec_result(&result, "json");
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("must be valid JSON");
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_format_exec_result_array_table_mode_is_json() {
        // Arrays always render as JSON, even in table mode.
        let result = json!([{"a": 1}, {"b": 2}]);
        let output = format_exec_result(&result, "table");
        let parsed: serde_json::Value = serde_json::from_str(&output).expect("array must produce JSON");
        assert!(parsed.is_array());
    }

    #[test]
    fn test_format_exec_result_number_scalar() {
        let result = json!(42);
        let output = format_exec_result(&result, "json");
        assert_eq!(output, "42");
    }

    #[test]
    fn test_format_exec_result_bool_scalar() {
        let result = json!(true);
        let output = format_exec_result(&result, "json");
        assert_eq!(output, "true");
    }

    #[test]
    fn test_format_exec_result_float_scalar() {
        let result = json!(3.14);
        let output = format_exec_result(&result, "json");
        assert!(output.starts_with("3.14"), "float must stringify correctly");
    }
```

Run `cargo test test_format_exec_result` — all fail (stub `todo!` panics).

---

## GREEN — Implement

Replace the `format_exec_result` stub in `src/output.rs`:

```rust
pub fn format_exec_result(result: &Value, format: &str) -> String {
    match result {
        Value::Null => String::new(),

        Value::String(s) => s.clone(),

        Value::Object(_) if format == "table" => {
            let obj = result.as_object().unwrap(); // safe: matched Object above
            let mut table = Table::new();
            table.set_content_arrangement(ContentArrangement::Dynamic);
            table.set_header(vec!["Key", "Value"]);
            for (k, v) in obj {
                let val_str = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                table.add_row(vec![k.clone(), val_str]);
            }
            table.to_string()
        }

        Value::Object(_) | Value::Array(_) => {
            serde_json::to_string_pretty(result).unwrap_or_else(|_| "null".to_string())
        }

        // Number, Bool — convert to display string.
        other => other.to_string(),
    }
}
```

**Notes:**
- `Value::Object(_) if format == "table"` uses a match guard to separate table vs. JSON rendering for objects. The `Value::Object(_) | Value::Array(_)` arm below handles the JSON case for objects and all array cases (including `table` mode for arrays, which always falls through to JSON output per the Python spec).
- `serde_json::Value::to_string()` for `Number` produces the numeric string (e.g., `"42"`, `"3.14"`). For `Bool` it produces `"true"` or `"false"`. This matches Python's `str(result)` behaviour.
- There is no `default=str` fallback needed: `serde_json::Value` is always serializable.

---

## REFACTOR

- Confirm the match arm ordering: the `Value::Object(_) if format == "table"` guard arm must appear before the `Value::Object(_) | Value::Array(_)` arm, otherwise the compiler will report an unreachable pattern. Rust match arms are evaluated in order; the guard arm is checked first.
- Consider whether `Value::Number` should use a custom formatter for large floats (e.g., `1e100` vs `1.0`). For now, `to_string()` is acceptable.
- Run `cargo clippy -- -D warnings`.
- Remove the four `assert!(false, "not implemented")` stubs from the inline `#[cfg(test)]` block in `src/output.rs` for `format_module_list_json`, `format_module_list_table`, `format_module_detail_json`, `format_exec_result_json`, `format_exec_result_table`.

---

## Verification

```bash
cargo test test_format_exec_result 2>&1
# Expected: 10 tests pass, 0 fail.
```
