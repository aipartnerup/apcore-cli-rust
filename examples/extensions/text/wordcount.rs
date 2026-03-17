// apcore-cli example — text.wordcount
// Demonstrates a text statistics module.
//
// Run via the apcore-cli binary:
//   APCORE_EXTENSIONS_ROOT=examples/extensions apcore-cli text.wordcount --text "hello world from apcore"

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

// ---------------------------------------------------------------------------
// Input / Output schemas
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct Input {
    /// Input text to analyse.
    pub text: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct Output {
    /// Total number of Unicode characters.
    pub characters: usize,
    /// Number of whitespace-delimited words.
    pub words: usize,
    /// Number of newline-delimited lines.
    pub lines: usize,
}

// ---------------------------------------------------------------------------
// Module implementation
// ---------------------------------------------------------------------------

/// Count words, characters, and lines in a text string.
pub struct TextWordCount;

impl TextWordCount {
    pub const MODULE_ID: &'static str = "text.wordcount";
    pub const DESCRIPTION: &'static str =
        "Count words, characters, and lines in a text string";

    pub fn execute(input: Input) -> Output {
        let text = &input.text;
        Output {
            characters: text.chars().count(),
            words: text.split_whitespace().count(),
            lines: text.lines().count(),
        }
    }
}

fn main() {
    let input = Input {
        text: "hello world from apcore".to_string(),
    };
    let output = TextWordCount::execute(input);
    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
