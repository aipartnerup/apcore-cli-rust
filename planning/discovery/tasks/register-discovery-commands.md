# Task: register-discovery-commands

**Feature**: FE-04 Discovery
**File**: `src/discovery.rs`, `tests/test_discovery.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~1.5h
**Depends on**: `list-command`, `describe-command`
**Required by**: (none — final integration task for this feature)

---

## Context

This task completes the discovery feature by:

1. Verifying `register_discovery_commands` correctly wires `list_command` and `describe_command` into the root clap `Command`.
2. Replacing all `assert!(false, "not implemented")` stubs in `tests/test_discovery.rs` with fully passing tests.
3. Confirming that invalid `--format` values (e.g. `--format yaml`) are rejected by clap at parse time (exit 2), not at runtime.

The `register_discovery_commands` function itself is implemented in the `tag-validation` task's GREEN step. This task's scope is integration-level verification and test completion.

The clap signature change noted in the plan (returning `Command` rather than mutating `&mut Command`) is also applied here if not already addressed in earlier tasks.

---

## RED — Write Failing Tests First (tests/test_discovery.rs)

Replace the contents of `tests/test_discovery.rs` with:

```rust
// apcore-cli — Integration tests for discovery commands (list + describe).
// Protocol spec: FE-04

mod common;

use std::sync::Arc;

use apcore_cli::discovery::{
    cmd_describe, cmd_list, register_discovery_commands, DiscoveryError, MockRegistry,
    RegistryProvider,
};
use clap::Command;
use serde_json::json;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn make_registry() -> Arc<MockRegistry> {
    Arc::new(MockRegistry::new(vec![
        json!({
            "module_id": "math.add",
            "description": "Add two numbers together.",
            "tags": ["math", "core"]
        }),
        json!({
            "module_id": "text.upper",
            "description": "Convert text to uppercase.",
            "tags": ["text"]
        }),
    ]))
}

fn build_root(registry: Arc<MockRegistry>) -> Command {
    let root = Command::new("apcore-cli");
    register_discovery_commands(root, registry)
}

// ---------------------------------------------------------------------------
// register_discovery_commands
// ---------------------------------------------------------------------------

#[test]
fn test_register_discovery_adds_list_subcommand() {
    let root = build_root(make_registry());
    let subcommand_names: Vec<&str> = root
        .get_subcommands()
        .map(|c| c.get_name())
        .collect();
    assert!(
        subcommand_names.contains(&"list"),
        "root must have 'list' subcommand; found: {subcommand_names:?}"
    );
}

#[test]
fn test_register_discovery_adds_describe_subcommand() {
    let root = build_root(make_registry());
    let subcommand_names: Vec<&str> = root
        .get_subcommands()
        .map(|c| c.get_name())
        .collect();
    assert!(
        subcommand_names.contains(&"describe"),
        "root must have 'describe' subcommand; found: {subcommand_names:?}"
    );
}

// ---------------------------------------------------------------------------
// list — clap argument structure
// ---------------------------------------------------------------------------

#[test]
fn test_list_has_tag_argument() {
    let root = build_root(make_registry());
    let list_cmd = root
        .get_subcommands()
        .find(|c| c.get_name() == "list")
        .expect("list subcommand must exist");
    let has_tag = list_cmd.get_arguments().any(|a| a.get_id() == "tag");
    assert!(has_tag, "list must have --tag argument");
}

#[test]
fn test_list_has_format_argument() {
    let root = build_root(make_registry());
    let list_cmd = root
        .get_subcommands()
        .find(|c| c.get_name() == "list")
        .expect("list subcommand must exist");
    let has_format = list_cmd.get_arguments().any(|a| a.get_id() == "format");
    assert!(has_format, "list must have --format argument");
}

// ---------------------------------------------------------------------------
// describe — clap argument structure
// ---------------------------------------------------------------------------

#[test]
fn test_describe_has_module_id_argument() {
    let root = build_root(make_registry());
    let describe_cmd = root
        .get_subcommands()
        .find(|c| c.get_name() == "describe")
        .expect("describe subcommand must exist");
    let has_id = describe_cmd.get_arguments().any(|a| a.get_id() == "module_id");
    assert!(has_id, "describe must have module_id positional argument");
}

#[test]
fn test_describe_has_format_argument() {
    let root = build_root(make_registry());
    let describe_cmd = root
        .get_subcommands()
        .find(|c| c.get_name() == "describe")
        .expect("describe subcommand must exist");
    let has_format = describe_cmd.get_arguments().any(|a| a.get_id() == "format");
    assert!(has_format, "describe must have --format argument");
}

// ---------------------------------------------------------------------------
// cmd_list — integration with registry
// ---------------------------------------------------------------------------

