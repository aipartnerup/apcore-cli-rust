# Task: tty-prompt-timeout

**ID**: tty-prompt-timeout
**Status**: pending
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~1.5 hr
**Depends on**: non-tty-rejection

---

## Objective

Implement `prompt_with_timeout` — the async TTY prompt that races blocking stdin input against a 60-second `tokio::time::sleep`. Replace the final `todo!("TTY prompt with timeout")` in `check_approval_with_tty`.

No SIGALRM. No platform-specific code. The solution uses:
- `tokio::task::spawn_blocking(|| stdin_readline())` — runs the blocking `stdin().read_line()` on Tokio's blocking thread pool.
- `tokio::select!` — races the blocking read against `tokio::time::sleep(Duration::from_secs(timeout_secs))`.

On timeout, the blocking thread remains parked waiting for stdin. This is acceptable: the process exits immediately with code 46 after returning `Err(ApprovalError::Timeout {...})`, terminating the blocked thread.

### Prompt Format

Write to **stderr** (not stdout):
1. The approval message (custom or default).
2. The prompt line: `"Proceed? [y/N]: "` (no trailing newline before flush).

Accepted responses for approval: `"y"` or `"yes"` (case-insensitive, trimmed). All other inputs — including empty (Enter key) — are treated as denial (default-deny).

---

## RED — Write failing tests first

