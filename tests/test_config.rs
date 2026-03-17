// apcore-cli — Integration tests for ConfigResolver.
// Protocol spec: FE-07

mod common;

use std::collections::HashMap;
use std::path::PathBuf;

use apcore_cli::config::ConfigResolver;
use tempfile::tempdir;

#[test]
fn test_config_resolver_instantiation() {
    let resolver = ConfigResolver::new(None, None);
    assert!(!resolver.defaults.is_empty());
}

#[test]
fn test_config_resolver_with_cli_flags() {
    let mut flags = HashMap::new();
    flags.insert("--extensions-dir".to_string(), Some("/cli".to_string()));
    let resolver = ConfigResolver::new(Some(flags.clone()), None);
    assert_eq!(resolver._cli_flags, flags);
}

#[test]
fn test_defaults_contains_expected_keys() {
    let resolver = ConfigResolver::new(None, None);
    for key in [
        "extensions.root",
        "logging.level",
        "sandbox.enabled",
        "cli.stdin_buffer_limit",
    ] {
        assert!(resolver.defaults.contains_key(key), "missing default: {key}");
    }
}

#[test]
fn test_resolve_tier1_cli_flag_wins() {
    // CLI flag must beat env var, config file, and default.
    // TODO: set env var + write config file, verify CLI flag value wins.
    assert!(false, "not implemented");
}

#[test]
fn test_resolve_tier2_env_var_wins() {
    // Env var must beat config file and default.
    // TODO: write config file, set env var, verify env var wins.
    assert!(false, "not implemented");
}

#[test]
fn test_resolve_tier3_config_file_wins() {
    // Config file must beat default when no cli flag or env var is set.
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("apcore.yaml");
    std::fs::write(
        &config_path,
        "extensions:\n  root: /config-path\n",
    )
    .unwrap();
    // TODO: clean env, create resolver with config_path, assert /config-path.
    assert!(false, "not implemented");
}

#[test]
fn test_resolve_tier4_default_wins() {
    // Default must be returned when no other tier provides a value.
    common::strip_apcore_env_vars();
    let resolver = ConfigResolver::new(None, Some(PathBuf::from("/nonexistent/apcore.yaml")));
    let result = resolver.resolve("extensions.root", None, None);
    // TODO: assert result == Some("./extensions".to_string())
    assert!(false, "not implemented");
}

#[test]
fn test_resolve_cli_flag_none_skips_tier1() {
    // A cli_flag entry with value None must be skipped (fall through to tier 2).
    assert!(false, "not implemented");
}

#[test]
fn test_resolve_env_var_empty_string_skips_tier2() {
    // An empty-string env var must be treated as unset (fall through to tier 3).
    assert!(false, "not implemented");
}

#[test]
fn test_resolve_unknown_key_returns_none() {
    common::strip_apcore_env_vars();
    let resolver = ConfigResolver::new(None, None);
    let result = resolver.resolve("nonexistent.key", None, None);
    assert!(result.is_none());
}

#[test]
fn test_load_config_file_valid_yaml() {
    let dir = tempdir().unwrap();
    let config_path = dir.path().join("apcore.yaml");
    std::fs::write(
        &config_path,
        "extensions:\n  root: /custom/path\nlogging:\n  level: DEBUG\n",
    )
    .unwrap();
    let resolver = ConfigResolver::new(None, Some(config_path));
    // TODO: assert _config_file contains extensions.root and logging.level.
    assert!(false, "not implemented");
}

#[test]
fn test_load_config_file_not_found() {
    let resolver = ConfigResolver::new(None, Some(PathBuf::from("/nonexistent/apcore.yaml")));
    assert!(resolver._config_file.is_none());
}

#[test]
fn test_flatten_dict_nested() {
    // TODO: {"extensions": {"root": "/path"}} → {"extensions.root": "/path"}
    assert!(false, "not implemented");
}

#[test]
fn test_flatten_dict_deeply_nested() {
    // TODO: {"a": {"b": {"c": "deep_value"}}} → {"a.b.c": "deep_value"}
    assert!(false, "not implemented");
}
