//! Compare parsers functionality for this subsystem.
//!

// Run with: cargo run --example compare_parsers -- corpus/reference/amp-subword.cha
use std::env;
use std::error::Error;
use std::fmt;
use talkbank_direct_parser::DirectParser;
use talkbank_model::model::{SemanticDiff, SemanticEq};
use talkbank_parser::TreeSitterParser;

/// Type representing UsageError.
#[derive(Debug)]
struct UsageError;

impl fmt::Display for UsageError {
    /// Runs fmt.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Usage: compare_parsers <file.cha>")
    }
}

impl Error for UsageError {}

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or(UsageError)?;
    let content = std::fs::read_to_string(&path)?;

    println!("Comparing parsers on: {}\n", path);

    let ts = TreeSitterParser::new()?;
    let direct = DirectParser::new()?;

    let ts_file = ts.parse_chat_file(&content)?;
    let direct_file = direct.parse_chat_file(&content)?;

    if ts_file.semantic_eq(&direct_file) {
        println!("✓ Files are semantically equivalent");
    } else {
        println!("✗ Semantic mismatch detected!\n");

        // Generate semantic diff report
        let report = ts_file.semantic_diff(&direct_file);

        // Show tree diff
        println!(
            "{}",
            report.render_tree_diff(
                None, // max_depth: unlimited
                true, // show_spans: yes
                talkbank_model::model::RenderMode::Full,
            )
        );

        // Write JSONs to temp files for deep inspection if needed
        let ts_json = serde_json::to_string_pretty(&ts_file)?;
        let direct_json = serde_json::to_string_pretty(&direct_file)?;
        std::fs::write("/tmp/ts.json", &ts_json)?;
        std::fs::write("/tmp/direct.json", &direct_json)?;

        println!("\nFull JSON output written to /tmp/ts.json and /tmp/direct.json");
        println!("Run: diff /tmp/ts.json /tmp/direct.json");
    }

    Ok(())
}
