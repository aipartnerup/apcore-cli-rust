// apcore-cli — Discovery subcommands (list + describe).
// Protocol spec: FE-03 (register_discovery_commands)

use clap::Command;

// ---------------------------------------------------------------------------
// register_discovery_commands
// ---------------------------------------------------------------------------

/// Attach the `list` and `describe` subcommands to the given root command and
/// return it. Uses the clap v4 builder idiom (consume + return).
///
/// * `list`     — print all registered module IDs, optionally filtered by tag
/// * `describe` — print full schema + metadata for a single module
///
/// Both commands respect the `--format` flag (`table` | `json`).
pub fn register_discovery_commands(cli: Command) -> Command {
    cli.subcommand(list_command())
        .subcommand(describe_command())
}

/// Build the `list` subcommand.
fn list_command() -> Command {
    Command::new("list")
        .about("List all registered module IDs.")
        .arg(
            clap::Arg::new("tag")
                .long("tag")
                .value_name("TAG")
                .help("Filter by tag."),
        )
        .arg(
            clap::Arg::new("format")
                .long("format")
                .value_parser(["json", "table"])
                .help("Output format."),
        )
}

/// Build the `describe` subcommand.
fn describe_command() -> Command {
    Command::new("describe")
        .about("Print schema and metadata for a module.")
        .arg(
            clap::Arg::new("MODULE_ID")
                .required(true)
                .help("Module identifier (e.g. math.add)."),
        )
        .arg(
            clap::Arg::new("format")
                .long("format")
                .value_parser(["json", "table"])
                .help("Output format."),
        )
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_discovery_commands_adds_list() {
        let root = Command::new("apcore-cli");
        let cmd = register_discovery_commands(root);
        let names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(names.contains(&"list"), "must have 'list' subcommand, got {names:?}");
    }

    #[test]
    fn test_register_discovery_commands_adds_describe() {
        let root = Command::new("apcore-cli");
        let cmd = register_discovery_commands(root);
        let names: Vec<&str> = cmd.get_subcommands().map(|c| c.get_name()).collect();
        assert!(
            names.contains(&"describe"),
            "must have 'describe' subcommand, got {names:?}"
        );
    }

    #[test]
    fn test_list_command_with_tag_filter() {
        let cmd = list_command();
        let arg_names: Vec<&str> = cmd.get_opts().filter_map(|a| a.get_long()).collect();
        assert!(arg_names.contains(&"tag"), "list must have --tag flag");
    }

    #[test]
    fn test_describe_command_module_not_found() {
        // Verify MODULE_ID positional arg is present.
        let cmd = describe_command();
        let positionals: Vec<&str> = cmd
            .get_positionals()
            .filter_map(|a| a.get_id().as_str().into())
            .collect();
        assert!(
            positionals.contains(&"MODULE_ID"),
            "describe must have MODULE_ID positional, got {positionals:?}"
        );
    }
}
