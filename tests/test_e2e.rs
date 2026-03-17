// apcore-cli — End-to-end CLI invocation tests.
// These tests invoke the binary-level CLI and check exit codes + stdout.

mod common;

/// Helper: invoke the CLI command parser with given args and capture output.
///
/// TODO: replace with a proper test harness that captures stdin/stdout once
/// the CLI is fully implemented.
fn invoke_cli(args: &[&str]) -> (i32, String) {
    // TODO: spawn apcore-cli binary, capture output and exit code.
    todo!("invoke_cli: args={args:?}")
}

#[test]
fn test_e2e_help_flag_exits_0() {
    // `apcore-cli --help` must exit 0.
    // TODO: implement with invoke_cli or assert_cmd crate.
    assert!(false, "not implemented");
}

#[test]
fn test_e2e_version_flag() {
    // `apcore-cli --version` must print the version string.
    assert!(false, "not implemented");
}

#[test]
fn test_e2e_list_command() {
    // `apcore-cli list --format json` must exit 0 and output a JSON array.
    assert!(false, "not implemented");
}

#[test]
fn test_e2e_describe_command() {
    // `apcore-cli describe math.add --format json` must exit 0.
    assert!(false, "not implemented");
}

#[test]
fn test_e2e_execute_math_add() {
    // `apcore-cli math.add --a 3 --b 4` must exit 0 and output {"sum": 7}.
    assert!(false, "not implemented");
}

#[test]
fn test_e2e_stdin_piping() {
    // `echo '{"a":1,"b":2}' | apcore-cli math.add --input -` must return sum=3.
    assert!(false, "not implemented");
}

#[test]
fn test_e2e_unknown_module_exits_44() {
    // Invoking a non-existent module must exit 44.
    assert!(false, "not implemented");
}

#[test]
fn test_e2e_invalid_input_exits_2() {
    // Missing required flag must exit 2.
    assert!(false, "not implemented");
}

#[test]
fn test_e2e_completion_bash() {
    // `apcore-cli completion bash` must exit 0 and emit non-empty output.
    assert!(false, "not implemented");
}
