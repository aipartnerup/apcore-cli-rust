# Task: exec-dispatch-callback

**Feature**: FE-01 Core Dispatcher
**File**: `src/cli.rs`, `src/main.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~4h
**Depends on**: `build-module-command`
**Required by**: `create-cli-and-main`

---

## Context

This task implements the full execution pipeline triggered when a module subcommand is invoked. It is the Rust equivalent of the Python `build_module_command` callback (the inner `def callback(**kwargs)` closure).

Because clap v4 does not support embedding closures into `Command` objects, the execution logic lives in a standalone async function `dispatch_module` (called from `main`). The function receives pre-parsed `ArgMatches` (or raw argv from `external_subcommand`), runs the full pipeline, and calls `std::process::exit` with the correct code.

### Pipeline Steps (matching Python exactly)

1. `validate_module_id(id)` — exit 2 on bad format.
2. `registry.get_definition(id)` — exit 44 if `None`.
3. `collect_input(stdin_flag, cli_kwargs, large_input)` — exit 2 on stdin errors.
4. `reconvert_enum_values(merged, &schema_args)` — coerce string enums to typed values.
5. Schema validation (`jsonschema`-equivalent) — exit 45 on failure.
6. `check_approval(id, auto_approve)` — exit 46 on denial/timeout.
7. `executor.call(id, merged)` — timed, raced against `ctrl_c` (exit 130 on signal).
8. Audit log success.
9. `format_exec_result(result, format_flag)` — write to stdout.
10. `std::process::exit(0)`.

On executor error: audit log error, write to stderr, exit with mapped code.

### Error Code Map

```rust
fn map_apcore_error_to_exit_code(error_code: &str) -> i32 {
    match error_code {
        "MODULE_NOT_FOUND" | "MODULE_LOAD_ERROR" | "MODULE_DISABLED" => 44,
        "SCHEMA_VALIDATION_ERROR"                                     => 45,
        "APPROVAL_DENIED" | "APPROVAL_TIMEOUT" | "APPROVAL_PENDING"  => 46,
        "CONFIG_NOT_FOUND" | "CONFIG_INVALID"                         => 47,
        "SCHEMA_CIRCULAR_REF"                                         => 48,
        "ACL_DENIED"                                                  => 77,
        _                                                             => 1,
    }
}
```

---

## RED — Write Failing Tests First

Add to `tests/test_cli.rs` (for unit-testable sub-functions) and `tests/test_e2e.rs` (for full-process exit code verification):

```rust
// tests/test_e2e.rs — subprocess-based exit code tests

use std::process::Command;

fn run_apcore(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_apcore-cli"))
        .args(args)
        .output()
        .expect("failed to run apcore-cli")
}

#[test]
fn test_exec_module_not_found_exits_44() {
    let out = run_apcore(&["--extensions-dir", "./tests/fixtures/extensions",
                           "exec", "non.existent"]);
    assert_eq!(out.status.code(), Some(44));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("not found") || stderr.contains("Module"), "stderr: {stderr}");
}

#[test]
fn test_exec_invalid_module_id_exits_2() {
    let out = run_apcore(&["--extensions-dir", "./tests/fixtures/extensions",
                           "exec", "INVALID!ID"]);
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Invalid module ID format"));
}

