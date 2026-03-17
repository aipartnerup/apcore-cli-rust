// apcore-cli — Integration tests for AuditLogger.
// Protocol spec: SEC-01

use apcore_cli::security::audit::AuditLogger;
use serde_json::json;
use tempfile::tempdir;

#[test]
fn test_audit_logger_disabled_no_file_written() {
    // AuditLogger with path=None must not create any file.
    let logger = AuditLogger::new(None);
    let result = logger.log_execution("math.add", &json!({"a": 1}), &json!({"sum": 2}), 5);
    // TODO: assert Ok and no file created.
    assert!(false, "not implemented");
}

#[test]
fn test_audit_logger_writes_jsonl_record() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("audit.jsonl");
    let logger = AuditLogger::new(Some(log_path.clone()));
    let result = logger.log_execution("math.add", &json!({"a": 1}), &json!({"sum": 2}), 5);
    // TODO: assert Ok, read file, parse first line, check fields.
    assert!(false, "not implemented");
}

#[test]
fn test_audit_logger_appends_multiple_records() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("audit.jsonl");
    let logger = AuditLogger::new(Some(log_path.clone()));
    for i in 0..3 {
        let _ = logger.log_execution(
            "math.add",
            &json!({"a": i}),
            &json!({"sum": i + 1}),
            i as u64,
        );
    }
    // TODO: read file, count lines, assert 3 lines.
    assert!(false, "not implemented");
}

#[test]
fn test_audit_logger_record_has_required_fields() {
    let dir = tempdir().unwrap();
    let log_path = dir.path().join("audit.jsonl");
    let logger = AuditLogger::new(Some(log_path.clone()));
    let _ = logger.log_execution("math.add", &json!({"a": 1}), &json!({"sum": 2}), 10);
    // TODO: parse record, assert timestamp, module_id, input, output, duration_ms present.
    assert!(false, "not implemented");
}
