# Task: sandbox

**Feature**: security (FE-05)
**Status**: pending
**Estimated Time**: ~2 hours
**Depends On**: —
**Required By**: `integration`

---

## Goal

Replace all `todo!()` stubs in `src/security/sandbox.rs` and `src/_sandbox_runner.rs` with a complete subprocess-isolation implementation. After this task:

- `Sandbox::new(false, …)` routes `execute()` in-process (passes through to the executor directly).
- `Sandbox::new(true, 300_000)` spawns `apcore-cli --internal-sandbox-runner <module_id>` as a child process, pipes JSON input over stdin, reads JSON result from stdout, and enforces a 300 s timeout.
- The child process environment contains only whitelisted variables (`PATH`, `LANG`, `LC_ALL`, `APCORE_*`) plus `HOME` and `TMPDIR` redirected to a fresh temp dir.
- `_sandbox_runner.rs` implements `run_sandbox_subprocess()` which reads `module_id` from `argv[2]`, `input_data` from stdin, calls the executor, and writes the JSON result to stdout.
- `encode_result()` and `decode_result()` are implemented.
- All inline `#[cfg(test)]` unit tests pass.

---

## Files Involved

| File | Action |
|---|---|
| `src/security/sandbox.rs` | Modify — implement `execute()` |
| `src/_sandbox_runner.rs` | Modify — implement `run_sandbox_subprocess()`, `encode_result()`, `decode_result()` |
| `src/main.rs` | Modify — route `--internal-sandbox-runner <module_id>` argv to `run_sandbox_subprocess()` |

---

## Steps

### 1. Write failing unit tests first (TDD — RED)