#[test]
fn test_exec_stdin_exceeds_limit_exits_2() {
    use std::io::Write;
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_apcore-cli"))
        .args(&["--extensions-dir", "./tests/fixtures/extensions",
                "exec", "math.add", "--input", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn failed");
    // Write 11 MiB.
    let payload = vec![b'x'; 11 * 1024 * 1024];
    child.stdin.as_mut().unwrap().write_all(&payload).ok();
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("exceeds 10MB limit"));
}

#[test]
fn test_exec_stdin_invalid_json_exits_2() {
    use std::io::Write;
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_apcore-cli"))
        .args(&["--extensions-dir", "./tests/fixtures/extensions",
                "exec", "math.add", "--input", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn failed");
    child.stdin.as_mut().unwrap().write_all(b"not valid json").ok();
    drop(child.stdin.take());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("does not contain valid JSON"));
}
```

Run `cargo test --test test_e2e` — all fail (dispatch not implemented).

---

## GREEN — Implement

Add `dispatch_module` to `src/cli.rs`:

```rust
/// Execute a module by ID: validate → collect input → validate schema
/// → approve → execute → audit → output.
///
/// Calls `std::process::exit` with the appropriate code; never returns normally.
pub async fn dispatch_module(
    module_id: &str,
    matches: &clap::ArgMatches,
    registry: &Arc<dyn Registry + Send + Sync>,
    executor: &Arc<dyn Executor + Send + Sync>,
) -> ! {
    // 1. Validate module ID.
    if let Err(e) = validate_module_id(module_id) {
        eprintln!("Error: {e}");
        std::process::exit(EXIT_INVALID_INPUT);
    }

    // 2. Registry lookup.
    let module_def = match registry.get_definition(module_id) {
        Ok(Some(def)) => def,
        Ok(None) => {
            eprintln!("Error: Module '{module_id}' not found in registry.");
            std::process::exit(EXIT_MODULE_NOT_FOUND);
        }
        Err(e) => {
            let exit_code = map_apcore_error_to_exit_code(e.code().unwrap_or(""));
            eprintln!("Error: Module '{module_id}' failed to load: {e}.");
            std::process::exit(exit_code);
        }
    };

    // 3. Extract built-in flags from matches.
    let stdin_flag = matches.get_one::<String>("input").map(|s| s.as_str());
    let auto_approve = matches.get_flag("yes");
    let large_input = matches.get_flag("large-input");
    let format_flag = matches.get_one::<String>("format").cloned();

    // 4. Build CLI kwargs from remaining args (schema-derived flags).
    let cli_kwargs = extract_cli_kwargs(matches, &module_def);

    // 5. Collect and merge input.
    let merged = match collect_input(stdin_flag, cli_kwargs, large_input) {
        Ok(m) => m,
        Err(CliError::InputTooLarge { .. }) => {
            eprintln!("Error: STDIN input exceeds 10MB limit. Use --large-input to override.");
            std::process::exit(EXIT_INVALID_INPUT);
        }
        Err(CliError::JsonParse(detail)) => {
            eprintln!("Error: STDIN does not contain valid JSON: {detail}.");
            std::process::exit(EXIT_INVALID_INPUT);
        }
        Err(CliError::NotAnObject) => {
            eprintln!("Error: STDIN JSON must be an object, got array or scalar.");
            std::process::exit(EXIT_INVALID_INPUT);
        }
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(EXIT_INVALID_INPUT);
        }
    };

    // 6. Schema validation (if module has input_schema with properties).
    if let Some(schema) = &module_def.input_schema {
        if schema.get("properties").is_some() {
            if let Err(detail) = validate_against_schema(&merged, schema) {
                eprintln!("Error: Validation failed: {detail}.");
                std::process::exit(EXIT_SCHEMA_VALIDATION_ERROR);
            }
        }
    }

    // 7. Approval gate.
    if let Err(e) = check_approval(module_id, auto_approve) {
        eprintln!("Error: {e}");
        std::process::exit(EXIT_APPROVAL_DENIED);
    }

    // 8. Execute with SIGINT race.
    let start = std::time::Instant::now();
    let exec_future = executor.call(module_id, serde_json::to_value(&merged).unwrap());
    let result = tokio::select! {
        res = exec_future => res,
        _ = tokio::signal::ctrl_c() => {
            eprintln!("Execution cancelled.");
            std::process::exit(EXIT_SIGINT);
        }
    };
    let duration_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(output) => {
            // 9. Audit log success.
            if let Ok(guard) = AUDIT_LOGGER.lock() {
                if let Some(logger) = guard.as_ref() {
                    logger.log_execution(module_id, &merged, "success", 0, duration_ms);
                }
            }
            // 10. Format and output.
            format_exec_result(&output, format_flag.as_deref());
            std::process::exit(EXIT_SUCCESS);
        }
        Err(e) => {
            let error_code = e.code().unwrap_or("");
            let exit_code = map_apcore_error_to_exit_code(error_code);
            // Audit log error.
            if let Ok(guard) = AUDIT_LOGGER.lock() {
                if let Some(logger) = guard.as_ref() {
                    logger.log_execution(module_id, &merged, "error", exit_code as u32, 0);
                }
            }
            eprintln!("Error: Module '{module_id}' execution failed: {e}.");
            std::process::exit(exit_code);
        }
    }
}
```

Add `set_audit_logger` implementation:

```rust
pub fn set_audit_logger(audit_logger: Option<AuditLogger>) {
    if let Ok(mut guard) = AUDIT_LOGGER.lock() {
        *guard = audit_logger;
    }
}
```

Add `validate_against_schema` as a private helper that uses `serde_json`/`jsonschema` crate (add `jsonschema = "0.18"` to `Cargo.toml` if not already present, or use a lightweight inline checker).

Run `cargo test --test test_e2e` — tests pass.

---

## REFACTOR

- Introduce `fn map_apcore_error_to_exit_code(code: &str) -> i32` as a standalone `pub(crate)` function and add unit tests for each mapping variant.
- Ensure exact stderr message wording matches the spec table in `core-dispatcher.md` section 6.
- Run `cargo clippy -- -D warnings`.

---

## Verification

```bash
# Unit tests for error code mapping:
cargo test map_apcore_error 2>&1

# E2E exit code tests:
cargo test --test test_e2e exec_ 2>&1

# Spot-check with real binary (requires built extensions fixture):
echo '{"a":5,"b":10}' | ./target/debug/apcore-cli \
    --extensions-dir tests/fixtures/extensions \
    exec math.add --input -
# Expected: exit 0, result on stdout
```
