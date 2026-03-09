//! Validates error specifications against actual parser behavior
//!
//! CRITICAL: This tool prevents layer classification bugs by testing
//! each spec's input against the actual parser before specs are committed.
//!
//! ## The Problem
//!
//! Original specs had errors marked as `layer="validation"` but their inputs
//! actually failed to parse (should be `layer="parser"`). This happened because
//! no validation step existed in the spec creation process.
//!
//! ## The Solution
//!
//! This tool enforces:
//! - Specs marked "validation" must have inputs that parse successfully
//! - Specs marked "parser" must have inputs that fail to parse
//!
//! ## Usage
//!
//! ```bash
//! cargo run --bin validate_error_specs
//! ```
//!
//! Returns exit code 0 if all specs are valid, 1 otherwise.

use clap::Parser;
use generators::spec::error_corpus::ErrorCorpusSpec;
use std::path::PathBuf;
use talkbank_parser::TreeSitterParser;

/// CLI arguments: directory containing error spec files to validate.
#[derive(Parser)]
#[command(name = "validate_error_specs")]
#[command(about = "Validate error specifications against actual parser behavior")]
struct Args {
    /// Root directory containing error specs
    #[arg(short, long, default_value = "spec/errors")]
    spec_dir: PathBuf,
}

/// Validates all error specifications
fn main() {
    let args = Args::parse();

    eprintln!(
        "🔍 Validating error specifications from: {}\n",
        args.spec_dir.display()
    );

    match validate_all_specs(&args.spec_dir) {
        Ok(_) => std::process::exit(0),
        Err(_) => std::process::exit(1),
    }
}

/// Validate all error specifications in a directory
fn validate_all_specs(spec_dir: &PathBuf) -> Result<(), String> {
    // Load all specs
    let specs =
        ErrorCorpusSpec::load_all(spec_dir).map_err(|e| format!("Failed to load specs: {}", e))?;

    if specs.is_empty() {
        eprintln!("⚠️  No specs found in {}", spec_dir.display());
        return Ok(());
    }

    eprintln!("Found {} spec files\n", specs.len());

    // Initialize parser
    let parser = TreeSitterParser::new().map_err(|e| format!("Failed to create parser: {}", e))?;

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut total = 0;

    // Validate each spec
    for spec in specs {
        for example in &spec.examples {
            total += 1;
            let error_code = example.error_code.as_deref().unwrap_or("UNKNOWN");

            // Test parsing
            let parse_result = parser.parse_chat_file(&example.input);

            // Check layer classification
            match (spec.metadata.layer.as_str(), parse_result) {
                ("validation", Err(_)) => {
                    errors.push(format!(
                        "❌ {}: marked 'validation' but input fails to parse\n   \
                         FIX: Change layer to 'parser' or fix input syntax",
                        error_code
                    ));
                }
                ("parser", Ok(_)) => {
                    warnings.push(format!(
                        "⚠️  {}: marked 'parser' but input parses successfully\n   \
                         FIX: Change layer to 'validation'",
                        error_code
                    ));
                }
                _ => {
                    // Correct classification
                    eprintln!(
                        "✓ {} - layer '{}' matches parse behavior",
                        error_code, spec.metadata.layer
                    );
                }
            }
        }
    }

    // Report results
    eprintln!();
    if !errors.is_empty() {
        eprintln!("🚨 ERRORS (must fix before committing):\n");
        for err in &errors {
            eprintln!("{}\n", err);
        }
    }

    if !warnings.is_empty() {
        eprintln!("⚠️  WARNINGS (should fix):\n");
        for warn in &warnings {
            eprintln!("{}\n", warn);
        }
    }

    if errors.is_empty() && warnings.is_empty() {
        eprintln!("✅ All {} specs correctly classified!", total);
        eprintln!("\nSafe to commit.\n");
        Ok(())
    } else {
        eprintln!("Validation failed:");
        eprintln!("  {} errors", errors.len());
        eprintln!("  {} warnings", warnings.len());
        eprintln!("  {} total specs checked\n", total);
        eprintln!("Fix errors before committing.\n");
        Err("Validation failed".to_string())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_binary_compiles() {
        // Ensures the binary compiles and dependencies are linked correctly
    }
}
