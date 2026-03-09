//! Test module for test stability in `talkbank-tools`.
//!
//! These tests document expected behavior and regressions.

use std::error::Error;
use std::fs;
use std::path::Path;
use talkbank_parser::TreeSitterParser;

/// Entry point for this binary target.
fn main() -> Result<(), Box<dyn Error>> {
    let parser = TreeSitterParser::new()?;
    let dir_arg = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "testchat/good".to_string());
    let testchat_dir = Path::new(&dir_arg);

    let mut total = 0;
    let mut passing = 0;
    let mut failures: Vec<(String, String)> = Vec::new();

    for entry in fs::read_dir(testchat_dir)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                failures.push(("<read_dir>".to_string(), err.to_string()));
                continue;
            }
        };
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("cha") {
            total += 1;
            let filename = match path.file_name().and_then(|s| s.to_str()) {
                Some(name) => name.to_string(),
                None => path.display().to_string(),
            };

            match fs::read_to_string(&path) {
                Ok(original) => {
                    // Parse original
                    match parser.parse_chat_file(&original) {
                        Ok(chat_file) => {
                            // Serialize to canonical form
                            let canonical = format!("{}", chat_file);

                            // Re-parse canonical
                            match parser.parse_chat_file(&canonical) {
                                Ok(chat_file2) => {
                                    // Serialize again - should be stable
                                    let canonical2 = format!("{}", chat_file2);

                                    if canonical == canonical2 {
                                        passing += 1;
                                    } else {
                                        failures.push((
                                            filename.clone(),
                                            "canonical form not stable".to_string(),
                                        ));
                                    }
                                }
                                Err(e) => {
                                    failures.push((
                                        filename.clone(),
                                        format!("canonical form failed to parse: {}", e),
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            failures.push((
                                filename.clone(),
                                format!("original failed to parse: {}", e),
                            ));
                        }
                    }
                }
                Err(_) => {
                    failures.push((filename.clone(), "read error".to_string()));
                }
            }
        }
    }

    println!(
        "Canonical Stability Test: {}/{} ({:.1}%)",
        passing,
        total,
        (passing as f64 / total as f64) * 100.0
    );

    if failures.len() <= 20 {
        println!(
            "
Failures:"
        );
        for (filename, reason) in failures {
            println!("  - {}: {}", filename, reason);
        }
    } else {
        println!("\nFirst 20 failures:");
        for (filename, reason) in failures.iter().take(20) {
            println!("  - {}: {}", filename, reason);
        }
    }

    Ok(())
}
