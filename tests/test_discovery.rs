// apcore-cli — Integration tests for discovery commands (list + describe).
// Protocol spec: FE-03

mod common;

use apcore_cli::discovery::register_discovery_commands;
use clap::Command;

#[test]
fn test_register_discovery_adds_list_subcommand() {
    // After register_discovery_commands, the root must have a `list` subcommand.
    // TODO: create a root Command, call register_discovery_commands,
    //       verify list subcommand exists.
    assert!(false, "not implemented");
}

#[test]
fn test_register_discovery_adds_describe_subcommand() {
    // After register_discovery_commands, the root must have a `describe` subcommand.
    assert!(false, "not implemented");
}

#[test]
fn test_list_command_json_format() {
    // `list --format json` must output valid JSON array.
    assert!(false, "not implemented");
}

#[test]
fn test_list_command_table_format() {
    // `list --format table` must output a formatted table.
    assert!(false, "not implemented");
}

#[test]
fn test_list_command_tag_filter() {
    // `list --tag math` must return only modules tagged "math".
    assert!(false, "not implemented");
}

#[test]
fn test_describe_command_known_module() {
    // `describe math.add` must output the module's schema and description.
    assert!(false, "not implemented");
}

#[test]
fn test_describe_command_unknown_module_exits_44() {
    // `describe nonexistent.module` must exit with code 44.
    assert!(false, "not implemented");
}