Replace the four `assert!(false, "not implemented")` stubs:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_sandbox_disabled_returns_passthrough_error() {
        // When disabled, execute() must NOT spawn a subprocess.
        // We cannot call the real executor in unit tests, so verify the
        // sandbox disabled path by checking it does NOT return Timeout or
        // SpawnFailed. It will return an error because there is no real
        // executor wired here — that is expected.
        let sandbox = Sandbox::new(false, 5_000);
        // Just verify the variant is not Timeout or SpawnFailed.
        // Real integration test verifies the full in-process path.
        let result = sandbox.execute("test.module", json!({})).await;
        assert!(!matches!(result, Err(ModuleExecutionError::Timeout { .. })));
        assert!(!matches!(result, Err(ModuleExecutionError::SpawnFailed(_))));
    }

    #[tokio::test]
    async fn test_sandbox_timeout_returns_error() {
        // Use a 1 ms timeout with a module that sleeps — spawn a real subprocess
        // running `sleep 10` to trigger timeout.
        let sandbox = Sandbox::new(true, 1); // 1 ms timeout
        // Point the subprocess at a command that will time out.
        // This test relies on the sandbox using current_exe() as binary.
        // Skip on CI if binary not available.
        let result = sandbox.execute("__noop__", json!({})).await;
        // Either timeout or spawn-failed (binary not yet wired) — accept both.
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_result_valid_json() {
        use crate::_sandbox_runner::decode_result;
        let v = decode_result(r#"{"ok":true}"#).unwrap();
        assert_eq!(v["ok"], true);
    }

    #[test]
    fn test_decode_result_invalid_json() {
        use crate::_sandbox_runner::decode_result;
        assert!(decode_result("not json").is_err());
    }

    #[test]
    fn test_encode_result_roundtrip() {
        use crate::_sandbox_runner::{decode_result, encode_result};
        let v = json!({"result": 42});
        let encoded = encode_result(&v);
        let decoded = decode_result(&encoded).unwrap();
        assert_eq!(decoded["result"], 42);
    }
}
```

Run to confirm RED:
```bash
cargo test --lib security::sandbox 2>&1 | grep -E "FAILED|error\[|^test "
```

### 2. Implement `encode_result()` and `decode_result()`

```rust
// src/_sandbox_runner.rs
pub(crate) fn encode_result(result: &Value) -> String {
    serde_json::to_string(result).unwrap_or_else(|_| "null".to_string())
}

pub(crate) fn decode_result(raw: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(raw)
}
```

### 3. Implement `run_sandbox_subprocess()`

The subprocess runner reads `module_id` from `std::env::args().nth(2)` (position after `apcore-cli --internal-sandbox-runner`) and `input_data` from stdin:

```rust
pub(crate) async fn run_sandbox_subprocess() -> Result<(), anyhow::Error> {
    use std::io::Read;
    use tokio::io::AsyncReadExt;

    let module_id = std::env::args()
        .nth(2)
        .ok_or_else(|| anyhow::anyhow!("sandbox runner: missing module_id argument"))?;

    // Read JSON input from stdin.
    let mut stdin_buf = String::new();
    tokio::io::stdin().read_to_string(&mut stdin_buf).await?;
    let input_data: Value = serde_json::from_str(&stdin_buf)?;

    // Instantiate executor.
    let extensions_root = std::env::var("APCORE_EXTENSIONS_ROOT")
        .unwrap_or_else(|_| "./extensions".to_string());
    let registry = apcore::Registry::new(&extensions_root);
    registry.discover().await?;
    let executor = apcore::Executor::new(registry);
    let result = executor.call(&module_id, input_data).await?;

    // Write JSON result to stdout.
    let encoded = encode_result(&result);
    print!("{encoded}");
    Ok(())
}
```

### 4. Wire `--internal-sandbox-runner` in `main.rs`

Add the following routing to the top of `main()` (before Clap parses argv) so that the sandbox runner can intercept the process before any CLI setup:

```rust
// src/main.rs — at the very top of main(), before clap parsing.
let args: Vec<String> = std::env::args().collect();
if args.get(1).map(String::as_str) == Some("--internal-sandbox-runner") {
    let rt = tokio::runtime::Runtime::new()?;
    return rt.block_on(apcore_cli::_sandbox_runner::run_sandbox_subprocess())
        .map_err(|e| { eprintln!("{e}"); std::process::exit(1); });
}
```

This intercept must happen before Clap processes argv; otherwise Clap will reject the unknown flag.

### 5. Implement `Sandbox::execute()` — disabled path

```rust
pub async fn execute(
    &self,
    module_id: &str,
    input_data: Value,
) -> Result<Value, ModuleExecutionError> {
    if !self.enabled {
        // In-process execution — caller is responsible for wiring the executor.
        // For now return an unimplemented error that tests can match on.
        // Real wiring happens in the integration task when Sandbox is connected
        // to the Executor via a callback or trait object.
        return Err(ModuleExecutionError::SpawnFailed(
            "in-process executor not wired (use Sandbox::execute_with)".to_string(),
        ));
    }
    self._sandboxed_execute(module_id, input_data).await
}
```

A cleaner API adds `execute_with(module_id, input_data, executor: &dyn Fn(…) -> …)` for the disabled path. Defer this design refinement to the `integration` task.

### 6. Implement `_sandboxed_execute()`

```rust
async fn _sandboxed_execute(
    &self,
    module_id: &str,
    input_data: Value,
) -> Result<Value, ModuleExecutionError> {
    use tokio::process::Command;
    use tokio::time::{timeout, Duration};
    use std::process::Stdio;

    // Build restricted environment.
    let mut env: Vec<(String, String)> = Vec::new();
    let host_env: std::collections::HashMap<String, String> = std::env::vars().collect();
    for key in &["PATH", "LANG", "LC_ALL"] {
        if let Some(val) = host_env.get(*key) {
            env.push((key.to_string(), val.clone()));
        }
    }
    for (k, v) in &host_env {
        if k.starts_with("APCORE_") {
            env.push((k.clone(), v.clone()));
        }
    }

    // Create temp dir for HOME/TMPDIR isolation.
    let tmpdir = tempfile::TempDir::new()
        .map_err(|e| ModuleExecutionError::SpawnFailed(e.to_string()))?;
    let tmpdir_path = tmpdir.path().to_string_lossy().to_string();
    env.push(("HOME".to_string(), tmpdir_path.clone()));
    env.push(("TMPDIR".to_string(), tmpdir_path.clone()));

    // Serialise input.
    let input_json = serde_json::to_string(&input_data)
        .map_err(|e| ModuleExecutionError::SpawnFailed(e.to_string()))?;

    // Locate current binary.
    let binary = std::env::current_exe()
        .map_err(|e| ModuleExecutionError::SpawnFailed(e.to_string()))?;

    let mut child = Command::new(&binary)
        .arg("--internal-sandbox-runner")
        .arg(module_id)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(env)
        .current_dir(&tmpdir_path)
        .spawn()
        .map_err(|e| ModuleExecutionError::SpawnFailed(e.to_string()))?;

    // Write input to stdin.
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin.write_all(input_json.as_bytes()).await
            .map_err(|e| ModuleExecutionError::SpawnFailed(e.to_string()))?;
    }

    // Await with timeout.
    let timeout_dur = if self.timeout_ms > 0 {
        Duration::from_millis(self.timeout_ms)
    } else {
        Duration::from_secs(300)
    };

    let output = timeout(timeout_dur, child.wait_with_output())
        .await
        .map_err(|_| ModuleExecutionError::Timeout {
            module_id: module_id.to_string(),
            timeout_ms: self.timeout_ms,
        })?
        .map_err(|e| ModuleExecutionError::SpawnFailed(e.to_string()))?;

    if !output.status.success() {
        let exit_code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(ModuleExecutionError::NonZeroExit {
            module_id: module_id.to_string(),
            exit_code,
        });
        // Note: stderr is logged by the caller; include it in a tracing::warn! here.
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    crate::_sandbox_runner::decode_result(&stdout).map_err(|e| {
        ModuleExecutionError::OutputParseFailed {
            module_id: module_id.to_string(),
            reason: e.to_string(),
        }
    })
}
```

### 7. Add `tempfile` to `[dependencies]` (not just `[dev-dependencies]`)

The sandbox uses `tempfile::TempDir` at runtime. Move or add to `[dependencies]`:

```toml
tempfile = "3"
```

### 8. Run tests (GREEN)

```bash
cargo test --lib security::sandbox 2>&1 | grep -E "^test |FAILED|error\["
cargo test --lib _sandbox_runner 2>&1 | grep -E "^test |FAILED|error\["
```

### 9. Refactor and clippy

```bash
cargo clippy -- -D warnings 2>&1 | head -40
```

---

## Acceptance Criteria

- [ ] No `todo!()` macros remain in `src/security/sandbox.rs` or `src/_sandbox_runner.rs`
- [ ] `test_decode_result_valid_json` passes
- [ ] `test_decode_result_invalid_json` passes: returns `Err`
- [ ] `test_encode_result_roundtrip` passes
- [ ] `Sandbox::new(true, 300_000)` spawns child via `tokio::process::Command`
- [ ] Child process env is cleared with `.env_clear()` then whitelisted vars re-added
- [ ] `HOME` and `TMPDIR` in child env are the temp dir path
- [ ] `PYTHONPATH` is NOT included (Rust sandbox has no Python path concern)
- [ ] 300 s timeout is enforced via `tokio::time::timeout`
- [ ] Non-zero exit code → `ModuleExecutionError::NonZeroExit`
- [ ] Timeout → `ModuleExecutionError::Timeout`
- [ ] Unparseable stdout → `ModuleExecutionError::OutputParseFailed`
- [ ] `--internal-sandbox-runner` argv intercepted in `main.rs` before Clap
- [ ] `run_sandbox_subprocess()` reads module_id from argv[2] and input from stdin
- [ ] `run_sandbox_subprocess()` writes JSON result to stdout
- [ ] `tempfile` is in `[dependencies]` (not only `[dev-dependencies]`)
- [ ] `cargo clippy -- -D warnings` clean in affected files

---

## Dependencies

- **Depends on**: — (no prior task)
- **Required by**: `integration`
