// apcore-cli — Integration tests for check_approval().
// Protocol spec: FE-05

mod common;

use apcore_cli::approval::{check_approval, ApprovalError};

#[test]
fn test_check_approval_auto_approve_skips_prompt() {
    // auto_approve=true must return Ok without any TTY interaction.
    let result = check_approval("math.add", true);
    assert!(result.is_ok(), "expected Ok for auto_approve=true: {result:?}");
}

#[test]
fn test_check_approval_no_tty_returns_error() {
    // In CI / non-TTY environments, approval must fail with NoTty.
    // auto_approve=false; stdin is not a TTY in test harness.
    let result = check_approval("math.add", false);
    // TODO: assert matches NoTty or Denied depending on environment.
    assert!(false, "not implemented");
}

#[test]
fn test_check_approval_denied_returns_error() {
    // Simulated "n" input must yield ApprovalError::Denied.
    // TODO: inject mock stdin with "n\n" and assert Denied variant.
    assert!(false, "not implemented");
}

#[test]
fn test_check_approval_timeout_returns_error() {
    // A very short timeout must yield ApprovalError::Timeout.
    // TODO: set timeout to 1ms and verify Timeout variant.
    assert!(false, "not implemented");
}

#[test]
fn test_approval_timeout_error_display() {
    let err = ApprovalError::Timeout {
        module_id: "math.add".to_string(),
        seconds: 30,
    };
    assert!(err.to_string().contains("math.add"));
    assert!(err.to_string().contains("30"));
}
