//! Debug happy path functionality for this subsystem.
//!

use std::error::Error;
use std::fs;
use talkbank_model::model::Line;
use talkbank_model::{ErrorCode, ErrorCollector};
use talkbank_parser::TreeSitterParser;

/// Returns whether temporal error.
fn is_temporal_error(code: ErrorCode) -> bool {
    matches!(
        code,
        ErrorCode::UnexpectedTierNode
            | ErrorCode::TierBeginTimeNotMonotonic
            | ErrorCode::InvalidMorphologyFormat
            | ErrorCode::UnexpectedMorphologyNode
            | ErrorCode::SpeakerSelfOverlap
            | ErrorCode::MorCountMismatchTooFew
            | ErrorCode::MorCountMismatchTooMany
            | ErrorCode::MalformedGrammarRelation
            | ErrorCode::InvalidGrammarIndex
            | ErrorCode::UnexpectedGrammarNode
            | ErrorCode::GraInvalidWordIndex
            | ErrorCode::GraInvalidHeadIndex
            | ErrorCode::PhoCountMismatchTooFew
            | ErrorCode::PhoCountMismatchTooMany
            | ErrorCode::SinCountMismatchTooFew
            | ErrorCode::SinCountMismatchTooMany
            | ErrorCode::MorGraCountMismatch
    )
}

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let content = fs::read_to_string("tests/alignment_corpus/happy_path/main_mor_aligned.cha")?;

    println!("=== File Content ===");
    println!("{}", content);

    let parser = TreeSitterParser::new()?;

    match parser.parse_chat_file(&content) {
        Ok(mut chat_file) => {
            println!("\n=== Parse Successful ===");
            println!("Lines: {}", chat_file.lines.len());

            // Print utterances
            for (i, line) in chat_file.lines.iter().enumerate() {
                match line {
                    Line::Utterance(utt) => {
                        println!(
                            "
Utterance {}:",
                            i
                        );
                        println!("  Main tier:");
                        let tier_content = &utt.main.content;
                        println!("    Content items: {}", tier_content.content.len());
                        for (j, item) in tier_content.content.iter().enumerate() {
                            println!("      {}: {:?}", j, item);
                        }
                        println!("    Terminator: {:?}", tier_content.terminator);

                        if let Some(mor) = utt.mor_tier() {
                            println!("  %mor tier:");
                            println!("    Items: {}", mor.items.len());
                            for (j, item) in mor.items.iter().enumerate() {
                                println!("      {}: {:?}", j, item);
                            }
                        }
                    }
                    Line::Header { header, .. } => {
                        println!("Header {}: {:?}", i, header);
                    }
                }
            }

            let error_sink = ErrorCollector::new();
            chat_file.validate_with_alignment(&error_sink, None);
            let errors = error_sink.into_vec();

            println!("\n=== All Validation Errors ===");
            for error in &errors {
                println!("[{}] {}", error.code.as_str(), error.message);
            }

            let alignment_errors: Vec<_> = errors
                .iter()
                .filter(|e| is_temporal_error(e.code))
                .collect();

            println!("\n=== Alignment Errors (E7xx) ===");
            println!("Count: {}", alignment_errors.len());
            for error in &alignment_errors {
                println!("[{}] {}", error.code.as_str(), error.message);
            }
        }
        Err(errors) => {
            println!("\n=== Parse Errors ===");
            for error in &errors.errors {
                println!(
                    "[{}] {}: {}",
                    error.code.as_str(),
                    error.severity,
                    error.message
                );
            }
        }
    }

    Ok(())
}
