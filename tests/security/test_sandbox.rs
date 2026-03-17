// apcore-cli — Integration tests for Sandbox execution.
// Protocol spec: SEC-04

use apcore_cli::security::sandbox::{ModuleExecutionError, Sandbox};
use serde_json::json;

#[tokio::test]
async fn test_sandbox_disabled_executes_inline() {
    // Sandbox::new(false, 0) must execute without spawning a subprocess.
    let sandbox = Sandbox::new(false, 0);
    let result = sandbox.execute("math.add", json!({"a": 1, "b": 2})).await;
    // TODO: assert result is Ok once execute is implemented.
    assert!(false, "not implemented");
}

#[tokio::test]
async fn test_sandbox_enabled_spawns_subprocess() {
    // Sandbox::new(true, 5000) must route execution through a subprocess.
    let sandbox = Sandbox::new(true, 5000);
    let result = sandbox.execute("math.add", json!({"a": 1, "b": 2})).await;
    // TODO: assert result is Ok (subprocess path).
    assert!(false, "not implemented");
}

#[tokio::test]
async fn test_sandbox_timeout_returns_error() {
    // A very short timeout must yield ModuleExecutionError::Timeout.
    let sandbox = Sandbox::new(true, 1); // 1 ms
    let result = sandbox.execute("slow.module", json!({})).await;
    // TODO: assert Timeout variant.
    assert!(false, "not implemented");
}

#[tokio::test]
async fn test_sandbox_nonzero_exit_returns_error() {
    // A subprocess exiting non-zero must yield NonZeroExit.
    assert!(false, "not implemented");
}
