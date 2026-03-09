//! Debug alignment functionality for this subsystem.
//!

use std::error::Error;
use std::fs;
use std::io;
use talkbank_model::model::Line;
use talkbank_model::{ErrorCode, ErrorCollector};
use talkbank_parser::TreeSitterParser;

/// Prints tree.
fn print_tree(node: tree_sitter::Node, source: &str, indent: usize) {
    let prefix = "  ".repeat(indent);
    let text: String = source[node.start_byte()..node.end_byte()]
        .chars()
        .take(40)
        .collect();
    let text = text.replace('\n', "\\n").replace('\t', "\\t");
    println!(
        "{}[{}] {} \"{}...\"",
        prefix,
        node.kind(),
        if node.is_error() { "(ERROR)" } else { "" },
        text
    );
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        print_tree(child, source, indent + 1);
    }
}

/// Parses tree.
fn parse_tree(
    parser: &mut tree_sitter::Parser,
    content: &str,
) -> Result<tree_sitter::Tree, io::Error> {
    parser
        .parse(content, None)
        .ok_or_else(|| io::Error::other("Tree-sitter parse failed"))
}

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
    let content = fs::read_to_string("tests/alignment_corpus/sad_path/E411_mor_too_few_items.cha")?;

    println!("=== File Content ===");
    println!("{}", content);

    let parser = TreeSitterParser::new()?;

    // Print tree-sitter parse tree
    println!("\n=== Tree-sitter Parse Tree ===");
    let mut ts_parser = tree_sitter::Parser::new();
    ts_parser.set_language(&tree_sitter_talkbank::LANGUAGE.into())?;
    let tree = parse_tree(&mut ts_parser, &content)?;
    print_tree(tree.root_node(), &content, 0);

    // Try parse_chat_file (returns Result)
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
                        println!("  Main tier word count: {}", utt.main.content.content.len());
                        println!("  Has %mor: {}", utt.mor_tier().is_some());
                        if let Some(mor) = utt.mor_tier() {
                            println!("  %mor item count: {}", mor.items.len());
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
