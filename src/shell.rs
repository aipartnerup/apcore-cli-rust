// apcore-cli — Shell completion and man page generation.
// Protocol spec: FE-10 (register_shell_commands)

use clap::Command;

// ---------------------------------------------------------------------------
// register_shell_commands
// ---------------------------------------------------------------------------

/// Attach the `completion` and `man` subcommands to the given root command.
///
/// * `completion <shell>` — emit shell completion script to stdout
///   Supported shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`
/// * `man`                — emit a man page to stdout
pub fn register_shell_commands(cli: &mut Command, prog_name: &str) {
    // TODO: build completion_command(prog_name) and man_command(), attach.
    let _ = (cli, prog_name);
    todo!("register_shell_commands")
}

/// Build the `completion` subcommand using `clap_complete`.
fn completion_command(prog_name: &str) -> Command {
    // TODO: add SHELL positional arg with possible_values; implement callback.
    let _ = prog_name;
    todo!("completion_command")
}

/// Build the `man` subcommand.
fn man_command() -> Command {
    // TODO: emit man-page text via clap_mangen or manual formatting.
    todo!("man_command")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_shell_commands_adds_completion() {
        // TODO: verify `completion` subcommand is registered.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_register_shell_commands_adds_man() {
        // TODO: verify `man` subcommand is registered.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_completion_bash_outputs_script() {
        // TODO: verify bash completion output is non-empty and starts with `#`.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_completion_zsh_outputs_script() {
        // TODO: verify zsh completion output is non-empty.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_completion_invalid_shell_exits_nonzero() {
        // TODO: verify invalid shell name exits with code 2.
        assert!(false, "not implemented");
    }
}
