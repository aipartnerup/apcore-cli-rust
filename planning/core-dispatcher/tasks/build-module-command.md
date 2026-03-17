# Task: build-module-command

**Feature**: FE-01 Core Dispatcher
**File**: `src/cli.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~3h
**Depends on**: `lazy-module-group-skeleton`
**Required by**: `exec-dispatch-callback`

---

## Context

`build_module_command` converts an apcore `ModuleDescriptor` into a `clap::Command`. In Python, this produces a `click.Command` with schema-derived options attached. In Rust, the equivalent generates a `clap::Command` with:

- `name` = `module_def.canonical_id` (or `module_id` fallback)
- `about` = `module_def.description`
- Built-in flags: `--input`, `--yes`/`-y`, `--large-input`, `--format`, `--sandbox`
- Schema-derived flags via `schema_to_clap_args(resolved_schema)` (already stubbed in `schema_parser.rs`)

A key difference from Python: the execution callback is **not** embedded in the `clap::Command` in Rust. Clap v4's derive API encourages separating parsing from execution. The command is used for:
1. Help text generation (name, about, flags).
2. Schema-aware argument parsing when the module is invoked via `exec <module_id>` or as a top-level subcommand.

The actual execution (approval, executor call, audit log) lives in the dispatch callback in `exec-dispatch-callback`.

---

## RED — Write Failing Tests First

Update `tests/test_cli.rs`:

```rust
use common::make_module_descriptor;

#[test]
fn test_build_module_command_name_and_about() {
    let module = make_module_descriptor("math.add", "Add two numbers", None);
    let executor = Arc::new(mock_executor());
    let cmd = build_module_command(&module, executor);
    assert_eq!(cmd.get_name(), "math.add");
    assert!(
        cmd.get_about().map(|s| s.to_string()).unwrap_or_default().contains("Add two numbers"),
        "about must include module description"
    );
}

#[test]
fn test_build_module_command_has_input_flag() {
    let module = make_module_descriptor("a.b", "desc", None);
    let executor = Arc::new(mock_executor());
    let cmd = build_module_command(&module, executor);
    let names: Vec<_> = cmd.get_opts()
        .flat_map(|a| a.get_long())
        .collect();
    assert!(names.contains(&"input"), "must have --input flag");
}

#[test]
fn test_build_module_command_has_yes_flag() {
    let module = make_module_descriptor("a.b", "desc", None);
    let executor = Arc::new(mock_executor());
    let cmd = build_module_command(&module, executor);
    let names: Vec<_> = cmd.get_opts()
        .flat_map(|a| a.get_long())
        .collect();
    assert!(names.contains(&"yes"), "must have --yes flag");
}

#[test]
fn test_build_module_command_has_large_input_flag() {
    let module = make_module_descriptor("a.b", "desc", None);
    let executor = Arc::new(mock_executor());
    let cmd = build_module_command(&module, executor);
    let names: Vec<_> = cmd.get_opts()
        .flat_map(|a| a.get_long())
        .collect();
    assert!(names.contains(&"large-input"), "must have --large-input flag");
}

#[test]
fn test_build_module_command_schema_args_attached() {
    // Schema with one required property "a" of type integer.
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "a": {"type": "integer", "description": "First operand"}
        },
        "required": ["a"]
    });
    let module = make_module_descriptor("math.add", "Add", Some(schema));
    let executor = Arc::new(mock_executor());
    let cmd = build_module_command(&module, executor);
    let arg_names: Vec<_> = cmd.get_opts()
        .flat_map(|a| a.get_long())
        .collect();
    assert!(arg_names.contains(&"a"), "schema property 'a' must become --a flag");
}

