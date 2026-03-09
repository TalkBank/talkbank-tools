//! Query corpus manifest for error details and statistics.
//!
//! This tool allows inspecting the corpus manifest without re-running tests,
//! enabling quick analysis of failure patterns across corpora.
//!
//! Usage:
//!   cargo run --release --bin query-manifest -- --error-type ParseError
//!   cargo run --release --bin query-manifest -- --corpus OralArguments --status Failed
//!   cargo run --release --bin query-manifest -- --message "alignment"
//!   cargo run --release --bin query-manifest -- --status Failed --export failures.json

use clap::Parser;
use std::path::PathBuf;
use talkbank_transform::corpus::manifest::{CorpusManifest, FileStatus};

/// CLI filters and output options for manifest queries.
#[derive(Parser, Debug)]
#[command(author, version, about = "Query corpus manifest for errors and statistics", long_about = None)]
struct Args {
    /// Filter by error type (e.g., ParseError, ValidationError, ChatMismatch)
    #[arg(long)]
    error_type: Option<String>,

    /// Filter by corpus name (partial match)
    #[arg(long)]
    corpus: Option<String>,

    /// Filter by file status (Passed, Failed, NotTested)
    #[arg(long, value_parser = parse_file_status)]
    status: Option<FileStatus>,

    /// Filter by error message (substring match, case-insensitive)
    #[arg(long)]
    message: Option<String>,

    /// Export results to JSON file
    #[arg(long)]
    export: Option<PathBuf>,

    /// Show full error details (not just summary)
    #[arg(long)]
    verbose: bool,

    /// Limit number of results shown
    #[arg(long, default_value = "50")]
    limit: usize,

    /// Path to manifest file (defaults to ~/.cache/talkbank-tools/corpus-manifest.json)
    #[arg(long)]
    manifest: Option<PathBuf>,
}

/// Parse user-provided status strings into manifest enum values.
fn parse_file_status(s: &str) -> Result<FileStatus, String> {
    match s.to_lowercase().as_str() {
        "passed" => Ok(FileStatus::Passed),
        "failed" => Ok(FileStatus::Failed),
        "nottested" | "not_tested" => Ok(FileStatus::NotTested),
        _ => Err(format!(
            "Invalid status '{}'. Must be one of: Passed, Failed, NotTested",
            s
        )),
    }
}

/// Flattened query row used for display and optional JSON export.
#[derive(Debug, serde::Serialize)]
struct QueryResult {
    corpus_name: String,
    corpus_path: String,
    file_path: String,
    status: String,
    failure_reason: Option<String>,
    error_type: Option<String>,
    error_message: Option<String>,
    error_location: Option<String>,
    diff_summary: Option<String>,
}

