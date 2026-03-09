//! Build corpus manifest for a data directory.
//!
//! Usage: cargo run --release --bin build-corpus-manifest [-- <data_directory>]
//!
//! This scans the given directory (default resolution: sibling `../data` from
//! the `talkbank-tools` repo root, then legacy `~/data`) for all corpora
//! (directories with `0metadata.cdc`) and creates a manifest file for batch
//! processing.

use std::path::PathBuf;
use talkbank_transform::{build_manifest, format_manifest};

/// Build and persist a corpus manifest.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let data_root = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        default_data_root()
    };

    if !data_root.exists() {
        eprintln!("Data directory not found: {}", data_root.display());
        std::process::exit(1);
    }

    println!("Building corpus manifest for: {}", data_root.display());
    println!();

    match build_manifest(&data_root) {
        Ok(manifest) => {
            println!();
            print!("{}", format_manifest(&manifest));

            // Save manifest
            let cache_dir = home_dir_or_exit().join(".cache/talkbank-tools");

            if let Err(e) = std::fs::create_dir_all(&cache_dir) {
                eprintln!("Failed to create cache directory: {}", e);
                std::process::exit(1);
            }

            let manifest_path = cache_dir.join("corpus-manifest.json");

            match manifest.save(&manifest_path) {
                Ok(()) => {
                    println!();
                    println!("✓ Manifest saved to: {}", manifest_path.display());
                    println!(
                        "  {} corpora, {} files",
                        manifest.total_corpora, manifest.total_files
                    );
                }
                Err(e) => {
                    eprintln!("Failed to save manifest: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to build manifest: {}", e);
            std::process::exit(1);
        }
    }
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

fn default_data_root() -> PathBuf {
    let workspace_data = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|dir| dir.join("data"))
        .unwrap_or_else(|| home_dir_or_exit().join("talkbank").join("data"));
    if workspace_data.exists() {
        return workspace_data;
    }

    let legacy_data = home_dir_or_exit().join("data");
    if legacy_data.exists() {
        return legacy_data;
    }

    workspace_data
}
