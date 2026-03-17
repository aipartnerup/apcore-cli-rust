# Output Formatter — Rust Port Overview

**Feature ID**: FE-08
**Status**: planned
**Language**: Rust 2021
**Source**: Python implementation in `apcore-cli-python/src/apcore_cli/output.py`

---

## What This Feature Does

The Output Formatter provides TTY-adaptive output rendering for `apcore-cli`. It:

1. Detects whether stdout is a terminal (`std::io::IsTerminal`) and defaults to `"table"` for TTY, `"json"` for non-TTY. An explicit `--format` flag overrides detection.
2. Renders module lists as `comfy-table` tables (with truncated descriptions) or JSON arrays.
3. Renders single module detail views as multi-section plain-text panels or full JSON objects.
4. Formats execution results: dict/object → table or JSON, array → JSON, string → plain, null → empty.

---

## Rust-Specific Design Decisions

### Return `String`, Not `()`

The Python functions print directly via `click.echo`. The Rust functions return `String` so callers control output (print to stdout, write to a buffer, compare in tests). This eliminates the need for stdout-capture libraries in tests.

### `resolve_format_inner` for Testable TTY Logic

`std::io::stdout().is_terminal()` returns `false` in CI. A private `resolve_format_inner(explicit_format: Option<&str>, is_tty: bool) -> &'static str` accepts the TTY state as a parameter, making both `true` and `false` paths testable without a subprocess.

### `comfy-table` as `rich` Equivalent

| Python (rich) | Rust (comfy-table) |
|---------------|---------------------|
| `Table(title="Modules")` | `Table::new()` + `set_header(...)` |
| `Panel("Module: ...")` | `render_panel(title)` — single-row table with box border |
| `Syntax(json, "json", theme="monokai")` | `serde_json::to_string_pretty` (no color) |
| `Console().print(table)` | caller calls `println!("{}", format_fn(...))` |

### `format_module_list` Signature Extension

The existing stub omits the `filter_tags` parameter. This port adds `filter_tags: &[&str]` as the third argument to match the Python contract. All current call sites pass `&[]`.

### No `default=str` Fallback Needed

Python's `json.dumps(result, default=str)` handles non-serializable objects. `serde_json::Value` is always serializable by construction; no fallback is needed.

---

## Files Modified / Created

| File | Role |
|------|------|
| `src/output.rs` | All four public functions + `truncate`, `render_panel`, `extract_str`, `extract_tags`, `resolve_format_inner` |
| `tests/test_output.rs` | Replace all `assert!(false, "not implemented")` stubs with working assertions |

No new source files are created. No `Cargo.toml` changes are needed (`comfy-table = "7"` is already present).

---

## Task Execution Order

```
resolve-format-and-truncate
          │
     ┌────┴────┐
     │         │
format-module-list    format-module-detail
     │         │
     └────┬────┘
          │
  format-exec-result
          │
   wire-format-flag
```

Full sequence: `resolve-format-and-truncate` → `format-module-list` + `format-module-detail` (parallel) → `format-exec-result` → `wire-format-flag`

---

## Acceptance Gate

All tasks complete when `cargo test` reports zero failures in `src/output.rs` (inline unit tests) and `tests/test_output.rs` (integration tests), and `cargo clippy -- -D warnings` produces no warnings in `src/output.rs`.