These tests inject a mock reader to avoid requiring a real TTY. Extract the read logic into a closure parameter for testability.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    // prompt_with_timeout is internal async fn; test via check_approval_with_tty
    // with is_tty=true and a mock reader injected via the internal helper.

    // For unit tests, expose a test-only variant:
    // prompt_with_reader(module_id, message, timeout_secs, reader_fn)

    #[tokio::test]
    async fn user_types_y_returns_ok() {
        let result = prompt_with_reader(
            "test-module",
            "Requires approval.",
            60,
            || Ok("y\n".to_string()),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn user_types_yes_returns_ok() {
        let result = prompt_with_reader(
            "test-module",
            "Requires approval.",
            60,
            || Ok("yes\n".to_string()),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn user_types_YES_uppercase_returns_ok() {
        let result = prompt_with_reader(
            "test-module",
            "Requires approval.",
            60,
            || Ok("YES\n".to_string()),
        )
        .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn user_types_n_returns_denied() {
        let result = prompt_with_reader(
            "test-module",
            "Requires approval.",
            60,
            || Ok("n\n".to_string()),
        )
        .await;
        assert!(matches!(result, Err(ApprovalError::Denied { .. })));
    }

    #[tokio::test]
    async fn user_presses_enter_returns_denied() {
        // Empty input = default deny (N).
        let result = prompt_with_reader(
            "test-module",
            "Requires approval.",
            60,
            || Ok("\n".to_string()),
        )
        .await;
        assert!(matches!(result, Err(ApprovalError::Denied { .. })));
    }

    #[tokio::test]
    async fn user_types_garbage_returns_denied() {
        let result = prompt_with_reader(
            "test-module",
            "Requires approval.",
            60,
            || Ok("maybe\n".to_string()),
        )
        .await;
        assert!(matches!(result, Err(ApprovalError::Denied { .. })));
    }

    #[tokio::test]
    async fn timeout_returns_timeout_error() {
        // Set timeout=0 to fire immediately; reader blocks indefinitely.
        let result = prompt_with_reader(
            "test-module",
            "Requires approval.",
            0, // fires immediately
            || {
                // Simulate a slow/blocking read that never returns.
                std::thread::sleep(std::time::Duration::from_secs(10));
                Ok("y\n".to_string())
            },
        )
        .await;
        match result {
            Err(ApprovalError::Timeout { module_id, seconds }) => {
                assert_eq!(module_id, "test-module");
                assert_eq!(seconds, 0);
            }
            other => panic!("expected Timeout, got {:?}", other),
        }
    }

    // Integration via check_approval_with_tty

    #[tokio::test]
    async fn check_approval_custom_message_displayed() {
        // Verify that the custom approval_message is passed to the prompt.
        // This is a structural test — the message is extracted before calling
        // prompt_with_reader, so its content is tested in annotation-extraction task.
        let module_def = json!({
            "module_id": "mod-custom",
            "annotations": {
                "requires_approval": true,
                "approval_message": "Custom: please confirm."
            }
        });
        // With is_tty=true and auto_approve=true, we bypass before TTY prompt.
        let result = check_approval_with_tty(&module_def, true, true).await;
        assert!(result.is_ok());
    }
}
```

Run: `cargo test --lib approval::tests` — compile failure expected (`prompt_with_reader` does not exist).

---

## GREEN — Implement

### 1. Add `prompt_with_reader` (test-injectable internal function)

```rust
async fn prompt_with_reader<F>(
    module_id: &str,
    message: &str,
    timeout_secs: u64,
    reader: F,
) -> Result<(), ApprovalError>
where
    F: FnOnce() -> std::io::Result<String> + Send + 'static,
{
    // Display message and prompt to stderr.
    eprint!("{}\nProceed? [y/N]: ", message);
    // Flush stderr so the prompt appears before blocking.
    use std::io::Write;
    let _ = std::io::stderr().flush();

    let module_id_owned = module_id.to_string();
    let read_handle = tokio::task::spawn_blocking(reader);

    tokio::select! {
        result = read_handle => {
            match result {
                Ok(Ok(line)) => {
                    let input = line.trim().to_lowercase();
                    if input == "y" || input == "yes" {
                        tracing::info!(
                            "User approved execution of module '{}'.",
                            module_id_owned
                        );
                        Ok(())
                    } else {
                        tracing::warn!(
                            "Approval rejected by user for module '{}'.",
                            module_id_owned
                        );
                        eprintln!("Error: Approval denied.");
                        Err(ApprovalError::Denied { module_id: module_id_owned })
                    }
                }
                Ok(Err(io_err)) => {
                    // stdin closed (EOF) without input — treat as denial.
                    tracing::warn!(
                        "stdin read error for module '{}': {}",
                        module_id_owned,
                        io_err
                    );
                    eprintln!("Error: Approval denied.");
                    Err(ApprovalError::Denied { module_id: module_id_owned })
                }
                Err(join_err) => {
                    // spawn_blocking task panicked.
                    tracing::error!("spawn_blocking panicked: {}", join_err);
                    Err(ApprovalError::Denied { module_id: module_id_owned })
                }
            }
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_secs(timeout_secs)) => {
            tracing::warn!(
                "Approval timed out after {}s for module '{}'.",
                timeout_secs,
                module_id_owned
            );
            eprintln!("Error: Approval prompt timed out after {} seconds.", timeout_secs);
            Err(ApprovalError::Timeout {
                module_id: module_id_owned,
                seconds: timeout_secs,
            })
        }
    }
}
```

### 2. Add `prompt_with_timeout` (production function using real stdin)

```rust
async fn prompt_with_timeout(module_id: &str, message: &str, timeout_secs: u64)
    -> Result<(), ApprovalError>
{
    prompt_with_reader(module_id, message, timeout_secs, || {
        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        Ok(line)
    })
    .await
}
```

### 3. Wire into `check_approval_with_tty`

Replace `todo!("TTY prompt with timeout")` with:

```rust
let message = get_approval_message(module_def, &module_id);
prompt_with_timeout(&module_id, &message, 60).await
```

Run: `cargo test --lib approval::tests` — all tests must pass.

---

## REFACTOR

- Confirm `eprint!` + `stderr().flush()` correctly displays the prompt before blocking. The flush is necessary because stderr may be line-buffered in some environments.
- Confirm timeout_secs=0 in tests fires `tokio::time::sleep(Duration::ZERO)` which yields immediately — this is correct tokio behavior and makes the timeout test fast.
- The abandoned blocking thread (on timeout) is documented behavior. Add a comment in code.

---

## Verification

```
cargo test --lib approval::tests::user_types_y_returns_ok
cargo test --lib approval::tests::user_types_yes_returns_ok
cargo test --lib approval::tests::user_types_YES_uppercase_returns_ok
cargo test --lib approval::tests::user_types_n_returns_denied
cargo test --lib approval::tests::user_presses_enter_returns_denied
cargo test --lib approval::tests::user_types_garbage_returns_denied
cargo test --lib approval::tests::timeout_returns_timeout_error
cargo test --lib approval
cargo clippy -- -D warnings
```

The timeout test (`timeout_returns_timeout_error`) must complete in under 2 seconds (not 10 — the blocking thread sleeps 10s but tokio::select fires at 0s).

---

## Files Modified

- `src/approval.rs` — add `prompt_with_reader`, `prompt_with_timeout`; wire into `check_approval_with_tty`
