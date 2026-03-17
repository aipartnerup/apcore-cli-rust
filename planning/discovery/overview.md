# Discovery — Rust Port Overview

**Feature ID**: FE-04
**Status**: planned
**Language**: Rust 2021
**Source**: Python implementation in `apcore-cli-python/src/apcore_cli/discovery.py`

---

## What This Feature Does

The Discovery feature provides `list` and `describe` as first-class clap subcommands on the root `apcore-cli` CLI. It:

1. Lists all modules registered in the apcore registry as a table or JSON array, with optional tag filtering (AND semantics).
2. Describes a single module's full metadata — description, input/output schema, annotations, extension fields, and tags — as a rich table or JSON object.
3. Defaults to `"table"` on a TTY and `"json"` on a non-TTY. An explicit `--format` flag overrides detection (invalid values are rejected by clap at parse time, exit 2).
4. Validates tag formats (`^[a-z][a-z0-9_-]*$`) — invalid tag → exit 2; non-existent tag → empty result, exit 0.
5. Validates module ID format and existence for `describe` — invalid format → exit 2; not found → exit 44.

---

## Rust-Specific Design Decisions

### `RegistryProvider` Trait for Testability

The real `apcore::Registry` is not yet wired (that is core-dispatcher scope). Discovery commands depend on a `RegistryProvider` trait:

```rust
pub trait RegistryProvider: Send + Sync {
    fn list(&self) -> Vec<String>;
    fn get_definition(&self, id: &str) -> Option<serde_json::Value>;
}
```

Tests use `MockRegistry`. The real registry is wired in the `core-dispatcher` feature via a thin adaptor implementing `RegistryProvider`.

### `DiscoveryError` Instead of `std::process::exit`

Command handlers return `Result<String, DiscoveryError>` rather than calling `std::process::exit` directly. This makes them testable without terminating the test process. The binary entry point converts errors to exit codes:

| Error variant | Exit code |
|---------------|-----------|
| `DiscoveryError::InvalidTag` | 2 |
| `DiscoveryError::InvalidModuleId` | 2 |
| `DiscoveryError::ModuleNotFound` | 44 |

### `register_discovery_commands` Returns `Command`

Following the clap v4 builder idiom, `register_discovery_commands` accepts and returns a `Command` (not `&mut Command`):

```rust
pub fn register_discovery_commands(
    cli: Command,
    registry: Arc<dyn RegistryProvider>,
) -> Command
```

The caller chains: `create_cli().pipe(|c| register_discovery_commands(c, registry))`.

### `--tag` Implemented as `ArgAction::Append`

Multiple `--tag` values are collected via `Arg::new("tag").action(ArgAction::Append)`, enabling `--tag math --tag core` with AND semantics. This mirrors the Python `@click.option("--tag", multiple=True)` behaviour.

### `--format` Rejected at Parse Time

`Arg::new("format").value_parser(PossibleValuesParser::new(["table", "json"]))` causes clap to emit `clap::error::ErrorKind::InvalidValue` and exit 2 before the handler runs for any value outside `["table", "json"]`. No runtime format check is needed.

### No Regex Crate

Tag validation uses a hand-written character iterator matching `^[a-z][a-z0-9_-]*$`:

```rust
pub fn validate_tag(tag: &str) -> bool {
    let mut chars = tag.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}
```

This avoids adding the `regex` crate for a simple grammar, consistent with the core-dispatcher plan.

### Delegation to `output` Module

All rendering is delegated to the `output` module (already planned in FE-08):

| Action | Delegation |
|--------|-----------|
| Format resolution | `output::resolve_format(explicit_format)` |
| Module list rendering | `output::format_module_list(&modules, fmt, &filter_tags)` |
| Module detail rendering | `output::format_module_detail(&module, fmt)` |

Discovery commands contain no formatting logic of their own.

---

## Files Modified

| File | Role |
|------|------|
| `src/discovery.rs` | Full rewrite: `DiscoveryError`, `RegistryProvider`, `validate_tag`, `register_discovery_commands`, `list_command`, `describe_command`, `cmd_list`, `cmd_describe`, `MockRegistry` |
| `tests/test_discovery.rs` | Replace all `assert!(false, "not implemented")` stubs with working assertions |
| `src/lib.rs` | Update re-export to include `DiscoveryError`, `RegistryProvider`, `MockRegistry`, `cmd_list`, `cmd_describe` |

No new source files. No `Cargo.toml` changes required.

---

## Task Execution Order

```
tag-validation
      │
  ┌───┴───┐
  │       │
list-command   describe-command
  │       │
  └───┬───┘
      │
register-discovery-commands
```

Full sequence: `tag-validation` → `list-command` + `describe-command` (parallel) → `register-discovery-commands`

---

## Acceptance Gate

All tasks complete when:

- `cargo test` reports zero failures in `src/discovery.rs` (inline unit tests) and `tests/test_discovery.rs` (integration tests).
- `cargo clippy -- -D warnings` produces no warnings in `src/discovery.rs`.
- `cargo build --release` succeeds.
- No `todo!()` macros remain in `src/discovery.rs`.
- No `assert!(false, "not implemented")` calls remain in `src/discovery.rs` or `tests/test_discovery.rs`.
