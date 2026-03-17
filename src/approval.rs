// apcore-cli — Human-in-the-loop approval gate.
// Protocol spec: FE-05 (check_approval, ApprovalTimeoutError)

use thiserror::Error;

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Errors returned by the approval gate.
#[derive(Debug, Error)]
pub enum ApprovalError {
    /// The operator denied execution (exit code 46).
    #[error("approval denied for module '{module_id}'")]
    Denied { module_id: String },

    /// No interactive TTY is available to prompt the user (exit code 46).
    #[error("no interactive TTY available for approval prompt")]
    NoTty,

    /// The approval prompt timed out (exit code 46).
    #[error("approval timed out after {seconds}s for module '{module_id}'")]
    Timeout { module_id: String, seconds: u64 },
}

/// Convenience alias exposed at the crate root.
pub type ApprovalTimeoutError = ApprovalError;

// ---------------------------------------------------------------------------
// check_approval
// ---------------------------------------------------------------------------

/// Gate module execution behind an interactive approval prompt.
///
/// If `auto_approve` is `true`, the function returns immediately with `Ok(())`.
/// Otherwise it checks for an interactive TTY, prompts the user, and either
/// returns `Ok(())` or an appropriate `ApprovalError`.
///
/// # Arguments
/// * `module_id`    — the module about to be executed
/// * `auto_approve` — skip the prompt when `true`
///
/// # Errors
/// * `ApprovalError::NoTty`    — stdout is not an interactive terminal
/// * `ApprovalError::Denied`   — user typed anything other than `y`/`yes`
/// * `ApprovalError::Timeout`  — prompt timed out
pub fn check_approval(module_id: &str, auto_approve: bool) -> Result<(), ApprovalError> {
    // TODO: implement TTY check, interactive prompt, and timeout.
    let _ = (module_id, auto_approve);
    todo!("check_approval")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_approval_auto_approve_returns_ok() {
        // auto_approve=true must skip the prompt and return Ok.
        // TODO: remove assert!(false) once implemented.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_check_approval_no_tty_returns_error() {
        // When no TTY is present and auto_approve=false, expect NoTty error.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_check_approval_denied_on_negative_input() {
        // User input "n" should yield ApprovalError::Denied.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_check_approval_timeout() {
        // A timeout must yield ApprovalError::Timeout.
        assert!(false, "not implemented");
    }
}
