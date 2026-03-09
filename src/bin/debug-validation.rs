//! Debug validation errors in a specific file.
//!
//! Usage: cargo run --release --bin debug-validation -- <file.cha>

use std::fs;
use std::path::Path;
use talkbank_model::ParseValidateOptions;
use talkbank_transform::parse_and_validate;

/// CLI entrypoint for inspecting parse/validation failures on one CHAT file.
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <file.cha>", args[0]);
        std::process::exit(1);
    }

    let file_path = Path::new(&args[1]);

    if !file_path.exists() {
        eprintln!("File not found: {}", file_path.display());
        std::process::exit(1);
    }

    let content = match fs::read_to_string(file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
            std::process::exit(1);
        }
    };

    println!("File: {}", file_path.display());
    println!("Size: {} bytes", content.len());
    println!();

    let options = ParseValidateOptions::default().with_alignment();

    match parse_and_validate(&content, options) {
        Ok(chat_file) => {
            println!("✓ Validation passed!");
            println!("  Participants: {}", chat_file.participants.len());
            let utter_count = chat_file.utterances().count();
            println!("  Utterances: {}", utter_count);
        }
        Err(talkbank_transform::PipelineError::Validation(errors)) => {
            println!("✗ Validation errors: {} found", errors.len());
            println!();

            for (idx, error) in errors.iter().enumerate().take(30) {
                let location = format!(
                    "({}-{})",
                    error.location.span.start, error.location.span.end
                );
                println!(
                    "{:3}. [{}] {} {}: {}",
                    idx + 1,
                    match error.severity {
                        talkbank_model::Severity::Error => "ERROR",
                        talkbank_model::Severity::Warning => "WARN ",
                    },
                    error.code,
                    location,
                    error.message
                );
            }

            if errors.len() > 30 {
                println!("\n... and {} more errors", errors.len() - 30);
            }
        }
        Err(talkbank_transform::PipelineError::Parse(msg)) => {
            println!("✗ Parse error: {}", msg);
        }
        Err(e) => {
            println!("✗ Error: {}", e);
        }
    }
}
