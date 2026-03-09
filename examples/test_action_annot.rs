//! Test module for test action annot in `talkbank-tools`.
//!
//! These tests document expected behavior and regressions.

use std::error::Error;
use std::fs;

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string("/tmp/test_para.cha")?;
    let parser = talkbank_parser::TreeSitterParser::new()?;

    match parser.parse_chat_file(&content) {
        Ok(file) => {
            let utts: Vec<_> = file.utterances().collect();
            println!("Parsed {} utterances", utts.len());
            for (i, utt) in utts.iter().enumerate() {
                println!(
                    "
Utterance {}:",
                    i + 1
                );
                for (j, content_item) in utt.main.content.content.iter().enumerate() {
                    println!("  Content {}: {:?}", j, content_item);
                }
            }
        }
        Err(errors) => {
            println!("Parse failed with {} errors", errors.len());
            for err in errors.errors.iter().take(5) {
                println!("  [{}] {}", err.code, err.message);
            }
        }
    }

    Ok(())
}
