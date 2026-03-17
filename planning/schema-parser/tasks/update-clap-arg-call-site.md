# Task: update-clap-arg-call-site

**Feature**: FE-02 Schema Parser
**File**: `src/cli.rs`, `src/lib.rs`, `tests/test_schema_parser.rs`, `tests/test_ref_resolver.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~2h
**Depends on**: `reconvert-enum-values`
**Required by**: nothing (final integration task)

---

## Context

Previous tasks changed the public API of `schema_parser`:

- `schema_to_clap_args` now returns `Result<SchemaArgs, SchemaParserError>` instead of `Vec<clap::Arg>`.
- `reconvert_enum_values` now takes `&SchemaArgs` instead of `&[clap::Arg]`.
- New types `SchemaArgs`, `BoolFlagPair`, `SchemaParserError` must be re-exported from `lib.rs`.

The call site in `build_module_command` (in `cli.rs`) must be updated to:

1. Unwrap / handle the `Result` from `schema_to_clap_args`.
2. Attach `schema_args.args` to the clap `Command`.
3. Pass `schema_args` to `reconvert_enum_values` in the dispatch callback.
4. Reconcile `bool_pairs` in `collect_input` or the dispatch callback.

Also: remove all remaining `assert!(false, "not implemented")` stubs from `tests/test_schema_parser.rs` and `tests/test_ref_resolver.rs`, replacing them with real assertions that exercise the now-complete implementations.

---

## Changes Required

### `src/lib.rs`

```rust
pub use schema_parser::{
    extract_help, reconvert_enum_values, schema_to_clap_args,
    BoolFlagPair, SchemaArgs, SchemaParserError,
};
```

Remove the old `pub use schema_parser::{reconvert_enum_values, schema_to_clap_args};` line.

### `src/cli.rs` — `build_module_command`

Update the `schema_to_clap_args` call:

```rust
// In build_module_command:
let schema_args = schema_to_clap_args(&resolved_schema).unwrap_or_else(|e| {
    eprintln!("Error: {e}");
    std::process::exit(48);
});

for arg in schema_args.args {
    cmd = cmd.arg(arg);
}

// Return (cmd, schema_args) or store schema_args in LazyModuleGroup for later use.
```

The `SchemaArgs` value must survive until the dispatch callback runs. Options:

1. Store `schema_args` in `LazyModuleGroup`'s command cache alongside the `clap::Command`.
2. Re-run `schema_to_clap_args` in the dispatch callback (cheap; schema is already resolved).

Option 2 is simpler for the stub stage. The dispatch callback calls `schema_to_clap_args` again on the same resolved schema, then calls `reconvert_enum_values(cli_kwargs, &schema_args)`.

### Boolean pair reconciliation

Add a helper to `cli.rs` or `schema_parser.rs`:

```rust
/// Reconcile --flag / --no-flag pairs from ArgMatches into bool values.
///
/// For each BoolFlagPair in schema_args.bool_pairs:
/// - If --flag was set → insert prop_name = true into result.
/// - If --no-flag was set → insert prop_name = false into result.
/// - If neither → insert prop_name = default_val into result.
pub fn reconcile_bool_pairs(
    matches: &clap::ArgMatches,
    bool_pairs: &[BoolFlagPair],
) -> HashMap<String, serde_json::Value> {
    let mut result = HashMap::new();
    for pair in bool_pairs {
        let pos_set = matches.get_flag(&pair.prop_name);
        let neg_id = format!("no-{}", pair.prop_name);
        let neg_set = matches.get_flag(&neg_id);
        let val = if pos_set {
            true
        } else if neg_set {
            false
        } else {
            pair.default_val
        };
        result.insert(pair.prop_name.clone(), serde_json::Value::Bool(val));
    }
    result
}
```

### `tests/test_schema_parser.rs` — remove remaining stubs

Replace all remaining `assert!(false, "not implemented")` bodies with real assertions. The integration tests that exist (for `reconvert_enum_values` with integer coercion, boolean coercion) were rewritten in `reconvert-enum-values`. Confirm the file compiles and all tests pass.

### `tests/test_ref_resolver.rs` — remove remaining stubs

The test bodies for `test_resolve_refs_circular_returns_error` and `test_resolve_refs_nested_properties` still have `assert!(false, "not implemented")`. Replace with real assertions matching the implementations from `ref-resolver-core` and `schema-composition`.

---

## RED — Write Failing Tests First

Add integration tests to `tests/test_schema_parser.rs` that exercise the full pipeline from schema → `SchemaArgs` → clap parsing → `reconvert_enum_values`:

```rust
#[test]
fn test_full_pipeline_integer_enum_roundtrip() {
    // Build a Command from a schema with an integer enum, parse args, reconvert.
    let schema = json!({
        "properties": {
            "level": {"type": "integer", "enum": [1, 2, 3]}
        }
    });
    let schema_args = schema_to_clap_args(&schema).unwrap();

    let cmd = clap::Command::new("test");
    let cmd = schema_args.args.iter().cloned().fold(cmd, |c, a| c.arg(a));
    let matches = cmd.try_get_matches_from(["test", "--level", "2"]).unwrap();

    // Extract as string (as clap produces).
    let raw_val = matches.get_one::<String>("level").cloned().unwrap();
    let mut kwargs = HashMap::new();
    kwargs.insert("level".to_string(), serde_json::Value::String(raw_val));

    let result = reconvert_enum_values(kwargs, &schema_args);
    assert_eq!(result["level"], json!(2));
    assert!(result["level"].is_number());
}

#[test]
fn test_full_pipeline_boolean_flag_pair() {
    let schema = json!({
        "properties": {"verbose": {"type": "boolean"}}
    });
    let schema_args = schema_to_clap_args(&schema).unwrap();

    let cmd = schema_args.args.iter().cloned().fold(
        clap::Command::new("test"),
        |c, a| c.arg(a),
    );

    // Test --verbose sets true.
    let matches = cmd.clone().try_get_matches_from(["test", "--verbose"]).unwrap();
    assert!(matches.get_flag("verbose"));

    // Test --no-verbose sets the no- flag.
    let matches2 = cmd.try_get_matches_from(["test", "--no-verbose"]).unwrap();
    assert!(matches2.get_flag("no-verbose"));
}
```

Run `cargo test test_full_pipeline` — must fail (CLI stubs not yet wired).

---

## GREEN — Implement

1. Update `lib.rs` exports as described above.
2. Update `build_module_command` in `cli.rs` (still a stub; update the `todo!` to use `SchemaArgs`).
3. Implement `reconcile_bool_pairs` in `cli.rs`.
4. Replace all `assert!(false, "not implemented")` in test files with real assertions.

After these changes, `cargo test` must produce zero failures.

---

## REFACTOR

- Run `cargo clippy -- -D warnings`.
- Confirm `cargo build --release` succeeds.
- If `SchemaArgs` needs to be `Clone` for the re-computation approach, derive `Clone` on it (requires `clap::Arg: Clone`, which it is in clap 4).

---

## Verification

```bash
cargo test 2>&1
# Expected: test result: ok. N passed; 0 failed; 0 ignored
# (zero assert!(false) stubs remaining)

cargo clippy -- -D warnings 2>&1
# Expected: no warnings

cargo build --release 2>&1
# Expected: Finished release
```
