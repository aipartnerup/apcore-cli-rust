# Task: describe-command

**Feature**: FE-04 Discovery
**File**: `src/discovery.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~2h
**Depends on**: `tag-validation`
**Required by**: `register-discovery-commands`

---

## Context

`cmd_describe` implements the `describe` subcommand handler. It validates the module ID format, looks up the module definition, and renders it via `output::format_module_detail`.

Key behaviours (ported from Python `describe_cmd`):

1. `validate_module_id(id)` is called first — invalid format → `DiscoveryError::InvalidModuleId` (caller exits 2).
2. `registry.get_definition(id)` returns `None` → `DiscoveryError::ModuleNotFound` (caller exits 44).
3. Format resolution is delegated to `output::resolve_format(explicit_format)`.
4. Rendering is delegated to `output::format_module_detail(&module, fmt)`.
5. Optional sections (input schema, output schema, annotations, x-fields) are absent from table output when missing — this is handled entirely by `format_module_detail`, not `cmd_describe`.
6. JSON output omits keys with `null` values — also handled by `format_module_detail`.

The `describe_command()` builder function adds `MODULE_ID` as a required positional argument and `--format` with `PossibleValuesParser`. Invalid `--format` values are rejected by clap at parse time (exit 2). The dispatch from the clap match arm to `cmd_describe` is implemented in the `register-discovery-commands` task.

---

## RED — Write Failing Tests First

Add to the `#[cfg(test)]` block in `src/discovery.rs`:

```rust
    // --- cmd_describe ---

    #[test]
    fn test_cmd_describe_valid_module_json() {
        let registry = MockRegistry::new(vec![
            mock_module("math.add", "Add two numbers", &["math", "core"]),
        ]);
        let output = cmd_describe(&registry, "math.add", Some("json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(parsed["id"], "math.add");
        assert_eq!(parsed["description"], "Add two numbers");
    }

    #[test]
    fn test_cmd_describe_valid_module_table() {
        let registry = MockRegistry::new(vec![
            mock_module("math.add", "Add two numbers", &["math"]),
        ]);
        let output = cmd_describe(&registry, "math.add", Some("table")).unwrap();
        assert!(output.contains("math.add"), "table must contain module id");
        assert!(output.contains("Add two numbers"), "table must contain description");
    }

    #[test]
    fn test_cmd_describe_not_found_returns_error() {
        let registry = MockRegistry::new(vec![]);
        let result = cmd_describe(&registry, "non.existent", Some("json"));
        assert!(result.is_err());
        match result.unwrap_err() {
            DiscoveryError::ModuleNotFound(id) => assert_eq!(id, "non.existent"),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn test_cmd_describe_invalid_id_returns_error() {
        let registry = MockRegistry::new(vec![]);
        let result = cmd_describe(&registry, "INVALID!ID", Some("json"));
        assert!(result.is_err());
        match result.unwrap_err() {
            DiscoveryError::InvalidModuleId(_) => {}
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn test_cmd_describe_no_output_schema_table_omits_section() {
        // Module without output_schema: section must be absent from table output.
        let registry = MockRegistry::new(vec![
            serde_json::json!({
                "module_id": "math.add",
                "description": "Add numbers",
                "input_schema": {"type": "object"},
                "tags": ["math"]
                // note: no output_schema key
            }),
        ]);
        let output = cmd_describe(&registry, "math.add", Some("table")).unwrap();
        assert!(!output.contains("Output Schema:"), "output_schema section must be absent");
    }

    #[test]
    fn test_cmd_describe_no_annotations_table_omits_section() {
        let registry = MockRegistry::new(vec![
            mock_module("math.add", "Add numbers", &["math"]),
        ]);
        let output = cmd_describe(&registry, "math.add", Some("table")).unwrap();
        assert!(!output.contains("Annotations:"), "annotations section must be absent");
    }

    #[test]
    fn test_cmd_describe_with_annotations_table_shows_section() {
        let registry = MockRegistry::new(vec![
            serde_json::json!({
                "module_id": "math.add",
                "description": "Add numbers",
                "annotations": {"readonly": true},
                "tags": []
            }),
        ]);
        let output = cmd_describe(&registry, "math.add", Some("table")).unwrap();
        assert!(output.contains("Annotations:"), "annotations section must be present");
        assert!(output.contains("readonly"), "annotation key must appear");
    }

    #[test]
    fn test_cmd_describe_json_omits_null_fields() {
        // Module with no input_schema, output_schema, annotations.
        let registry = MockRegistry::new(vec![
            mock_module("a.b", "Desc", &[]),
        ]);
        let output = cmd_describe(&registry, "a.b", Some("json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.get("input_schema").is_none());
        assert!(parsed.get("output_schema").is_none());
        assert!(parsed.get("annotations").is_none());
    }

    #[test]
    fn test_cmd_describe_json_includes_all_fields() {
        let registry = MockRegistry::new(vec![
            serde_json::json!({
                "module_id": "math.add",
                "description": "Add two numbers",
                "input_schema": {"type": "object", "properties": {"a": {"type": "integer"}}},
                "output_schema": {"type": "object", "properties": {"result": {"type": "integer"}}},
                "annotations": {"readonly": false},
                "tags": ["math", "core"]
            }),
        ]);
        let output = cmd_describe(&registry, "math.add", Some("json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.get("input_schema").is_some());
        assert!(parsed.get("output_schema").is_some());
        assert!(parsed.get("annotations").is_some());
        assert!(parsed.get("tags").is_some());
    }

    #[test]
    fn test_cmd_describe_with_x_fields_table_shows_extension_section() {
        let registry = MockRegistry::new(vec![
            serde_json::json!({
                "module_id": "a.b",
                "description": "Desc",
                "x-custom": "custom-value",
                "tags": []
            }),
        ]);
        let output = cmd_describe(&registry, "a.b", Some("table")).unwrap();
        assert!(
            output.contains("Extension Metadata:") || output.contains("x-custom"),
            "x-fields must appear in table output"
        );
    }
```

