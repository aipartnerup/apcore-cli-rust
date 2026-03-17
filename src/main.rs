// apcore-cli — Binary entry point.
// Protocol spec: FE-01 (create_cli, main)

use apcore_cli::EXIT_CONFIG_NOT_FOUND;

/// Build the top-level clap Command.
///
/// `extensions_dir` — path to the extensions directory (overrides auto-discovery).
/// `prog_name`       — override the program name shown in help text.
///
/// # Errors
/// Exits with `EXIT_CONFIG_NOT_FOUND` (47) if `extensions_dir` is provided but
/// does not exist on the filesystem.
pub fn create_cli(extensions_dir: Option<String>, prog_name: Option<String>) -> clap::Command {
    // TODO: construct full CLI via cli::build_root_command and attach discovery,
    //       shell, and exec subcommands.
    let name = prog_name.unwrap_or_else(|| "apcore-cli".to_string());

    if let Some(ref dir) = extensions_dir {
        if !std::path::Path::new(dir).exists() {
            eprintln!("error: extensions directory not found: {dir}");
            std::process::exit(EXIT_CONFIG_NOT_FOUND);
        }
    }

    clap::Command::new(name)
        .version(env!("CARGO_PKG_VERSION"))
        .about("Command-line interface for apcore modules")
    // TODO: add subcommands and global flags
}

/// Pre-parse `--extensions-dir` from argv before clap processes the full
/// argument list, then hand off to the built CLI.
fn extract_extensions_dir() -> Option<String> {
    // TODO: walk std::env::args() to find --extensions-dir VALUE before clap
    //       consumes it (mirrors Python's _extract_extensions_dir).
    None
}

#[tokio::main]
async fn main() {
    // TODO: initialise tracing subscriber from env-filter.
    // TODO: call extract_extensions_dir(), create_cli(), and invoke the command.
    let extensions_dir = extract_extensions_dir();
    let cmd = create_cli(extensions_dir, None);
    cmd.get_matches();
}
