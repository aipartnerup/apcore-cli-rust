// apcore-cli — Subprocess sandbox for module execution.
// Protocol spec: SEC-04 (Sandbox, ModuleExecutionError)

use serde_json::Value;
use thiserror::Error;

// ---------------------------------------------------------------------------
// ModuleExecutionError
// ---------------------------------------------------------------------------

/// Errors produced during sandboxed module execution.
#[derive(Debug, Error)]
pub enum ModuleExecutionError {
    /// The subprocess exited with a non-zero exit code.
    #[error("module '{module_id}' exited with code {exit_code}")]
    NonZeroExit { module_id: String, exit_code: i32 },

    /// The subprocess timed out.
    #[error("module '{module_id}' timed out after {timeout_ms}ms")]
    Timeout { module_id: String, timeout_ms: u64 },

    /// The subprocess output could not be parsed.
    #[error("failed to parse sandbox output for module '{module_id}': {reason}")]
    OutputParseFailed { module_id: String, reason: String },

    /// Failed to spawn the sandbox subprocess.
    #[error("failed to spawn sandbox process: {0}")]
    SpawnFailed(String),
}

// ---------------------------------------------------------------------------
// Sandbox
// ---------------------------------------------------------------------------

/// Executes modules in an isolated subprocess for security isolation.
///
/// When `enabled` is `false`, execution is performed in-process (no sandbox).
/// When `enabled` is `true`, a child process running `_sandbox_runner` handles
/// the execution and communicates results via JSON over stdin/stdout.
pub struct Sandbox {
    enabled: bool,
    timeout_ms: u64,
}

impl Sandbox {
    /// Create a new `Sandbox`.
    ///
    /// # Arguments
    /// * `enabled`    — enable subprocess isolation
    /// * `timeout_ms` — subprocess timeout in milliseconds (0 = no timeout)
    pub fn new(enabled: bool, timeout_ms: u64) -> Self {
        Self { enabled, timeout_ms }
    }

    /// Execute a module, optionally in an isolated subprocess.
    ///
    /// # Arguments
    /// * `module_id`  — identifier of the module to execute
    /// * `input_data` — JSON input for the module
    ///
    /// Returns the module output as a `serde_json::Value`.
    ///
    /// # Errors
    /// Returns `ModuleExecutionError` on timeout, non-zero exit, or parse failure.
    pub async fn execute(
        &self,
        module_id: &str,
        input_data: Value,
    ) -> Result<Value, ModuleExecutionError> {
        // TODO: if !enabled, call executor directly;
        //       otherwise spawn _sandbox_runner subprocess, pipe input,
        //       collect output, decode result.
        let _ = (module_id, input_data);
        todo!("Sandbox::execute")
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sandbox_disabled_executes_inline() {
        // Sandbox::new(false, 0).execute must run in-process.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_sandbox_enabled_spawns_subprocess() {
        // Sandbox::new(true, 5000).execute must spawn a child process.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_sandbox_timeout_returns_error() {
        // A subprocess that exceeds timeout_ms must yield Timeout error.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_sandbox_nonzero_exit_returns_error() {
        // A subprocess exiting non-zero must yield NonZeroExit error.
        assert!(false, "not implemented");
    }
}
