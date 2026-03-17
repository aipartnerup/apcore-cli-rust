# Core Dispatcher — Rust Port Overview

**Feature ID**: FE-01
**Status**: planned
**Language**: Rust 2021
**Source**: Python implementation in `apcore-cli-python/src/apcore_cli/`

---

## What This Feature Does

The Core Dispatcher is the primary entry point for `apcore-cli`. It:

1. Resolves the extensions directory from CLI flag, environment variable, config file, or built-in default.
2. Instantiates the apcore `Registry`, discovers modules, and instantiates the `Executor`.
3. Builds the clap `Command` tree with built-in subcommands (`exec`, `list`, `describe`, `completion`, `man`).
4. Dispatches execution to built-in handlers or dynamically-resolved module commands.
5. For module commands: validates the module ID, collects and merges STDIN JSON + CLI flags, validates input against the module schema, gates on user approval, runs the executor, writes output, and audit-logs the result.
6. Enforces exact exit codes matching the spec (0, 1, 2, 44, 45, 46, 47, 48, 77, 130).

---

## Rust-Specific Design Decisions

### No Lazy Clap Group

Python uses `LazyModuleGroup(click.Group)` to hook `list_commands` and `get_command`. Clap v4 has no equivalent. The Rust approach:

- `LazyModuleGroup` is a plain struct with `list_commands()` and `get_command()` used for help enumeration and dispatch, not as a clap extension.
- The root `clap::Command` uses `.allow_external_subcommands(true)` to capture unrecognised subcommand names.
- The `main` async function dispatches unrecognised names to `dispatch_module(name, ...)` after clap runs.

### Pre-parse of `--extensions-dir`

The registry must be instantiated before clap processes arguments (needed to enumerate module subcommands). `extract_extensions_dir` walks raw `std::env::args()` to extract the flag value before clap runs. This mirrors `_extract_extensions_dir` in the Python implementation.

### STDIN Injection for Tests

`collect_input` is split into `collect_input_from_reader<R: Read>` (inner, testable) and `collect_input` (public wrapper using `stdin()`). Tests inject a `std::io::Cursor`.

### SIGINT Handling

`tokio::select!` races the executor future against `tokio::signal::ctrl_c()`. On signal receipt: stderr `"Execution cancelled."`, exit 130.

### Log Level

`tracing-subscriber` with `EnvFilter` + `reload::Layer` supports the three-tier precedence: `APCORE_CLI_LOGGING_LEVEL` > `APCORE_LOGGING_LEVEL` > `warn`. The `--log-level` flag updates the filter at runtime via the reload handle.

---

## Files Modified / Created

| File | Role |
|------|------|
| `src/main.rs` | `extract_extensions_dir`, `create_cli`, `init_tracing`, `main` |
| `src/cli.rs` | `LazyModuleGroup`, `build_module_command`, `collect_input`, `collect_input_from_reader`, `validate_module_id`, `dispatch_module`, `set_audit_logger`, `map_apcore_error_to_exit_code` |
| `src/lib.rs` | Export `collect_input_from_reader` (new) |
| `tests/test_cli.rs` | Unit/integration tests for `collect_input`, `validate_module_id`, `build_module_command` |
| `tests/test_e2e.rs` | Process-level exit code tests for all T-DISP-* scenarios |
| `tests/common/mod.rs` | Mock `Registry` and `Executor` helpers |

---

## Exit Code Reference

| Code | Condition |
|------|-----------|
| 0 | Success |
| 1 | Module execution error / timeout |
| 2 | Invalid input (bad module ID, STDIN parse error, size limit) |
| 44 | Module not found / disabled / load error |
| 45 | Schema validation failure |
| 46 | Approval denied / timeout |
| 47 | Extensions directory missing or unreadable |
| 48 | Schema circular reference |
| 77 | ACL denied |
| 130 | SIGINT (Ctrl-C) |

---

## Task Execution Order

```
validate-module-id
      │
      ├── collect-input
      │         │
      │         └── lazy-module-group-skeleton
      │                       │
      │                       └── build-module-command
      │                                   │
      │                                   └── exec-dispatch-callback
      │                                               │
tracing-setup ──────────────────────────────── create-cli-and-main
```

Full sequence: `validate-module-id` → `collect-input` → `lazy-module-group-skeleton` → `build-module-command` → `exec-dispatch-callback` → `tracing-setup` → `create-cli-and-main`

---

## Acceptance Gate

All tasks complete when `cargo test` reports zero failures and the binary passes the T-DISP-01 through T-DISP-17 verification scenarios from the feature spec.
