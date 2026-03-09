//! Test module for test rust parser in `talkbank-tools`.
//!
//! These tests document expected behavior and regressions.

use std::error::Error;
use std::fs;

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string("/tmp/test_separators.cha")?;
    let parser = talkbank_parser::TreeSitterParser::new()?;

    match parser.parse_chat_file(&content) {
        Ok(file) => {
            let utts: Vec<_> = file.utterances().collect();
            println!("✓ Parsed successfully! {} utterances", utts.len());
            for (i, utt) in utts.iter().enumerate() {
                println!(
                    "Utterance {}: {} words",
                    i + 1,
                    utt.main.content.content.len()
                );
            }
        }
        Err(errors) => {
            println!("✗ Parse failed with {} errors:", errors.len());
            for (i, error) in errors.errors.iter().enumerate().take(5) {
                println!("  {}. [{}] {}", i + 1, error.code, error.message);
            }
        }
    }

    Ok(())
}
