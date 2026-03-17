# Task: list-command

**Feature**: FE-04 Discovery
**File**: `src/discovery.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~2h
**Depends on**: `tag-validation`
**Required by**: `register-discovery-commands`

---

## Context

`cmd_list` implements the `list` subcommand handler. It takes a registry reference, a slice of filter tags, and an optional explicit format string, then returns a formatted `String` (or `DiscoveryError` on invalid tag format).

Key behaviours (ported from Python `list_cmd`):

1. Each tag is validated against `^[a-z][a-z0-9_-]*$` — invalid format → `DiscoveryError::InvalidTag` (caller exits 2).
2. All module definitions are fetched via `registry.list()` + `registry.get_definition`.
3. Tag filtering applies AND semantics: every specified tag must appear in the module's `tags` array.
4. Format resolution is delegated to `output::resolve_format(explicit_format)`.
5. Rendering is delegated to `output::format_module_list(&modules, fmt, tags)`.
6. Empty results with tags specified → `"No modules found matching tags: ..."`.
7. Empty results with no tags → `"No modules found."`.
8. Both empty-result cases exit 0 (empty-result messages come from `format_module_list`, not `cmd_list`).

The `list_command()` function builds the clap `Command` with `--tag` (`ArgAction::Append`) and `--format` (`PossibleValuesParser`). The dispatch from the clap match arm to `cmd_list` is implemented in the `register-discovery-commands` task.

---

## RED — Write Failing Tests First

Add to the `#[cfg(test)]` block in `src/discovery.rs`:

```rust
    // --- cmd_list ---

    #[test]
    fn test_cmd_list_all_modules_no_filter() {
        let registry = MockRegistry::new(vec![
            mock_module("math.add", "Add numbers", &["math", "core"]),
            mock_module("text.upper", "Uppercase text", &["text"]),
        ]);
        let output = cmd_list(&registry, &[], Some("json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn test_cmd_list_empty_registry_table() {
        let registry = MockRegistry::new(vec![]);
        let output = cmd_list(&registry, &[], Some("table")).unwrap();
        assert_eq!(output.trim(), "No modules found.");
    }

    #[test]
    fn test_cmd_list_empty_registry_json() {
        let registry = MockRegistry::new(vec![]);
        let output = cmd_list(&registry, &[], Some("json")).unwrap();
        assert_eq!(output.trim(), "[]");
    }

    #[test]
    fn test_cmd_list_tag_filter_single_match() {
        let registry = MockRegistry::new(vec![
            mock_module("math.add", "Add numbers", &["math", "core"]),
            mock_module("text.upper", "Uppercase text", &["text"]),
        ]);
        let output = cmd_list(&registry, &["math"], Some("json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "math.add");
    }

    #[test]
    fn test_cmd_list_tag_filter_and_semantics() {
        let registry = MockRegistry::new(vec![
            mock_module("math.add", "Add numbers", &["math", "core"]),
            mock_module("math.mul", "Multiply", &["math"]),
        ]);
        // Only math.add has BOTH "math" AND "core".
        let output = cmd_list(&registry, &["math", "core"], Some("json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        let arr = parsed.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "math.add");
    }

    #[test]
    fn test_cmd_list_tag_filter_no_match_table() {
        let registry = MockRegistry::new(vec![
            mock_module("math.add", "Add numbers", &["math"]),
        ]);
        let output = cmd_list(&registry, &["nonexistent"], Some("table")).unwrap();
        assert!(output.contains("No modules found matching tags:"));
        assert!(output.contains("nonexistent"));
    }

    #[test]
    fn test_cmd_list_tag_filter_no_match_json() {
        let registry = MockRegistry::new(vec![
            mock_module("math.add", "Add numbers", &["math"]),
        ]);
        let output = cmd_list(&registry, &["nonexistent"], Some("json")).unwrap();
        assert_eq!(output.trim(), "[]");
    }

    #[test]
    fn test_cmd_list_invalid_tag_format_returns_error() {
        let registry = MockRegistry::new(vec![]);
        let result = cmd_list(&registry, &["INVALID!"], Some("json"));
        assert!(result.is_err());
        match result.unwrap_err() {
            DiscoveryError::InvalidTag(tag) => assert_eq!(tag, "INVALID!"),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn test_cmd_list_description_truncated_in_table() {
        let long_desc = "x".repeat(100);
        let registry = MockRegistry::new(vec![
            mock_module("a.b", &long_desc, &[]),
        ]);
        let output = cmd_list(&registry, &[], Some("table")).unwrap();
        assert!(output.contains("..."), "long description must be truncated");
        assert!(!output.contains(&"x".repeat(100)), "full description must not appear");
    }

    #[test]
    fn test_cmd_list_json_contains_id_description_tags() {
        let registry = MockRegistry::new(vec![
            mock_module("a.b", "Desc", &["x", "y"]),
        ]);
        let output = cmd_list(&registry, &[], Some("json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        let entry = &parsed[0];
        assert!(entry.get("id").is_some());
        assert!(entry.get("description").is_some());
        assert!(entry.get("tags").is_some());
    }
```

Run `cargo test test_cmd_list` — all fail (`cmd_list` is not yet implemented).

---

## GREEN — Implement

The `cmd_list` function is introduced in the `tag-validation` task's GREEN step as part of the full `discovery.rs` rewrite. This task's GREEN step verifies that the implementation from `tag-validation` covers all the tests above by running them. If any tests fail, adjust `cmd_list` accordingly.

Expected adjustments from running these tests:

1. **Tag ordering in AND filter**: ensure `tags.iter().all(...)` checks against the module's own tag list correctly — use `Vec<&str>` to avoid ownership issues.
2. **JSON output field name**: `format_module_list` emits `"id"` (not `"module_id"`), which is correct per the output-formatter spec (`extract_str` uses `["module_id", "id", "canonical_id"]` as fallback keys).
3. **Empty registry JSON**: `format_module_list(&[], "json", &[])` must return `"[]"` — verified by the output-formatter tests.

---

## REFACTOR

- Extract the tag-filter predicate into a named helper `fn module_has_all_tags(module: &Value, tags: &[&str]) -> bool` to improve readability.
- Run `cargo clippy -- -D warnings` on `src/discovery.rs`.
- Confirm no `unwrap()` calls remain outside of tests.

---

## Verification

```bash
cargo test test_cmd_list 2>&1
# Expected: 9 tests pass, 0 fail.

cargo test --test test_discovery 2>&1
# Expected: list-related tests pass (others may still fail at this stage).
```
