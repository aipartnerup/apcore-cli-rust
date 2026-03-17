# Task: cli-wiring

**Feature**: FE-06 Shell Integration
**File**: `src/shell.rs`, `tests/test_shell.rs`, `src/lib.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~1.5h
**Depends on**: `completion-command`, `man-page-generator`
**Required by**: none (final task)

---

## Context

This task wires everything together:

1. Implements `register_shell_commands` with the correct clap v4 builder signature.
2. Implements `man_command()` builder.
3. Updates `lib.rs` to export the new public types (`ShellError`).
4. Replaces all `assert!(false, "not implemented")` stubs in `tests/test_shell.rs` with real assertions.
5. Optionally adds a `#[cfg(unix)]` bash syntax-check test using `bash -n`.

The existing `register_shell_commands` stub uses `fn register_shell_commands(cli: &mut Command, prog_name: &str)`. This task changes it to `fn register_shell_commands(cli: Command, prog_name: &str) -> Command` to match the project's clap v4 builder idiom (consistent with `register_discovery_commands`).

The call site in `main.rs` currently passes a mutable reference. After this task, `main.rs` chains the return value instead.

---

## RED — Write Failing Tests First

Replace the stub contents of `tests/test_shell.rs` with:

```rust
// apcore-cli — Integration tests for shell completion and man page commands.
// Protocol spec: FE-10 (FR-SHELL-001, FR-SHELL-002)

mod common;

use apcore_cli::shell::{register_shell_commands, ShellError};
use clap::Command;
use clap_complete::Shell;

fn make_root_cmd() -> Command {
    Command::new("apcore-cli")
        .version("0.2.0")
        .about("Command-line interface for apcore modules")
        .subcommand(Command::new("exec").about("Execute an apcore module"))
        .subcommand(Command::new("list").about("List available modules"))
        .subcommand(Command::new("describe").about("Show module metadata and schema"))
}

#[test]
fn test_register_shell_commands_adds_completion() {
    let root = register_shell_commands(make_root_cmd(), "apcore-cli");
    let names: Vec<_> = root.get_subcommands().map(|c| c.get_name()).collect();
    assert!(
        names.contains(&"completion"),
        "root must have 'completion' subcommand, got: {names:?}"
    );
}

#[test]
fn test_register_shell_commands_adds_man() {
    let root = register_shell_commands(make_root_cmd(), "apcore-cli");
    let names: Vec<_> = root.get_subcommands().map(|c| c.get_name()).collect();
    assert!(
        names.contains(&"man"),
        "root must have 'man' subcommand, got: {names:?}"
    );
}

#[test]
fn test_completion_bash_outputs_nonempty() {
    let mut cmd = make_root_cmd();
    let output = apcore_cli::shell::cmd_completion(Shell::Bash, "apcore-cli", &mut cmd);
    assert!(!output.is_empty(), "bash completion must not be empty");
}

#[test]
fn test_completion_zsh_outputs_nonempty() {
    let mut cmd = make_root_cmd();
    let output = apcore_cli::shell::cmd_completion(Shell::Zsh, "apcore-cli", &mut cmd);
    assert!(!output.is_empty(), "zsh completion must not be empty");
}

#[test]
fn test_completion_fish_outputs_nonempty() {
    let mut cmd = make_root_cmd();
    let output = apcore_cli::shell::cmd_completion(Shell::Fish, "apcore-cli", &mut cmd);
    assert!(!output.is_empty(), "fish completion must not be empty");
}

#[test]
fn test_completion_invalid_shell_rejected_at_parse() {
    // clap rejects unknown shell values at parse time; verify the arg definition
    // uses a value_parser that does not accept arbitrary strings.
    use apcore_cli::shell::completion_command;
    let cmd = completion_command();
    let shell_arg = cmd.get_arguments().find(|a| a.get_id() == "shell");
    assert!(
        shell_arg.is_some(),
        "completion_command must have a 'shell' argument"
    );
    // Verify parse-time rejection by attempting to parse an invalid value.
    // clap returns an error for unknown PossibleValues.
    let result = cmd.clone().try_get_matches_from(["completion", "invalid-shell"]);
    assert!(
        result.is_err(),
        "completion with invalid shell must be rejected by clap"
    );
}

#[test]
fn test_man_command_outputs_nonempty_for_known_builtin() {
    use apcore_cli::shell::cmd_man;
    let root = make_root_cmd();
    let result = cmd_man("list", &root, "apcore-cli", "0.2.0");
    assert!(result.is_ok(), "man for known builtin 'list' must succeed");
    let page = result.unwrap();
    assert!(!page.is_empty(), "man page must not be empty");
    assert!(page.contains(".TH"), "man page must contain .TH");
}

#[test]
fn test_man_command_outputs_nonempty_for_exec() {
    use apcore_cli::shell::cmd_man;
    let root = make_root_cmd();
    let result = cmd_man("exec", &root, "apcore-cli", "0.2.0");
    assert!(result.is_ok(), "man for 'exec' must succeed");
    let page = result.unwrap();
    assert!(page.contains(".SH EXIT CODES"), "man page must have EXIT CODES section");
}

#[test]
fn test_man_command_unknown_returns_error() {
    use apcore_cli::shell::cmd_man;
    let root = make_root_cmd();
    let result = cmd_man("bogus-command", &root, "apcore-cli", "0.2.0");
    assert!(result.is_err());
    match result.unwrap_err() {
        ShellError::UnknownCommand(name) => assert_eq!(name, "bogus-command"),
    }
}

#[test]
#[cfg(unix)]
fn test_completion_bash_valid_syntax() {
    // Validate bash completion script with `bash -n`.
    use std::io::Write;
    let mut cmd = make_root_cmd();
    let script = apcore_cli::shell::cmd_completion(Shell::Bash, "apcore-cli", &mut cmd);
    let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
    tmpfile.write_all(script.as_bytes()).unwrap();
    let status = std::process::Command::new("bash")
        .arg("-n")
        .arg(tmpfile.path())
        .status();
    match status {
        Ok(s) => assert!(s.success(), "bash -n failed on generated completion script"),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // bash not installed — skip silently
        }
        Err(e) => panic!("failed to run bash: {e}"),
    }
}
```

