// apcore-cli example — text.reverse
// Demonstrates a string reversal module.
//
// Run via the apcore-cli binary:
//   APCORE_EXTENSIONS_ROOT=examples/extensions apcore-cli text.reverse --text "apcore-cli"

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// ---------------------------------------------------------------------------
// Input / Output schemas
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Input {
    /// Input string to reverse.
    pub text: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Output {
    /// The reversed string.
    pub result: String,
}

// ---------------------------------------------------------------------------
// Module implementation
// ---------------------------------------------------------------------------

/// Reverse a string character by character.
pub struct TextReverse;

impl TextReverse {
    pub const MODULE_ID: &'static str = "text.reverse";
    pub const DESCRIPTION: &'static str = "Reverse a string character by character";

    pub fn execute(input: Input) -> Output {
        Output {
            result: input.text.chars().rev().collect(),
        }
    }
}

fn main() {
    let input = Input { text: "apcore-cli".to_string() };
    let output = TextReverse::execute(input);
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
