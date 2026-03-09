//! Debug engn4175 functionality for this subsystem.
//!

use std::error::Error;
use std::fs;
use talkbank_model::model::Line;
use talkbank_parser::TreeSitterParser;

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let parser = TreeSitterParser::new()?;
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "testchat/good/engn4175.cha".to_string());

    let original = fs::read_to_string(path)?;

    println!("=== Parsing original file ===");
    match parser.parse_chat_file(&original) {
        Ok(chat_file) => {
            println!("✓ Original parsed successfully");

            println!("\n=== Serializing to canonical ===");
            let canonical = format!("{}", chat_file);

            // Save to file for inspection
            fs::write("/tmp/engn4175_canonical.cha", &canonical)?;
            println!("Saved canonical form to /tmp/engn4175_canonical.cha");

            // Count all main tiers
            let utterances: Vec<_> = chat_file
                .lines
                .iter()
                .enumerate()
                .filter_map(|(idx, line)| match line {
                    Line::Utterance(utt) => Some((idx + 1, utt)),
                    Line::Header { .. } => None,
                })
                .collect();
            println!("Number of main tiers: {}", utterances.len());

            // Show all main tiers
            for (i, (line_number, utt)) in utterances.iter().enumerate() {
                let main_text = format!("{}", utt.main);
                println!("\nMain tier {} (line {}):", i + 1, line_number);
                println!("{}", main_text);

                // Try parsing each
                match parser.parse_main_tier(&main_text) {
                    Ok(_) => println!("✓ Parses OK"),
                    Err(e) => println!("✗ Parse failed: {}", e),
                }
            }

            println!("\n=== Re-parsing canonical ===");
            match parser.parse_chat_file(&canonical) {
                Ok(_) => {
                    println!("✓ Canonical parsed successfully");
                }
                Err(e) => {
                    println!("✗ Canonical parse failed:");
                    println!("  {}", e);

                    // Show first line of canonical with issue
                    if let Some(first_line) = canonical.lines().next() {
                        println!("\n  First line: {}", first_line);
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ Original parse failed:");
            println!("  {}", e);
        }
    }

    Ok(())
}
