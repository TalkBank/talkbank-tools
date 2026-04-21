//! View corpus manifest information.
//!
//! Usage:
//!   cargo run --release --bin corpus-manifest-info                   # Show summary
//!   cargo run --release --bin corpus-manifest-info -- `<corpus-name>`  # Show corpus details
//!
//! This displays corpus information from the manifest created by build-corpus-manifest.

use std::path::PathBuf;
use talkbank_transform::CorpusManifest;

/// Print manifest summary or drill into one matching corpus entry.
fn main() {
    let manifest_path = home_dir_or_exit().join(".cache/talkbank-tools/corpus-manifest.json");

    if !manifest_path.exists() {
        eprintln!("Manifest not found: {}", manifest_path.display());
        eprintln!("Run: cargo run --release --bin build-corpus-manifest");
        std::process::exit(1);
    }

    let manifest = match CorpusManifest::load(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Failed to load manifest: {}", e);
            std::process::exit(1);
        }
    };

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // Show specific corpus
        let corpus_name = &args[1];
        show_corpus(&manifest, corpus_name);
    } else {
        // Show summary
        show_summary(&manifest);
    }
}

/// Render global manifest totals plus the largest corpora table.
fn show_summary(manifest: &CorpusManifest) {
    println!("Corpus Manifest Summary");
    println!("=======================");
    println!();
    println!("Total corpora:        {}", manifest.total_corpora);
    println!("Total files:          {}", manifest.total_files);
    println!();
    println!("Status:");
    println!(
        "  Passed:             {} ({:.1}%)",
        manifest.total_passed,
        if manifest.total_files > 0 {
            (manifest.total_passed as f64 / manifest.total_files as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "  Failed:             {} ({:.1}%)",
        manifest.total_failed,
        if manifest.total_files > 0 {
            (manifest.total_failed as f64 / manifest.total_files as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "  Not tested:         {} ({:.1}%)",
        manifest.total_not_tested,
        if manifest.total_files > 0 {
            (manifest.total_not_tested as f64 / manifest.total_files as f64) * 100.0
        } else {
            0.0
        }
    );
    println!();
    println!("Overall progress:     {:.1}%", manifest.overall_progress());
    println!("Overall pass rate:    {:.1}%", manifest.overall_pass_rate());
    println!();

    println!("Top 30 largest corpora:");
    println!("{:<50} {:>8} {:>8}", "Corpus Name", "Files", "Status");
    println!("{}", "-".repeat(70));

    let mut corpus_list: Vec<_> = manifest.corpora.values().collect();
    corpus_list.sort_by_key(|corpus| std::cmp::Reverse(corpus.file_count));

    for corpus in corpus_list.iter().take(30) {
        let status = if corpus.not_tested > 0 {
            format!("{} NT", corpus.passed + corpus.failed)
        } else {
            format!("✓ {}", corpus.passed)
        };

        let name = if corpus.name.len() > 50 {
            format!("...{}", &corpus.name[corpus.name.len() - 47..])
        } else {
            corpus.name.clone()
        };

        println!("{:<50} {:>8} {:>8}", name, corpus.file_count, status);
    }
}

/// Render details for the first corpus whose name/path contains the query string.
fn show_corpus(manifest: &CorpusManifest, corpus_name: &str) {
    for (path, corpus) in &manifest.corpora {
        if corpus.name.contains(corpus_name) || path.contains(corpus_name) {
            println!("Corpus: {}", corpus.name);
            println!("Path:   {}", corpus.path);
            println!();
            println!("Files:       {}", corpus.file_count);
            println!(
                "Passed:      {} ({:.1}%)",
                corpus.passed,
                corpus.pass_rate()
            );
            println!("Failed:      {}", corpus.failed);
            println!("Not tested:  {}", corpus.not_tested);
            println!();
            println!("Progress:    {:.1}%", corpus.progress());
            println!();

            if corpus.failed > 0 {
                println!("Failed files:");
                let failed_files: Vec<_> = corpus
                    .files
                    .values()
                    .filter(|f| f.status == talkbank_transform::CorpusFileStatus::Failed)
                    .collect();

                for file in failed_files.iter().take(20) {
                    let reason = match file.failure_reason.as_ref() {
                        Some(reason) => reason.to_string(),
                        None => "Unknown".to_string(),
                    };
                    println!("  {} - {}", file.path, reason);
                }

                if failed_files.len() > 20 {
                    println!("  ... and {} more", failed_files.len() - 20);
                }
            }

            return;
        }
    }

    println!("Corpus not found: {}", corpus_name);
}

/// Resolve `$HOME` or terminate with a user-facing error.
fn home_dir_or_exit() -> PathBuf {
    match dirs::home_dir() {
        Some(dir) => dir,
        None => {
            eprintln!("Failed to get home directory");
            std::process::exit(1);
        }
    }
}
