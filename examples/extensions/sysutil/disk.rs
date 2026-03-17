// apcore-cli example — sysutil.disk
// Demonstrates a disk usage query module.
//
// Run via the apcore-cli binary:
//   APCORE_EXTENSIONS_ROOT=examples/extensions apcore-cli sysutil.disk --path /

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// ---------------------------------------------------------------------------
// Input / Output schemas
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Input {
    /// Filesystem path to check (default: "/").
    #[serde(default = "default_path")]
    pub path: String,
}

fn default_path() -> String {
    "/".to_string()
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Output {
    /// The queried path.
    pub path: String,
    /// Total capacity (human-readable, e.g. "256.0 GB").
    pub total: String,
    /// Used space (human-readable).
    pub used: String,
    /// Free space (human-readable).
    pub free: String,
    /// Percentage of used space (0.0 – 100.0).
    pub percent_used: f64,
}

// ---------------------------------------------------------------------------
// Module implementation
// ---------------------------------------------------------------------------

/// Get disk usage statistics for a given filesystem path.
pub struct SysutilDisk;

impl SysutilDisk {
    pub const MODULE_ID: &'static str = "sysutil.disk";
    pub const DESCRIPTION: &'static str =
        "Get disk usage statistics for a given filesystem path";

    pub fn execute(input: Input) -> Output {
        // TODO: use statvfs (nix crate) or sysinfo crate for cross-platform support.
        // For now return placeholder values to keep the example compilable.
        Output {
            path: input.path.clone(),
            total: "N/A (not implemented)".to_string(),
            used: "N/A".to_string(),
            free: "N/A".to_string(),
            percent_used: 0.0,
        }
    }
}

/// Format a byte count as a human-readable string (B, KB, MB, GB, TB).
fn format_bytes(mut bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB", "TB", "PB"];
    let mut value = bytes as f64;
    let mut unit = units[0];
    for u in &units[1..] {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = u;
    }
    format!("{value:.1} {unit}")
}

fn main() {
    let input = Input { path: "/".to_string() };
    let output = SysutilDisk::execute(input);
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
