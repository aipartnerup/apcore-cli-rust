// apcore-cli example — sysutil.info
// Demonstrates a system information module.
//
// Run via the apcore-cli binary:
//   APCORE_EXTENSIONS_ROOT=examples/extensions apcore-cli sysutil.info

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// ---------------------------------------------------------------------------
// Input / Output schemas
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Input;

#[derive(Debug, Serialize, JsonSchema)]
pub struct Output {
    /// Operating system name.
    pub os: String,
    /// OS release version.
    pub os_version: String,
    /// CPU architecture.
    pub architecture: String,
    /// Machine hostname.
    pub hostname: String,
    /// Current working directory.
    pub cwd: String,
    /// Value of the USER/USERNAME environment variable.
    pub user: String,
    /// Rust compiler version (compile-time constant).
    pub rust_version: String,
}

// ---------------------------------------------------------------------------
// Module implementation
// ---------------------------------------------------------------------------

/// Get basic system information (OS, architecture, hostname).
pub struct SysutilInfo;

impl SysutilInfo {
    pub const MODULE_ID: &'static str = "sysutil.info";
    pub const DESCRIPTION: &'static str =
        "Get basic system information (OS, architecture, hostname)";

    pub fn execute(_input: Input) -> Output {
        Output {
            os: std::env::consts::OS.to_string(),
            os_version: "unknown".to_string(), // requires os_info crate for detail
            architecture: std::env::consts::ARCH.to_string(),
            hostname: hostname(),
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            user: std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "unknown".to_string()),
            rust_version: env!("CARGO_PKG_RUST_VERSION", "unknown").to_string(),
        }
    }
}

fn hostname() -> String {
    // Use gethostname via std when available; fall back to env.
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn main() {
    let output = SysutilInfo::execute(Input);
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