#[test]
fn test_list_command_json_format() {
    let registry = make_registry();
    let output = cmd_list(registry.as_ref(), &[], Some("json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("must be valid JSON");
    let arr = parsed.as_array().expect("must be JSON array");
    assert_eq!(arr.len(), 2);
}

#[test]
fn test_list_command_table_format() {
    let registry = make_registry();
    let output = cmd_list(registry.as_ref(), &[], Some("table")).unwrap();
    assert!(output.contains("math.add"), "table must contain math.add");
    assert!(output.contains("text.upper"), "table must contain text.upper");
}

#[test]
fn test_list_command_tag_filter_single() {
    let registry = make_registry();
    let output = cmd_list(registry.as_ref(), &["math"], Some("json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "math.add");
}

#[test]
fn test_list_command_tag_filter_and_semantics() {
    let registry = make_registry();
    // Only math.add has both "math" AND "core".
    let output = cmd_list(registry.as_ref(), &["math", "core"], Some("json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1, "AND semantics: only 1 module has both tags");
}

#[test]
fn test_list_command_nonexistent_tag_empty_result_table() {
    let registry = make_registry();
    let output = cmd_list(registry.as_ref(), &["nonexistent"], Some("table")).unwrap();
    assert!(output.contains("No modules found matching tags:"));
    assert!(output.contains("nonexistent"));
}

#[test]
fn test_list_command_nonexistent_tag_empty_result_json() {
    let registry = make_registry();
    let output = cmd_list(registry.as_ref(), &["nonexistent"], Some("json")).unwrap();
    assert_eq!(output.trim(), "[]");
}

#[test]
fn test_list_command_invalid_tag_format_exits_2() {
    let registry = make_registry();
    let result = cmd_list(registry.as_ref(), &["INVALID!"], Some("json"));
    assert!(
        matches!(result, Err(DiscoveryError::InvalidTag(_))),
        "invalid tag format must return InvalidTag error"
    );
}

// ---------------------------------------------------------------------------
// cmd_describe — integration with registry
// ---------------------------------------------------------------------------

#[test]
fn test_describe_command_known_module_json() {
    let registry = make_registry();
    let output = cmd_describe(registry.as_ref(), "math.add", Some("json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).expect("must be valid JSON");
    assert_eq!(parsed["id"], "math.add");
    assert!(
        parsed["description"].as_str().unwrap().contains("Add two numbers"),
        "description must be present"
    );
}

#[test]
fn test_describe_command_known_module_table() {
    let registry = make_registry();
    let output = cmd_describe(registry.as_ref(), "math.add", Some("table")).unwrap();
    assert!(output.contains("math.add"));
    assert!(output.contains("Add two numbers"));
}

#[test]
fn test_describe_command_unknown_module_exits_44() {
    let registry = make_registry();
    let result = cmd_describe(registry.as_ref(), "non.existent", Some("json"));
    assert!(
        matches!(result, Err(DiscoveryError::ModuleNotFound(_))),
        "unknown module must return ModuleNotFound error (caller maps to exit 44)"
    );
}

#[test]
fn test_describe_command_invalid_id_exits_2() {
    let registry = make_registry();
    let result = cmd_describe(registry.as_ref(), "INVALID!ID", Some("json"));
    assert!(
        matches!(result, Err(DiscoveryError::InvalidModuleId(_))),
        "invalid module id must return InvalidModuleId error (caller maps to exit 2)"
    );
}

// ---------------------------------------------------------------------------
// --format flag: PossibleValuesParser rejects invalid values at parse time
// ---------------------------------------------------------------------------

#[test]
fn test_list_format_flag_rejects_yaml_at_parse_time() {
    // Clap must reject "--format yaml" before the handler runs.
    let root = build_root(make_registry());
    let result = root.try_get_matches_from(["apcore-cli", "list", "--format", "yaml"]);
    assert!(result.is_err(), "--format yaml must be rejected by clap");
    let err = result.unwrap_err();
    // Clap error kind for invalid value is InvalidValue.
    assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
}

#[test]
fn test_describe_format_flag_rejects_yaml_at_parse_time() {
    let root = build_root(make_registry());
    let result =
        root.try_get_matches_from(["apcore-cli", "describe", "math.add", "--format", "yaml"]);
    assert!(result.is_err(), "--format yaml must be rejected by clap");
    let err = result.unwrap_err();
    assert_eq!(err.kind(), clap::error::ErrorKind::InvalidValue);
}
```

Run `cargo test --test test_discovery` — all fail (stubs still have `assert!(false, "not implemented")`).

---

## GREEN — Remove stubs and expose MockRegistry publicly

1. In `src/discovery.rs`: ensure `MockRegistry` and `mock_module` are exported for integration tests. Change `pub(crate)` to `pub` within the `#[cfg(test)]` guard — but since integration tests in `tests/` need access, they must be re-exported differently.

   **Resolution**: Move `MockRegistry` and `mock_module` outside the `#[cfg(test)]` block and gate them with `#[cfg(any(test, feature = "test-utils"))]`. Add a `test-utils` feature to `Cargo.toml` if needed:

   ```toml
   [features]
   test-utils = []
   ```

   Or, simpler: expose them unconditionally as `pub` under a `#[doc(hidden)]` attribute, since they are small and harmless in release builds. The project's existing test pattern (no `test-utils` feature in other planning docs) suggests the unconditional `pub` approach with `#[doc(hidden)]`.

2. Ensure `register_discovery_commands` has the correct signature:

   ```rust
   pub fn register_discovery_commands(
       cli: Command,
       registry: Arc<dyn RegistryProvider>,
   ) -> Command
   ```

   Update `src/lib.rs` re-export if the signature changed:

   ```rust
   pub use discovery::{
       register_discovery_commands, DiscoveryError, MockRegistry, RegistryProvider,
       cmd_list, cmd_describe,
   };
   ```

3. Remove all `assert!(false, "not implemented")` calls from both `src/discovery.rs` inline tests and `tests/test_discovery.rs`.

---

## REFACTOR

- Run the full test suite: `cargo test 2>&1` and confirm zero failures.
- Run `cargo clippy -- -D warnings`.
- Confirm `tests/test_discovery.rs` has zero `assert!(false, ...)` calls:
  ```bash
  grep -n 'assert!(false' tests/test_discovery.rs
  # Expected: no output.
  ```
- Confirm `src/discovery.rs` has zero `todo!()` calls:
  ```bash
  grep -n 'todo!' src/discovery.rs
  # Expected: no output.
  ```

---

## Verification

```bash
cargo test --test test_discovery 2>&1
# Expected: all tests pass, 0 fail.

cargo test 2>&1 | grep -E "^test result"
# Expected: test result: ok. N passed; 0 failed.

cargo build --release 2>&1
# Expected: no errors.
```