#[test]
fn test_build_module_command_reserved_name_conflict_exits() {
    // If schema property name matches a reserved flag (e.g. "input"), exit 2.
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "input": {"type": "string"}
        }
    });
    let module = make_module_descriptor("bad.mod", "desc", Some(schema));
    let executor = Arc::new(mock_executor());
    // This must call std::process::exit(2). Test via std::panic::catch_unwind
    // or by running in a subprocess.
    // For now, assert the function returns a command with a conflict error marker:
    // (exact test mechanism depends on how conflict is surfaced — see GREEN notes)
    let _ = module; // placeholder until implementation is clear
}
```

Run `cargo test build_module_command` — all fail (function panics).

---

## GREEN — Implement

Update `build_module_command` signature to accept real types and implement:

```rust
use apcore::ModuleDescriptor;

const RESERVED_FLAG_NAMES: &[&str] = &["input", "yes", "large-input", "format", "sandbox"];

pub fn build_module_command(
    module_def: &ModuleDescriptor,
    executor: Arc<dyn Executor + Send + Sync>,
) -> clap::Command {
    let module_id = module_def.canonical_id
        .as_deref()
        .unwrap_or(&module_def.module_id)
        .to_string();

    let description = module_def.description.clone().unwrap_or_default();

    // Resolve $refs in input_schema (max depth 32).
    let raw_schema = module_def.input_schema.clone().unwrap_or(serde_json::Value::Null);
    let resolved_schema = if raw_schema.get("properties").is_some() {
        match resolve_refs(&raw_schema, 32, &module_id) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("Failed to resolve $refs for '{module_id}': {e}. Using raw schema.");
                raw_schema
            }
        }
    } else {
        raw_schema
    };

    // Generate schema-derived clap Args.
    let schema_args = schema_to_clap_args(&resolved_schema);

    // Check for reserved name conflicts.
    for arg in &schema_args {
        if let Some(long) = arg.get_long() {
            if RESERVED_FLAG_NAMES.contains(&long) {
                eprintln!(
                    "Error: Module '{module_id}' schema property '{long}' conflicts \
                     with a reserved CLI option name. Rename the property."
                );
                std::process::exit(2);
            }
        }
    }

    // Build command.
    let mut cmd = clap::Command::new(module_id.clone())
        .about(description)
        // Built-in flags:
        .arg(
            clap::Arg::new("input")
                .long("input")
                .value_name("SOURCE")
                .help("Read input from file or STDIN ('-')."),
        )
        .arg(
            clap::Arg::new("yes")
                .long("yes")
                .short('y')
                .action(clap::ArgAction::SetTrue)
                .help("Bypass approval prompts."),
        )
        .arg(
            clap::Arg::new("large-input")
                .long("large-input")
                .action(clap::ArgAction::SetTrue)
                .help("Allow STDIN input larger than 10MB."),
        )
        .arg(
            clap::Arg::new("format")
                .long("format")
                .value_parser(["json", "table"])
                .help("Output format."),
        )
        .arg(
            clap::Arg::new("sandbox")
                .long("sandbox")
                .action(clap::ArgAction::SetTrue)
                .help("Run module in subprocess sandbox."),
        );

    // Attach schema-derived args.
    for arg in schema_args {
        cmd = cmd.arg(arg);
    }

    cmd
}
```

Note: `executor` is accepted but not stored in the `Command` (clap has no user-data attachment). The executor is passed separately to the dispatch callback. Keep the parameter for API symmetry with Python's `build_module_command(module_def, executor)`.

---

## REFACTOR

- Extract `RESERVED_FLAG_NAMES` to a module-level constant shared with the dispatch callback.
- Confirm `clap::Arg::get_long()` API exists in clap 4. If not, use `get_id()` with a mapping.
- Ensure `resolve_refs` signature in `ref_resolver.rs` matches the call: `resolve_refs(schema: &Value, max_depth: usize, module_id: &str) -> Result<Value, _>`. Adjust if the existing stub differs.
- Run `cargo clippy -- -D warnings`.

---

## Verification

```bash
cargo test build_module_command 2>&1
# Expected: test result: ok. N passed; 0 failed
```