Run `cargo test --test test_shell` — all fail because `register_shell_commands` still uses the old signature and `ShellError` / `cmd_completion` / `cmd_man` are not yet public.

---

## GREEN — Implement

### 1. Update `register_shell_commands` in `src/shell.rs`

```rust
/// Attach the `completion` and `man` subcommands to the given root command.
///
/// Returns the modified command following the clap v4 builder idiom.
pub fn register_shell_commands(cli: Command, prog_name: &str) -> Command {
    let _ = prog_name; // prog_name reserved for future dynamic use
    cli.subcommand(completion_command())
       .subcommand(man_command())
}
```

### 2. Implement `man_command` in `src/shell.rs`

```rust
/// Build the `man` clap subcommand.
fn man_command() -> Command {
    Command::new("man")
        .about("Generate a roff man page for COMMAND and print it to stdout")
        .long_about(
            "Generate a roff man page for COMMAND and print it to stdout.\n\n\
             View immediately:\n\
             \x20 apcore-cli man exec | man -l -\n\
             \x20 apcore-cli man list | col -bx | less\n\n\
             Install system-wide:\n\
             \x20 apcore-cli man exec > /usr/local/share/man/man1/apcore-cli-exec.1\n\
             \x20 mandb   # (Linux)  or  /usr/libexec/makewhatis  # (macOS)",
        )
        .arg(
            clap::Arg::new("command")
                .value_name("COMMAND")
                .required(true)
                .help("CLI subcommand to generate the man page for"),
        )
}
```

### 3. Make public items visible from `src/shell.rs`

Ensure the following are `pub` in `src/shell.rs`:
- `ShellError`
- `KNOWN_BUILTINS`
- `cmd_completion`
- `cmd_man`
- `completion_command`
- `generate_man_page`
- `build_synopsis`
- `register_shell_commands`

### 4. Update `src/lib.rs`

Add `ShellError` to the shell re-export:

```rust
pub use shell::{
    register_shell_commands,
    ShellError,
    cmd_completion,
    cmd_man,
    completion_command,
    generate_man_page,
    build_synopsis,
    KNOWN_BUILTINS,
};
```

### 5. Fix `src/main.rs` call site

Change the `register_shell_commands` invocation from `&mut Command` style to the new builder style. Since `main.rs` uses `create_cli()` which returns a `Command`, the wiring is:

```rust
// Before (stub style):
// register_shell_commands(&mut cmd, &name);

// After (builder style — main.rs create_cli body):
let cmd = clap::Command::new(name)
    .version(env!("CARGO_PKG_VERSION"))
    .about("Command-line interface for apcore modules");
let cmd = register_shell_commands(cmd, "apcore-cli");
cmd
```

Run `cargo test --test test_shell` — all tests pass except optionally the bash syntax test if `bash` is not available.

---

## REFACTOR

- Run `cargo clippy -- -D warnings` on `src/shell.rs` and `tests/test_shell.rs`; fix any warnings.
- Ensure all `todo!()` calls are removed from `src/shell.rs`.
- Ensure all `assert!(false, "not implemented")` calls are removed from `tests/test_shell.rs`.
- Verify `cargo build --release` succeeds.

---

## Verification

```bash
cargo test --test test_shell 2>&1
# Expected: 9+ tests pass (10 on unix with bash installed), 0 fail.

cargo test 2>&1
# Expected: no regressions in other test files.

cargo clippy -- -D warnings 2>&1
# Expected: no warnings in src/shell.rs.

cargo build --release 2>&1
# Expected: clean build.

# Manual smoke test:
cargo run -- completion bash | head -5
cargo run -- man exec | head -10
cargo run -- man nonexistent; echo "exit: $?"
# Expected: exit 2 for nonexistent.
```
