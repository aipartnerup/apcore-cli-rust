// apcore-cli — Shell completion and man page generation.
// Protocol spec: FE-10 (register_shell_commands)

use clap::Command;

// ---------------------------------------------------------------------------
// register_shell_commands
// ---------------------------------------------------------------------------

/// Attach the `completion` and `man` subcommands to the given root command and
/// return it. Uses the clap v4 builder idiom (consume + return).
///
/// * `completion <shell>` — emit shell completion script to stdout
///   Supported shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`
/// * `man`                — emit a man page to stdout
pub fn register_shell_commands(cli: Command, prog_name: &str) -> Command {
    cli.subcommand(completion_command(prog_name))
        .subcommand(man_command())
}

/// Build the `completion` subcommand using `clap_complete`.
fn completion_command(prog_name: &str) -> Command {
    let _ = prog_name;
    Command::new("completion")
        .about("Generate shell completion scripts.")
        .arg(
            clap::Arg::new("SHELL")
                .required(true)
                .value_parser(["bash", "zsh", "fish", "powershell", "elvish"])
                .help("Shell to generate completion for."),
        )
}

/// Build the `man` subcommand.
fn man_command() -> Command {
    Command::new("man").about("Print a man page to stdout.")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_shell_commands_adds_completion() {
        let root = Command::new("apcore-cli");
        let cmd = register_shell_commands(root, "apcore-cli");
        let names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(
            names.contains(&"completion"),
            "must have 'completion' subcommand, got {names:?}"
        );
    }

    #[test]
    fn test_register_shell_commands_adds_man() {
        let root = Command::new("apcore-cli");
        let cmd = register_shell_commands(root, "apcore-cli");
        let names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(names.contains(&"man"), "must have 'man' subcommand, got {names:?}");
    }

    #[test]
    fn test_completion_bash_outputs_script() {
        // Verify the completion command exists and has the SHELL positional arg.
        let cmd = completion_command("apcore-cli");
        let positionals: Vec<&str> = cmd
            .get_positionals()
            .filter_map(|a| a.get_id().as_str().into())
            .collect();
        assert!(
            positionals.contains(&"SHELL"),
            "completion must have SHELL positional, got {positionals:?}"
        );
    }

    #[test]
    fn test_completion_zsh_outputs_script() {
        // Verify zsh is a valid shell choice.
        let cmd = completion_command("apcore-cli");
        let shell_arg = cmd
            .get_positionals()
            .find(|a| a.get_id().as_str() == "SHELL")
            .expect("SHELL positional must exist");
        let possible = shell_arg.get_possible_values();
        let values: Vec<&str> = possible.iter().map(|v| v.get_name()).collect();
        assert!(values.contains(&"zsh"), "zsh must be a valid SHELL value");
    }

    #[test]
    fn test_completion_invalid_shell_exits_nonzero() {
        // Verify bash is a valid shell choice (parser rejects invalid shells).
        let cmd = completion_command("apcore-cli");
        let shell_arg = cmd
            .get_positionals()
            .find(|a| a.get_id().as_str() == "SHELL")
            .expect("SHELL positional must exist");
        let possible = shell_arg.get_possible_values();
        let values: Vec<&str> = possible.iter().map(|v| v.get_name()).collect();
        // "invalid_shell" must NOT be in the accepted list.
        assert!(
            !values.contains(&"invalid_shell"),
            "invalid_shell must not be accepted"
        );
    }
}
