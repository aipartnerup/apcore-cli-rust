// apcore-cli — Discovery subcommands (list + describe).
// Protocol spec: FE-03 (register_discovery_commands)

use clap::Command;

// ---------------------------------------------------------------------------
// register_discovery_commands
// ---------------------------------------------------------------------------

/// Attach the `list` and `describe` subcommands to the given root command.
///
/// * `list`     — print all registered module IDs, optionally filtered by tag
/// * `describe` — print full schema + metadata for a single module
///
/// Both commands respect the `--format` flag (`table` | `json`).
pub fn register_discovery_commands(
    cli: &mut Command,
    // registry: std::sync::Arc<dyn apcore::Registry>,
) {
    // TODO: build list_command() and describe_command(), attach to cli.
    let _ = cli;
    todo!("register_discovery_commands")
}

/// Build the `list` subcommand.
fn list_command(/* registry: std::sync::Arc<dyn apcore::Registry> */) -> Command {
    // TODO: add --tag, --format flags; implement callback.
    todo!("list_command")
}

/// Build the `describe` subcommand.
fn describe_command(/* registry: std::sync::Arc<dyn apcore::Registry> */) -> Command {
    // TODO: add MODULE_ID positional arg and --format flag; implement callback.
    todo!("describe_command")
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_discovery_commands_adds_list() {
        // TODO: verify `list` subcommand is added to the root command.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_register_discovery_commands_adds_describe() {
        // TODO: verify `describe` subcommand is added to the root command.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_list_command_with_tag_filter() {
        // TODO: verify --tag flag filters module list.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_describe_command_module_not_found() {
        // TODO: verify exit code 44 when module does not exist.
        assert!(false, "not implemented");
    }
}
