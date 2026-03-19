# Schema Parser — Rust Port Overview

**Feature ID**: FE-02
**Status**: planned
**Language**: Rust 2021
**Source**: Python implementation in `apcore-cli-python/src/apcore_cli/`

---

## What This Feature Does

The Schema Parser converts a module's JSON Schema `input_schema` into `clap::Arg` instances that are attached to a dynamically-built `clap::Command`. It:

1. Resolves all `$ref` references by inlining definitions from `$defs` / `definitions`, enforcing a depth limit of 32 and detecting circular chains.
2. Flattens schema composition keywords: `allOf` merges properties (last wins, required extended); `anyOf` and `oneOf` union properties and intersect required lists.
3. Maps each schema property to a `clap::Arg`:
   - `string` → plain string arg; `_file`-suffix or `x-cli-file: true` → `PathBuf` value_parser
   - `integer` → `value_parser!(i64)`
   - `number` → `value_parser!(f64)`
   - `boolean` → two args: `--flag` (`SetTrue`) and `--no-flag` (`SetFalse`)
   - `object`, `array` → plain string arg (JSON string expected)
   - Unknown / missing type → plain string arg + warning log
4. Restricts accepted values for `enum` properties via `PossibleValuesParser`.
5. Detects flag name collisions (underscore-to-hyphen normalisation) and returns `SchemaParserError::FlagCollision` (caller exits 48).
6. Extracts help text: `x-llm-description` takes precedence over `description`; text exceeding configurable limit (default 1000 chars, via `cli.help_text_max_length`) is truncated to `(limit - 3)` + `"..."`.
7. Provides `reconvert_enum_values` to coerce string values parsed by clap back to their original JSON types (Number, Bool) after parsing.

---

## Rust-Specific Design Decisions

### `SchemaArgs` Instead of `Vec<clap::Arg>`

The Python `schema_to_click_options` returns `list[click.Option]`. The Rust port returns `SchemaArgs`:

```rust
pub struct SchemaArgs {
    pub args: Vec<clap::Arg>,
    pub bool_pairs: Vec<BoolFlagPair>,
    pub enum_maps: HashMap<String, Vec<serde_json::Value>>,
}
```

`bool_pairs` carries the metadata needed to reconcile `--flag` / `--no-flag` into a single bool after clap parsing. `enum_maps` carries original JSON-typed enum values for `reconvert_enum_values`.

### No Native `--flag/--no-flag`

Python's `click.Option(["--verbose/--no-verbose"])` is a single object. Clap v4 has no equivalent. Two separate `Arg`s are created with `ArgAction::SetTrue` and `ArgAction::SetFalse`. The dispatcher reconciles them using `reconcile_bool_pairs` in `cli.rs`.

### `resolve_refs` Returns Owned `Value`

The existing stub signature `fn resolve_refs(schema: &mut Value, max_depth: usize, module_id: &str) -> Result<Value, RefResolverError>` is retained. Internally, `resolve_refs` deep-copies the input and does not mutate the caller's value. The returned `Value` is the fully-inlined copy with `$defs` / `definitions` removed.

### Error Handling vs `std::process::exit`

`schema_to_clap_args` returns `Err(SchemaParserError::FlagCollision)` rather than calling `exit(48)` directly. `resolve_refs` returns `Err(RefResolverError::...)` for all error conditions. The caller in `cli.rs` maps errors to the correct exit codes:

| Error | Exit code |
|-------|-----------|
| `RefResolverError::Unresolvable` | 45 |
| `RefResolverError::Circular` | 48 |
| `RefResolverError::MaxDepthExceeded` | 48 |
| `SchemaParserError::FlagCollision` | 48 |

---

## Files Modified / Created

| File | Role |
|------|------|
| `src/ref_resolver.rs` | `RefResolverError`, `resolve_refs`, `resolve_node` (private) |
| `src/schema_parser.rs` | `SchemaParserError`, `BoolFlagPair`, `SchemaArgs`, `schema_to_clap_args`, `reconvert_enum_values`, `extract_help` (private) |
| `src/cli.rs` | Update `build_module_command` call site; add `reconcile_bool_pairs` |
| `src/lib.rs` | Re-export new types |
| `tests/test_ref_resolver.rs` | Replace `assert!(false)` stubs with real assertions |
| `tests/test_schema_parser.rs` | Replace `assert!(false)` stubs with real assertions; add full-pipeline tests |

---

## Exit Code Reference

| Code | Condition |
|------|-----------|
| 45 | Unresolvable `$ref` |
| 48 | Circular `$ref`, depth > 32, or flag name collision |

---

## Task Execution Order

```
ref-resolver-core
      │
      └── schema-composition
                │
                └── type-mapping
                      │
                      ├── boolean-flag-pairs
                      │         │
                      │         └── help-text-and-collision
                      │                    │
                      └── enum-choices ────┘
                                           │
                                    reconvert-enum-values
                                           │
                                  update-clap-arg-call-site
```

Full sequence:
`ref-resolver-core` → `schema-composition` → `type-mapping` → `boolean-flag-pairs` + `enum-choices` (parallel) → `help-text-and-collision` → `reconvert-enum-values` → `update-clap-arg-call-site`

---

## Acceptance Gate

All tasks complete when `cargo test` reports zero failures, zero `assert!(false)` stubs remain, and `cargo clippy -- -D warnings` produces no warnings.