/// Execute a filtered manifest query and print summary statistics.
fn main() {
    let args = Args::parse();

    // Determine manifest path
    let manifest_path = match args.manifest {
        Some(path) => path,
        None => home_dir_or_exit().join(".cache/talkbank-tools/corpus-manifest.json"),
    };

    if !manifest_path.exists() {
        eprintln!("❌ Manifest not found: {}", manifest_path.display());
        eprintln!();
        eprintln!("Run: cargo run --release --bin build-corpus-manifest");
        std::process::exit(1);
    }

    // Load manifest
    let manifest = match CorpusManifest::load(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("❌ Failed to load manifest: {}", e);
            std::process::exit(1);
        }
    };

    // Collect matching files
    let mut results = Vec::new();

    for (corpus_path_key, corpus_entry) in &manifest.corpora {
        // Filter by corpus name
        if let Some(ref corpus_filter) = args.corpus
            && !corpus_entry
                .name
                .to_lowercase()
                .contains(&corpus_filter.to_lowercase())
        {
            continue;
        }

        for file_entry in corpus_entry.files.values() {
            // Filter by status
            if let Some(status_filter) = args.status
                && file_entry.status != status_filter
            {
                continue;
            }

            // Filter by error type
            if let Some(ref error_type_filter) = args.error_type {
                match &file_entry.error_detail {
                    Some(detail)
                        if detail.error_type.to_lowercase() == error_type_filter.to_lowercase() => {
                    }
                    _ => continue,
                }
            }

            // Filter by message
            if let Some(ref message_filter) = args.message {
                match &file_entry.error_detail {
                    Some(detail)
                        if detail
                            .message
                            .to_lowercase()
                            .contains(&message_filter.to_lowercase()) => {}
                    _ => continue,
                }
            }

            // Build result
            let (error_type, error_message, error_location, diff_summary) =
                match &file_entry.error_detail {
                    Some(detail) => {
                        let location = detail
                            .location
                            .as_ref()
                            .map(|loc| format!("line {}, column {}", loc.line, loc.column));
                        (
                            Some(detail.error_type.clone()),
                            Some(detail.message.clone()),
                            location,
                            detail.diff_summary.clone(),
                        )
                    }
                    None => (None, None, None, None),
                };

            results.push(QueryResult {
                corpus_name: corpus_entry.name.clone(),
                corpus_path: corpus_path_key.clone(),
                file_path: file_entry.path.clone(),
                status: file_entry.status.to_string(),
                failure_reason: file_entry.failure_reason.as_ref().map(|r| r.to_string()),
                error_type,
                error_message,
                error_location,
                diff_summary,
            });
        }
    }

    // Show summary
    println!();
    println!("🔍 Query Results");
    println!("═══════════════════════════════════════════════════════════");
    println!();

    if results.is_empty() {
        println!("No matching files found.");
        println!();
        return;
    }

    println!("Found {} matching files", results.len());
    println!();

    // Export to JSON if requested
    if let Some(export_path) = args.export {
        match serde_json::to_string_pretty(&results) {
            Ok(json) => match std::fs::write(&export_path, json) {
                Ok(()) => {
                    println!(
                        "✓ Exported {} results to: {}",
                        results.len(),
                        export_path.display()
                    );
                    println!();
                }
                Err(e) => {
                    eprintln!("❌ Failed to write export file: {}", e);
                }
            },
            Err(e) => {
                eprintln!("❌ Failed to serialize results: {}", e);
            }
        }
    }

    // Display results
    let display_count = results.len().min(args.limit);
    println!("Showing {} of {} results:", display_count, results.len());
    println!();

    for (idx, result) in results.iter().take(display_count).enumerate() {
        println!("{}. {}", idx + 1, result.file_path);
        println!("   Corpus:  {}", result.corpus_name);
        println!("   Status:  {}", result.status);

        if let Some(ref reason) = result.failure_reason {
            println!("   Reason:  {}", reason);
        }

        if let Some(ref error_type) = result.error_type {
            println!("   Type:    {}", error_type);
        }

        if args.verbose {
            if let Some(ref message) = result.error_message {
                println!("   Message: {}", message);
            }

            if let Some(ref location) = result.error_location {
                println!("   Location: {}", location);
            }

            if let Some(ref diff) = result.diff_summary {
                println!("   Diff:    {}", diff);
            }
        }

        println!();
    }

    if results.len() > display_count {
        println!("... and {} more results", results.len() - display_count);
        println!("Use --limit {} to see more", results.len());
        println!();
    }

    // Show statistics
    println!("Statistics:");
    println!("───────────────────────────────────────────────────────────");

    // Count by status
    let passed_count = results.iter().filter(|r| r.status == "Passed").count();
    let failed_count = results.iter().filter(|r| r.status == "Failed").count();
    let not_tested_count = results.iter().filter(|r| r.status == "NotTested").count();

    println!("  Status:");
    if passed_count > 0 {
        println!("    ✓ Passed:      {}", passed_count);
    }
    if failed_count > 0 {
        println!("    ✗ Failed:      {}", failed_count);
    }
    if not_tested_count > 0 {
        println!("    ⏳ Not tested:  {}", not_tested_count);
    }

    // Count by error type
    if failed_count > 0 {
        let mut error_type_counts = std::collections::HashMap::new();
        for result in &results {
            if let Some(ref error_type) = result.error_type {
                *error_type_counts.entry(error_type.clone()).or_insert(0) += 1;
            }
        }

        if !error_type_counts.is_empty() {
            println!();
            println!("  Error Types:");
            let mut sorted_types: Vec<_> = error_type_counts.iter().collect();
            sorted_types.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending

            for (error_type, count) in sorted_types {
                println!("    {}: {}", error_type, count);
            }
        }
    }

    // Count by corpus
    let mut corpus_counts = std::collections::HashMap::new();
    for result in &results {
        *corpus_counts.entry(result.corpus_name.clone()).or_insert(0) += 1;
    }

    if corpus_counts.len() > 1 {
        println!();
        println!("  Corpora:");
        let mut sorted_corpora: Vec<_> = corpus_counts.iter().collect();
        sorted_corpora.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count descending

        for (corpus_name, count) in sorted_corpora.iter().take(10) {
            println!("    {}: {}", corpus_name, count);
        }

        if sorted_corpora.len() > 10 {
            println!("    ... and {} more", sorted_corpora.len() - 10);
        }
    }

    println!();
}

/// Resolve `$HOME` or terminate with a user-facing error.
fn home_dir_or_exit() -> std::path::PathBuf {
    match dirs::home_dir() {
        Some(dir) => dir,
        None => {
            eprintln!("Failed to get home directory");
            std::process::exit(1);
        }
    }
}
