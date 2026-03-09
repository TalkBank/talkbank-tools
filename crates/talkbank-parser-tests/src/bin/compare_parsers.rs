//! Compare TreeSitterParser and DirectParser output for a specific file.
//!
//! Usage:
//! ```bash
//! cargo run -p talkbank-parser-tests --bin compare_parsers -- path/to/file.cha
//! ```
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use std::env;
use std::fs;
use std::path::PathBuf;
use talkbank_direct_parser::DirectParser;
use talkbank_model::ErrorCollector;
use talkbank_model::model::SemanticEq;
use talkbank_model::{ChatParser, ParseOutcome};
use talkbank_parser::TreeSitterParser;

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file.cha>", args[0]);
        eprintln!("Example: {} corpus/reference/dep-tiers.cha", args[0]);
        std::process::exit(1);
    }

    let path = PathBuf::from(&args[1]);
    let input = fs::read_to_string(&path)?;

    println!("=== Comparing parsers for: {} ===\n", path.display());

    // Parse with TreeSitterParser
    let ts = TreeSitterParser::new()?;
    let ts_errors = ErrorCollector::new();
    let ts_result = ChatParser::parse_chat_file(&ts, &input, 0, &ts_errors);

    // Parse with DirectParser
    let direct = DirectParser::new()?;
    let direct_errors = ErrorCollector::new();
    let direct_result = ChatParser::parse_chat_file(&direct, &input, 0, &direct_errors);

    match (ts_result, direct_result) {
        (ParseOutcome::Parsed(ts_file), ParseOutcome::Parsed(direct_file)) => {
            println!("✓ Both parsers succeeded\n");

            // Check semantic equivalence
            if ts_file.semantic_eq(&direct_file) {
                println!("✓ SEMANTICALLY EQUIVALENT");
            } else {
                println!("✗ SEMANTIC MISMATCH\n");
                let ts_errs = ts_errors.to_vec();
                let direct_errs = direct_errors.to_vec();
                if !ts_errs.is_empty() || !direct_errs.is_empty() {
                    println!("TreeSitter errors ({}):", ts_errs.len());
                    for err in ts_errs {
                        println!("  - {}", err.message);
                    }
                    println!("Direct errors ({}):", direct_errs.len());
                    for err in direct_errs {
                        println!("  - {}", err.message);
                    }
                    println!();
                }

                // Serialize both to JSON for comparison
                let ts_json = match serde_json::to_string_pretty(&ts_file) {
                    Ok(json) => json,
                    Err(err) => {
                        eprintln!("TreeSitter JSON serialization failed: {err}");
                        "JSON serialization failed".to_string()
                    }
                };
                let direct_json = match serde_json::to_string_pretty(&direct_file) {
                    Ok(json) => json,
                    Err(err) => {
                        eprintln!("Direct JSON serialization failed: {err}");
                        "JSON serialization failed".to_string()
                    }
                };

                // Write to temp files
                let ts_path = "/tmp/ts_output.json";
                let direct_path = "/tmp/direct_output.json";

                fs::write(ts_path, &ts_json)?;
                fs::write(direct_path, &direct_json)?;

                println!("TreeSitter output written to: {}", ts_path);
                println!("Direct output written to: {}", direct_path);
                println!("\nCompare with:");
                println!("  diff {} {}", ts_path, direct_path);
                println!("  code --diff {} {}", ts_path, direct_path);
            }
        }
        (ParseOutcome::Parsed(_), ParseOutcome::Rejected) => {
            println!("✗ DirectParser FAILED (TreeSitter succeeded)");
            let errors = direct_errors.into_vec();
            println!("\nDirectParser errors ({}):", errors.len());
            for err in errors {
                println!("  - {}", err.message);
            }
        }
        (ParseOutcome::Rejected, ParseOutcome::Parsed(_)) => {
            println!("✗ TreeSitterParser FAILED (Direct succeeded)");
            let errors = ts_errors.into_vec();
            println!("\nTreeSitterParser errors ({}):", errors.len());
            for err in errors {
                println!("  - {}", err.message);
            }
        }
        (ParseOutcome::Rejected, ParseOutcome::Rejected) => {
            println!("✗ BOTH parsers FAILED");
            let ts_errs = ts_errors.into_vec();
            let direct_errs = direct_errors.into_vec();
            println!("\nTreeSitterParser errors ({}):", ts_errs.len());
            for err in ts_errs {
                println!("  - {}", err.message);
            }
            println!("\nDirectParser errors ({}):", direct_errs.len());
            for err in direct_errs {
                println!("  - {}", err.message);
            }
        }
    }

    Ok(())
}
