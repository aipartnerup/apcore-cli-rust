// apcore-cli — Integration tests for shell completion and man page commands.
// Protocol spec: FE-10

mod common;

use apcore_cli::shell::register_shell_commands;
use clap::Command;

#[test]
fn test_register_shell_commands_adds_completion() {
    // After register_shell_commands, root must have a `completion` subcommand.
    // TODO: construct root Command, call register_shell_commands, verify.
    assert!(false, "not implemented");
}

#[test]
fn test_register_shell_commands_adds_man() {
    // After register_shell_commands, root must have a `man` subcommand.
    assert!(false, "not implemented");
}

#[test]
fn test_completion_bash_outputs_nonempty() {
    // `completion bash` must produce non-empty output.
    assert!(false, "not implemented");
}

#[test]
fn test_completion_zsh_outputs_nonempty() {
    // `completion zsh` must produce non-empty output.
    assert!(false, "not implemented");
}

#[test]
fn test_completion_fish_outputs_nonempty() {
    // `completion fish` must produce non-empty output.
    assert!(false, "not implemented");
}

#[test]
fn test_completion_invalid_shell_exits_nonzero() {
    // An invalid shell name must exit with code 2.
    assert!(false, "not implemented");
}

#[test]
fn test_man_command_outputs_nonempty() {
    // `man` must produce non-empty output.
    assert!(false, "not implemented");
}
