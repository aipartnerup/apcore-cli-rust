// apcore-cli — Configuration resolver.
// Protocol spec: FE-07 (ConfigResolver, 4-tier precedence)

use std::collections::HashMap;
use std::path::PathBuf;

use tracing::warn;

// ---------------------------------------------------------------------------
// ConfigResolver
// ---------------------------------------------------------------------------

/// Resolved configuration following 4-tier precedence:
///
/// 1. CLI flags   — highest priority
/// 2. Environment variables
/// 3. Config file (YAML, dot-flattened keys)
/// 4. Built-in defaults — lowest priority
pub struct ConfigResolver {
    /// CLI flags map (flag name → value or None if not provided).
    pub _cli_flags: HashMap<String, Option<String>>,

    /// Flattened key → value map loaded from the config file.
    /// `None` if the file was not found or could not be parsed.
    pub _config_file: Option<HashMap<String, String>>,

    /// Path to the config file that was loaded (or attempted).
    config_path: Option<PathBuf>,

    /// Built-in default values.
    pub defaults: HashMap<&'static str, &'static str>,
}

impl ConfigResolver {
    /// Default configuration values.
    pub const DEFAULTS: &'static [(&'static str, &'static str)] = &[
        ("extensions.root", "./extensions"),
        ("logging.level", "WARNING"),
        ("sandbox.enabled", "false"),
        ("cli.stdin_buffer_limit", "10485760"),
    ];

    /// Create a new `ConfigResolver`.
    ///
    /// # Arguments
    /// * `cli_flags`   — CLI flag overrides (e.g. `--extensions-dir → /path`)
    /// * `config_path` — Optional explicit path to `apcore.yaml`
    pub fn new(
        cli_flags: Option<HashMap<String, Option<String>>>,
        config_path: Option<PathBuf>,
    ) -> Self {
        let defaults = Self::DEFAULTS.iter().copied().collect();
        let config_file = config_path.as_ref().and_then(|p| Self::load_config_file(p));

        Self {
            _cli_flags: cli_flags.unwrap_or_default(),
            _config_file: config_file,
            config_path,
            defaults,
        }
    }

    /// Resolve a configuration value using 4-tier precedence.
    ///
    /// # Arguments
    /// * `key`       — dot-separated config key (e.g. `"extensions.root"`)
    /// * `cli_flag`  — optional CLI flag name to check in `_cli_flags`
    /// * `env_var`   — optional environment variable name
    ///
    /// Returns `None` when the key is not present in any tier.
    pub fn resolve(
        &self,
        key: &str,
        cli_flag: Option<&str>,
        env_var: Option<&str>,
    ) -> Option<String> {
        // TODO: implement 4-tier resolution:
        //   1. cli_flag present and non-None in _cli_flags
        //   2. env_var present and non-empty in std::env
        //   3. key present in _config_file
        //   4. key present in defaults
        let _ = (key, cli_flag, env_var);
        todo!("ConfigResolver::resolve")
    }

    /// Load and flatten a YAML config file into dot-notation keys.
    ///
    /// Returns `None` if the file does not exist or cannot be parsed.
    fn load_config_file(path: &PathBuf) -> Option<HashMap<String, String>> {
        // TODO: read file, parse YAML, call flatten_dict.
        //       Log a warning on malformed YAML (do NOT panic).
        let _ = path;
        todo!("ConfigResolver::load_config_file")
    }

    /// Recursively flatten a nested map into dot-separated keys.
    ///
    /// Example: `{"extensions": {"root": "/path"}}` → `{"extensions.root": "/path"}`
    pub fn flatten_dict(&self, map: serde_json::Value) -> HashMap<String, String> {
        // TODO: implement recursive flattening.
        let _ = map;
        todo!("ConfigResolver::flatten_dict")
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_resolver_instantiation() {
        let resolver = ConfigResolver::new(None, None);
        assert!(!resolver.defaults.is_empty());
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
        // TODO: verify CLI flag takes precedence over env and file.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_resolve_tier2_env_var_wins() {
        // TODO: verify env var takes precedence over file and default.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_resolve_tier3_config_file_wins() {
        // TODO: verify config file value takes precedence over default.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_resolve_tier4_default_wins() {
        // TODO: verify default is returned when no other tier matches.
        assert!(false, "not implemented");
    }

    #[test]
    fn test_flatten_dict_nested() {
        // TODO: {"extensions": {"root": "/path"}} → {"extensions.root": "/path"}
        assert!(false, "not implemented");
    }

    #[test]
    fn test_flatten_dict_deeply_nested() {
        // TODO: {"a": {"b": {"c": "v"}}} → {"a.b.c": "v"}
        assert!(false, "not implemented");
    }
}
