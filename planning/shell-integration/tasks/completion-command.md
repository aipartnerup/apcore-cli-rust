# Task: completion-command

**Feature**: FE-06 Shell Integration
**File**: `src/shell.rs`
**Type**: RED-GREEN-REFACTOR
**Estimate**: ~1.5h
**Depends on**: `shell-error-type`
**Required by**: `cli-wiring`

---

## Context

This task implements the `completion` subcommand and its underlying handler `cmd_completion`. The entire script-generation logic is delegated to `clap_complete::generate` — no hand-written shell scripts are needed.

Key behaviours:

1. `completion_command()` builds a clap `Command` named `"completion"` with a single positional argument `SHELL`.
2. The `SHELL` argument uses `value_parser(clap::value_parser!(clap_complete::Shell))` so that invalid shell names are rejected by clap at parse time (exit 2, no handler invoked).
3. Valid shell values: `bash`, `zsh`, `fish`, `elvish`, `powershell` (all five variants of `clap_complete::Shell`).
4. `cmd_completion(shell, prog_name, cmd)` calls `clap_complete::generate(shell, cmd, prog_name, &mut buf)` where `buf: Vec<u8>`, then converts to `String` and returns it.
5. Output goes to stdout; exit 0.

The Python implementation hand-wrote three scripts; this Rust port replaces that entirely with `clap_complete`. The dynamic module-ID hook (inline `apcore-cli list --format json | python3 -c ...` in the bash script) is intentionally not ported — it is out of scope for this feature and can be added as a follow-up once the registry is wired.

---

## RED — Write Failing Tests First

Add to the `#[cfg(test)]` block in `src/shell.rs`:

```rust
    use clap_complete::Shell;

    fn make_test_cmd(prog: &str) -> clap::Command {
        clap::Command::new(prog.to_string())
            .about("test")
            .subcommand(clap::Command::new("exec"))
            .subcommand(clap::Command::new("list"))
    }

    #[test]
    fn test_cmd_completion_bash_nonempty() {
        let mut cmd = make_test_cmd("apcore-cli");
        let output = cmd_completion(Shell::Bash, "apcore-cli", &mut cmd);
        assert!(!output.is_empty(), "bash completion output must not be empty");
    }

    #[test]
    fn test_cmd_completion_zsh_nonempty() {
        let mut cmd = make_test_cmd("apcore-cli");
        let output = cmd_completion(Shell::Zsh, "apcore-cli", &mut cmd);
        assert!(!output.is_empty(), "zsh completion output must not be empty");
    }

    #[test]
    fn test_cmd_completion_fish_nonempty() {
        let mut cmd = make_test_cmd("apcore-cli");
        let output = cmd_completion(Shell::Fish, "apcore-cli", &mut cmd);
        assert!(!output.is_empty(), "fish completion output must not be empty");
    }

    #[test]
    fn test_cmd_completion_elvish_nonempty() {
        let mut cmd = make_test_cmd("apcore-cli");
        let output = cmd_completion(Shell::Elvish, "apcore-cli", &mut cmd);
        assert!(!output.is_empty(), "elvish completion output must not be empty");
    }

    #[test]
    fn test_cmd_completion_bash_contains_prog_name() {
        let mut cmd = make_test_cmd("my-tool");
        let output = cmd_completion(Shell::Bash, "my-tool", &mut cmd);
        assert!(
            output.contains("my-tool") || output.contains("my_tool"),
            "bash completion must reference the program name"
        );
    }

    #[test]
    fn test_completion_command_has_shell_arg() {
        let cmd = completion_command();
        let arg = cmd.get_arguments().find(|a| a.get_id() == "shell");
        assert!(arg.is_some(), "completion_command must have a 'shell' argument");
    }

    #[test]
    fn test_completion_command_name() {
        let cmd = completion_command();
        assert_eq!(cmd.get_name(), "completion");
    }
```

Run `cargo test test_cmd_completion test_completion_command` — all fail because `cmd_completion` and `completion_command` are not yet implemented.

---

## GREEN — Implement

```rust
use clap_complete::{generate, Shell};

/// Handler: generate a shell completion script and return it as a String.
///
/// `shell`     — the target shell (parsed from clap argument)
/// `prog_name` — the program name to embed in the script
/// `cmd`       — mutable reference to the root Command (required by clap_complete)
pub fn cmd_completion(shell: Shell, prog_name: &str, cmd: &mut clap::Command) -> String {
    let mut buf: Vec<u8> = Vec::new();
    generate(shell, cmd, prog_name, &mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

/// Build the `completion` clap subcommand.
pub fn completion_command() -> clap::Command {
    clap::Command::new("completion")
        .about("Generate a shell completion script and print it to stdout")
        .long_about(
            "Generate a shell completion script and print it to stdout.\n\n\
             Install examples:\n\
             \x20 bash:       eval \"$(apcore-cli completion bash)\"\n\
             \x20 zsh:        eval \"$(apcore-cli completion zsh)\"\n\
             \x20 fish:       apcore-cli completion fish | source\n\
             \x20 elvish:     eval (apcore-cli completion elvish)\n\
             \x20 powershell: apcore-cli completion powershell | Out-String | Invoke-Expression",
        )
        .arg(
            clap::Arg::new("shell")
                .value_name("SHELL")
                .required(true)
                .value_parser(clap::value_parser!(Shell))
                .help("Shell to generate completions for (bash, zsh, fish, elvish, powershell)"),
        )
}
```

Run `cargo test test_cmd_completion test_completion_command` — all pass.

---

## REFACTOR

- Confirm `cmd_completion` has no `unwrap()` calls (it does not — `generate` writes infallibly and `from_utf8_lossy` never fails).
- Run `cargo clippy -- -D warnings` on `src/shell.rs`; fix any warnings.
- Check that `completion_command` long_about uses `\n` correctly in roff context (it does not affect roff; this is only clap help text).

---

## Verification

```bash
cargo test test_cmd_completion test_completion_command 2>&1
# Expected: 7 tests pass, 0 fail.

cargo clippy -- -D warnings 2>&1
# Expected: no new warnings in src/shell.rs.
```
