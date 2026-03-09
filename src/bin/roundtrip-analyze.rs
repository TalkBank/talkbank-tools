//! CLI tool for analyzing roundtrip test results and generating reports.
//!
//! Usage:
//!   cargo run --release --bin roundtrip-analyze -- <diffs_directory>
//!
//! This tool analyzes the diff files created by roundtrip tests and categorizes failures
//! by type (terminator issues, spacing issues, etc.).

use std::fs;
use std::path::Path;

/// Analyze saved roundtrip diff artifacts and print a categorized failure report.
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: roundtrip-analyze <diffs_directory>");
        eprintln!();
        eprintln!("Analyzes roundtrip test diff files and generates a failure report.");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  roundtrip-analyze ~/talkbank-roundtrip-json/diffs/");
        std::process::exit(1);
    }

    let diffs_dir = Path::new(&args[1]);

    if !diffs_dir.exists() {
        eprintln!("Error: Directory not found: {}", diffs_dir.display());
        std::process::exit(1);
    }

    println!(
        "\nAnalyzing roundtrip test failures in: {}",
        diffs_dir.display()
    );

    // Count failures by type
    let mut failure_summary = FailureSummary::default();

    if let Ok(entries) = fs::read_dir(diffs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.ends_with(".canonical-original") {
                    let serialized = path
                        .to_string_lossy()
                        .replace(".canonical-original", ".canonical-serialized");
                    if let (Ok(original), Ok(serialized_content)) =
                        (fs::read_to_string(&path), fs::read_to_string(&serialized))
                    {
                        analyze_file_diff(&original, &serialized_content, &mut failure_summary);
                    }
                }
            }
        }
    }

    print_failure_report(&failure_summary);
}

/// Aggregate counters and examples for diff categories across analyzed files.
#[derive(Debug, Default)]
struct FailureSummary {
    terminator_differences: Vec<TerminatorDiff>,
    spacing_differences: Vec<String>,
    other_differences: Vec<(String, String)>,
    total_files_analyzed: usize,
}

/// Unique before/after terminator mismatch pattern plus frequency.
#[derive(Debug, Clone)]
struct TerminatorDiff {
    original: String,
    serialized: String,
    count: usize,
}

/// Classify line-level differences from one file pair into summary buckets.
fn analyze_file_diff(original: &str, serialized: &str, summary: &mut FailureSummary) {
    summary.total_files_analyzed += 1;

    for (i, (orig_line, ser_line)) in original.lines().zip(serialized.lines()).enumerate() {
        if orig_line != ser_line {
            // Check for terminator issues
            if orig_line.contains("+")
                && ser_line.contains("+")
                && (orig_line.contains("+!?")
                    || ser_line.contains("+!?")
                    || orig_line.contains("+/?")
                    || ser_line.contains("+/?"))
            {
                let term_diff = TerminatorDiff {
                    original: orig_line.to_string(),
                    serialized: ser_line.to_string(),
                    count: 1,
                };

                // Check if we already have this pattern
                if let Some(existing) = summary.terminator_differences.iter_mut().find(|t| {
                    t.original == term_diff.original && t.serialized == term_diff.serialized
                }) {
                    existing.count += 1;
                } else {
                    summary.terminator_differences.push(term_diff);
                }
                continue;
            }

            // Check for spacing issues (same content, different whitespace)
            if orig_line.replace(" ", "") == ser_line.replace(" ", "") {
                summary
                    .spacing_differences
                    .push(format!("Line {}: spacing differs", i + 1));
            } else {
                summary
                    .other_differences
                    .push((orig_line.to_string(), ser_line.to_string()));
            }
        }
    }
}

/// Render a human-readable report with category totals and follow-up guidance.
fn print_failure_report(summary: &FailureSummary) {
    println!("\n{}", "=".repeat(80));
    println!("ROUNDTRIP FAILURE ANALYSIS REPORT");
    println!("{}", "=".repeat(80));

    println!("\n## Summary");
    println!("Files analyzed: {}", summary.total_files_analyzed);
    println!(
        "Terminator differences: {}",
        summary.terminator_differences.len()
    );
    println!("Spacing differences: {}", summary.spacing_differences.len());
    println!("Other differences: {}", summary.other_differences.len());

    if !summary.terminator_differences.is_empty() {
        println!("\n## Terminator Issues");
        for (idx, term_diff) in summary.terminator_differences.iter().take(10).enumerate() {
            println!(
                "\n{}.  Pattern #{} (appears {} times):",
                idx + 1,
                idx + 1,
                term_diff.count
            );
            println!("  Original:   {}", term_diff.original);
            println!("  Serialized: {}", term_diff.serialized);
        }
        if summary.terminator_differences.len() > 10 {
            println!(
                "\n  ... and {} more terminator patterns",
                summary.terminator_differences.len() - 10
            );
        }
    }

    if !summary.spacing_differences.is_empty() {
        println!(
            "\n## Spacing Issues ({} instances)",
            summary.spacing_differences.len()
        );
        println!(
            "Spacing differs in various lines - typically around punctuation after replacements."
        );
    }

    if !summary.other_differences.is_empty() {
        println!(
            "\n## Other Differences ({} instances)",
            summary.other_differences.len()
        );
        for (idx, (orig, ser)) in summary.other_differences.iter().take(5).enumerate() {
            println!("\n{}.  Original:   {}", idx + 1, orig);
            println!("    Serialized: {}", ser);
        }
        if summary.other_differences.len() > 5 {
            println!(
                "\n  ... and {} more differences",
                summary.other_differences.len() - 5
            );
        }
    }

    println!("\n## Recommendations");
    if !summary.terminator_differences.is_empty() {
        println!(
            "1. FIX TERMINATOR PARSING: {} distinct terminator patterns differ",
            summary.terminator_differences.len()
        );
        println!("   - Primary issue: +!? terminator vs +/?? terminator");
        println!("   - Check: talkbank-model/src/model/content/terminator.rs");
        println!("   - Check: talkbank-parser/src/parser/tree_parsing/main_tier/");
    }

    if !summary.spacing_differences.is_empty() {
        println!(
            "2. STANDARDIZE SPACING: {} lines have spacing differences",
            summary.spacing_differences.len()
        );
        println!("   - Issue: Inconsistent spaces around punctuation in replacements");
        println!("   - Example: '[: word] ,' vs '[: word] , '");
    }

    println!("\n## Next Steps");
    println!("1. Review terminator parsing in tree-sitter grammar");
    println!("2. Check canonical serialization rules for punctuation spacing");
    println!("3. Run tests with fixes to verify improvements");
    println!("4. Re-cache results after fixes");

    println!("\n{}", "=".repeat(80));
}
