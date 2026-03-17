// apcore-cli — Audit logger.
// Protocol spec: SEC-01 (AuditLogger)

use std::path::PathBuf;

use serde_json::Value;
use thiserror::Error;

// ---------------------------------------------------------------------------
// AuditLogger
// ---------------------------------------------------------------------------

/// Append-only audit logger that records each module execution to a JSONL file.
///
/// When constructed with `path = None`, logging is a no-op (disabled).
#[derive(Debug, Clone)]
pub struct AuditLogger {
    path: Option<PathBuf>,
}

impl AuditLogger {
    /// Create a new `AuditLogger`.
    ///
    /// # Arguments
    /// * `path` — path to the JSONL audit log file; `None` disables logging
    pub fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    /// Log a single module execution event.
    ///
    /// Each event is written as a single JSON line containing:
    /// * `timestamp`   — RFC 3339 UTC timestamp
    /// * `module_id`   — the executed module's identifier
    /// * `input`       — sanitised input data (secrets redacted)
    /// * `output`      — execution result or error description
    /// * `duration_ms` — wall-clock execution time in milliseconds
    pub fn log_execution(
        &self,
        module_id: &str,
        input: &Value,
        output: &Value,
        duration_ms: u64,
    ) -> Result<(), AuditLogError> {
        // TODO: build JSONL record, open file in append mode, write line.
        let _ = (module_id, input, output, duration_ms);
        todo!("AuditLogger::log_execution")
    }
}

/// Errors produced by the audit logger.
#[derive(Debug, Error)]
pub enum AuditLogError {
    #[error("failed to write audit log: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to serialise audit record: {0}")]
    Serialise(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_audit_logger_disabled_no_op() {
        // AuditLogger with path=None must not produce any files.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_audit_logger_writes_jsonl_record() {
        // AuditLogger with a path must write a JSONL line per invocation.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_audit_logger_appends_multiple_records() {
        // Multiple log_execution calls must append separate lines.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_audit_logger_record_contains_required_fields() {
        // Each record must include timestamp, module_id, input, output, duration_ms.
        assert!(false, "not implemented");
    }
}
