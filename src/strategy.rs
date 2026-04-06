// apcore-cli -- Pipeline strategy commands (FE-11).
// Provides describe-pipeline subcommand showing execution pipeline steps.

use clap::{Arg, Command};
use serde_json::Value;
use std::io::IsTerminal;

// ---------------------------------------------------------------------------
// Preset strategy step definitions
// ---------------------------------------------------------------------------

/// Known preset pipeline strategies and their steps.
fn preset_steps(strategy: &str) -> Vec<&'static str> {
    match strategy {
        "standard" => vec![
            "context_creation",
            "call_chain_guard",
            "module_lookup",
            "acl_check",
            "approval_gate",
            "middleware_before",
            "input_validation",
            "execute",
            "output_validation",
            "middleware_after",
            "return_result",
        ],
        "internal" => vec![
            "context_creation",
            "call_chain_guard",
            "module_lookup",
            "middleware_before",
            "input_validation",
            "execute",
            "output_validation",
            "middleware_after",
            "return_result",
        ],
        "testing" => vec![
            "context_creation",
            "module_lookup",
            "middleware_before",
            "input_validation",
            "execute",
            "output_validation",
            "middleware_after",
            "return_result",
        ],
        "performance" => vec![
            "context_creation",
            "call_chain_guard",
            "module_lookup",
            "acl_check",
            "approval_gate",
            "input_validation",
            "execute",
            "output_validation",
            "return_result",
        ],
        "minimal" => vec![
            "context_creation",
            "module_lookup",
            "execute",
            "return_result",
        ],
        _ => vec![],
    }
}

// ---------------------------------------------------------------------------
// Command builder
// ---------------------------------------------------------------------------

/// Build the `describe-pipeline` clap subcommand.
pub fn describe_pipeline_command() -> Command {
    Command::new("describe-pipeline")
        .about("Show the execution pipeline steps for a strategy")
        .arg(
            Arg::new("strategy")
                .long("strategy")
                .value_parser(["standard", "internal", "testing", "performance", "minimal"])
                .default_value("standard")
                .value_name("STRATEGY")
                .help("Strategy to describe (default: standard)."),
        )
        .arg(
            Arg::new("format")
                .long("format")
                .value_parser(["table", "json"])
                .value_name("FORMAT")
                .help("Output format."),
        )
}

/// Register the describe-pipeline subcommand on the root command.
pub fn register_pipeline_command(cli: Command) -> Command {
    cli.subcommand(describe_pipeline_command())
}

// ---------------------------------------------------------------------------
// Dispatch
// ---------------------------------------------------------------------------

/// Dispatch the `describe-pipeline` subcommand.
pub fn dispatch_describe_pipeline(matches: &clap::ArgMatches) {
    let strategy = matches
        .get_one::<String>("strategy")
        .map(|s| s.as_str())
        .unwrap_or("standard");
    let format = matches.get_one::<String>("format").map(|s| s.as_str());
    let fmt = crate::output::resolve_format(format);

    let steps = preset_steps(strategy);

    // Step metadata: which steps are pure and which are non-removable.
    let pure_steps = [
        "context_creation",
        "call_chain_guard",
        "module_lookup",
        "acl_check",
        "input_validation",
    ];
    let non_removable = [
        "context_creation",
        "module_lookup",
        "execute",
        "return_result",
    ];

    if fmt == "json" || !std::io::stdout().is_terminal() {
        let steps_json: Vec<Value> = steps
            .iter()
            .enumerate()
            .map(|(i, s)| {
                serde_json::json!({
                    "index": i + 1,
                    "name": s,
                    "pure": pure_steps.contains(s),
                    "removable": !non_removable.contains(s),
                })
            })
            .collect();
        let payload = serde_json::json!({
            "strategy": strategy,
            "step_count": steps.len(),
            "steps": steps_json,
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
        );
    } else {
        println!("Pipeline: {strategy} ({} steps)\n", steps.len());
        println!("  #    Step                         Pure   Removable   Timeout");
        println!("  ---- ---------------------------- ------ ----------- --------");
        for (i, s) in steps.iter().enumerate() {
            let pure = if pure_steps.contains(s) { "yes" } else { "no" };
            let removable = if non_removable.contains(s) {
                "no"
            } else {
                "yes"
            };
            println!("  {:<4} {:<28} {:<6} {:<11} --", i + 1, s, pure, removable);
        }
    }

    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_steps_standard() {
        let steps = preset_steps("standard");
        assert_eq!(steps.len(), 11);
        assert_eq!(steps[0], "context_creation");
        assert_eq!(steps[7], "execute");
    }

    #[test]
    fn test_preset_steps_internal() {
        let steps = preset_steps("internal");
        assert_eq!(steps.len(), 9);
        assert!(!steps.contains(&"acl_check"));
    }

    #[test]
    fn test_preset_steps_testing() {
        let steps = preset_steps("testing");
        assert_eq!(steps.len(), 8);
        assert!(!steps.contains(&"call_chain_guard"));
    }

    #[test]
    fn test_preset_steps_performance() {
        let steps = preset_steps("performance");
        assert_eq!(steps.len(), 9);
        assert!(!steps.contains(&"middleware_before"));
    }

    #[test]
    fn test_preset_steps_unknown() {
        let steps = preset_steps("unknown");
        assert!(steps.is_empty());
    }

    #[test]
    fn test_describe_pipeline_command_builder() {
        let cmd = describe_pipeline_command();
        assert_eq!(cmd.get_name(), "describe-pipeline");
        let opts: Vec<&str> = cmd.get_opts().filter_map(|a| a.get_long()).collect();
        assert!(opts.contains(&"strategy"));
        assert!(opts.contains(&"format"));
    }

    #[test]
    fn test_register_pipeline_command() {
        let root = Command::new("test");
        let root = register_pipeline_command(root);
        let subs: Vec<&str> = root.get_subcommands().map(|c| c.get_name()).collect();
        assert!(subs.contains(&"describe-pipeline"));
    }
}