Run `cargo test test_cmd_describe` — all fail (`cmd_describe` is not yet implemented).

---

## GREEN — Implement

The `cmd_describe` function and `describe_command` builder are introduced in the `tag-validation` task's GREEN step as part of the full `discovery.rs` rewrite. This task's GREEN step verifies the implementation covers all tests above.

Expected adjustments from running these tests:

1. **Module ID validation**: `crate::cli::validate_module_id` is a `todo!()` stub. For the discovery feature, `cmd_describe` should implement its own ID validation using the same character rules, or the `validate-module-id` task from `core-dispatcher` must be completed first. **Decision**: Add a dependency note — if `validate-module-id` is not yet implemented, replace the delegation with an inline check using `validate_module_id_discovery` that mirrors the core-dispatcher logic:

   ```rust
   fn validate_module_id_discovery(id: &str) -> bool {
       if id.is_empty() || id.len() > 128 { return false; }
       if id.starts_with('.') || id.ends_with('.') || id.contains("..") { return false; }
       let mut chars = id.chars();
       match chars.next() {
           Some(c) if c.is_ascii_lowercase() || c == '_' => {}
           _ => return false,
       }
       chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
   }
   ```

   If `cli::validate_module_id` becomes available before this task runs, use it instead.

2. **`format_module_detail` contract**: The output-formatter plan specifies that `format_module_detail` in table mode prints `"Output Schema:"` only when `output_schema` is present, and `"Annotations:"` only when `annotations` is present and non-empty. Verify these tests pass once `format-module-detail` task is complete.

3. **JSON null field omission**: `format_module_detail` in JSON mode must omit keys with `null` or absent values. The `test_cmd_describe_json_omits_null_fields` test passes `mock_module` which has no `input_schema`, `output_schema`, or `annotations` keys — so they should be absent from the JSON output.

---

## REFACTOR

- Ensure `describe_command` uses `.value_name("MODULE_ID")` on the positional arg for clean `--help` output.
- Confirm that `DiscoveryError::InvalidModuleId` vs `DiscoveryError::ModuleNotFound` error messages match the Python spec:
  - Invalid ID: no specific message from `cmd_describe` (clap rejects `--format` invalid values; ID format check uses `DiscoveryError::InvalidModuleId`).
  - Not found: `"Error: Module '{id}' not found."` — the caller prints this from the error message.
- Run `cargo clippy -- -D warnings` on `src/discovery.rs`.

---

## Verification

```bash
cargo test test_cmd_describe 2>&1
# Expected: 9 tests pass, 0 fail.

cargo test --test test_discovery 2>&1
# Expected: describe-related tests pass (register tests may still fail).
```
