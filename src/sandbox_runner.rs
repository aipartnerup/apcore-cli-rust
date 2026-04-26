// apcore-cli — Internal sandbox runner entry point.
// This module is NOT part of the public API. It is invoked as a subprocess
// by the Sandbox security layer to execute modules in isolation.
// Protocol spec: SEC-04 (Sandbox)

use serde_json::Value;

/// Default extensions root when `APCORE_EXTENSIONS_ROOT` is absent.
const DEFAULT_EXTENSIONS_ROOT: &str = "./extensions";

/// Entry point for the sandboxed subprocess.
///
/// Reads `module_id` from `argv[2]` (position after `apcore-cli --internal-sandbox-runner`)
/// and `input_data` as JSON from stdin, rebuilds the apcore registry by
/// running the same filesystem discovery as the parent process (driven by
/// `APCORE_EXTENSIONS_ROOT`, which is always propagated by the parent via
/// `SANDBOX_ALLOWED_ENV_PREFIXES`), calls the executor, and writes the JSON
/// result to stdout.
///
/// Exit codes mirror the main CLI conventions (0, 1, 44, 45, …).
pub async fn run_sandbox_subprocess() -> Result<(), anyhow::Error> {
    use tokio::io::AsyncReadExt;

    let module_id = std::env::args()
        .nth(2)
        .ok_or_else(|| anyhow::anyhow!("sandbox runner: missing module_id argument"))?;

    // Read JSON input from stdin.
    let mut stdin_buf = String::new();
    tokio::io::stdin().read_to_string(&mut stdin_buf).await?;
    let input_data: Value = serde_json::from_str(&stdin_buf)?;

    // Rebuild registry by discovering the same extensions tree the parent
    // resolved at startup. Without this, executor.call below would always
    // return MODULE_NOT_FOUND because the registry is empty.
    let extensions_root = std::env::var("APCORE_EXTENSIONS_ROOT")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_EXTENSIONS_ROOT.to_string());
    let registry = apcore::Registry::new();
    let discoverer = crate::fs_discoverer::FsDiscoverer::new(&extensions_root);
    // Propagate discovery failures rather than continuing with an empty
    // registry — otherwise the parent sees a generic MODULE_NOT_FOUND
    // (exit 44) and the real cause (permission denied on extensions root,
    // YAML scan error, etc.) gets buried earlier in the stderr stream.
    registry.discover(&discoverer).await.map_err(|e| {
        anyhow::anyhow!(
            "sandbox runner: discovery failed for extensions root '{}': {}",
            extensions_root,
            e
        )
    })?;
    // Publish discovered executables so binding-style modules can resolve
    // their subprocess entry points when invoked via executor.call.
    crate::cli::set_executables(discoverer.executables_snapshot());

    // Build executor from the now-populated registry.
    let config = apcore::Config::default();
    let executor = apcore::Executor::new(std::sync::Arc::new(registry), config);
    let result = executor.call(&module_id, input_data, None, None).await?;

    // Write JSON result to stdout.
    let encoded = encode_result(&result);
    print!("{encoded}");
    Ok(())
}

/// Serialise the sandbox result for IPC.
pub fn encode_result(result: &Value) -> String {
    serde_json::to_string(result).unwrap_or_else(|_| "null".to_string())
}

/// Deserialise the sandbox result received by the parent process.
pub fn decode_result(raw: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(raw)
}

// -------------------------------------------------------------------
// Unit tests
// -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn encode_result_handles_object() {
        let v = json!({"ok": true, "n": 42});
        let s = encode_result(&v);
        // Object key order is not stable across serde_json versions; round-trip
        // through decode_result to assert semantic equality instead.
        let parsed: Value = serde_json::from_str(&s).expect("encoder must produce valid JSON");
        assert_eq!(parsed, v);
    }

    #[test]
    fn encode_result_handles_null() {
        let v = Value::Null;
        assert_eq!(encode_result(&v), "null");
    }

    #[test]
    fn encode_result_handles_array() {
        let v = json!(["a", 1, null]);
        // Array order is preserved by serde_json's encoder; check the literal.
        assert_eq!(encode_result(&v), r#"["a",1,null]"#);
        // Also assert via round-trip for documentation parity with the object case.
        let parsed: Value = serde_json::from_str(&encode_result(&v)).unwrap();
        assert_eq!(parsed, v);
    }

    #[test]
    fn decode_result_round_trips_object() {
        let original = json!({"alpha": [1, 2, 3], "beta": {"nested": "x"}});
        let encoded = encode_result(&original);
        let decoded = decode_result(&encoded).expect("valid JSON");
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_result_rejects_invalid_json() {
        let result = decode_result("{not json");
        assert!(result.is_err());
    }

    #[test]
    fn decode_result_accepts_null() {
        let decoded = decode_result("null").expect("valid JSON");
        assert!(decoded.is_null());
    }

    #[test]
    fn default_extensions_root_constant() {
        // Guard against accidental changes to the documented fallback path.
        assert_eq!(DEFAULT_EXTENSIONS_ROOT, "./extensions");
    }
}
