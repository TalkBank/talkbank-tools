//! Build script for talkbank-model.
//!
//! Generates a compile-time perfect hash set of ISO 639-3 language codes from
//! the authoritative registry at `clan-info/lib/fixes/ISO 639-3.txt`.
//!
//! The generated file is written to `$OUT_DIR/iso639_3_set.rs` and included
//! by `src/model/header/codes/iso639.rs` at compile time.

use std::env;
use std::fs;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;

fn main() {
    generate_iso639_3_set();
}

/// Parse the ISO 639-3 registry and generate a `phf::Set<&str>`.
fn generate_iso639_3_set() {
    // The ISO 639-3 file lives in clan-info/ which is a sibling repo in the
    // talkbank-dev workspace. Walk up from the crate root to find it.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let crate_root = Path::new(&manifest_dir);

    // Try workspace root (talkbank-tools/) → parent (talkbank-dev/) → clan-info/
    let iso_file = crate_root
        .parent() // crates/
        .and_then(|p| p.parent()) // talkbank-tools/
        .and_then(|p| p.parent()) // talkbank-dev/
        .map(|workspace| workspace.join("clan-info/lib/fixes/ISO 639-3.txt"));

    let iso_path = match iso_file {
        Some(ref p) if p.exists() => p.clone(),
        _ => {
            // Fallback: emit an empty set if the file isn't available
            // (e.g., CI without clan-info cloned).
            eprintln!(
                "cargo:warning=ISO 639-3 file not found, generating empty set. \
                 Language code membership validation will be disabled."
            );
            generate_empty_set();
            return;
        }
    };

    println!("cargo:rerun-if-changed={}", iso_path.display());

    let content = fs::read_to_string(&iso_path).unwrap_or_else(|e| {
        panic!(
            "Failed to read ISO 639-3 file at {}: {}",
            iso_path.display(),
            e
        )
    });

    let mut codes: Vec<&str> = Vec::with_capacity(8500);

    for line in content.lines() {
        // Format: `aaa\t|...|...|Language Name
        // The backtick prefix + 3-letter code is positions 0..4.
        if line.starts_with('`') && line.len() >= 4 {
            let code = &line[1..4];
            if code.len() == 3 && code.chars().all(|c| c.is_ascii_lowercase()) {
                codes.push(code);
            }
        }
    }

    if codes.is_empty() {
        eprintln!("cargo:warning=No codes parsed from ISO 639-3 file, generating empty set.");
        generate_empty_set();
        return;
    }

    // Generate the phf set.
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("iso639_3_set.rs");
    let file = fs::File::create(&dest_path).unwrap();
    let mut writer = BufWriter::new(file);

    writeln!(
        writer,
        "/// ISO 639-3 language code set ({} codes).",
        codes.len()
    )
    .unwrap();
    writeln!(
        writer,
        "/// Generated from clan-info/lib/fixes/ISO 639-3.txt by build.rs."
    )
    .unwrap();

    let mut set = phf_codegen::Set::new();
    for code in &codes {
        set.entry(*code);
    }

    writeln!(
        writer,
        "static ISO_639_3_CODES: phf::Set<&'static str> = {};",
        set.build()
    )
    .unwrap();

    eprintln!(
        "cargo:warning=Generated ISO 639-3 set with {} codes",
        codes.len()
    );
}

/// Generate an empty set for environments without the ISO file.
fn generate_empty_set() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("iso639_3_set.rs");
    let mut file = fs::File::create(&dest_path).unwrap();
    writeln!(
        file,
        "/// Empty ISO 639-3 set (file not available at build time).\n\
         static ISO_639_3_CODES: phf::Set<&'static str> = {};",
        phf_codegen::Set::<&str>::new().build()
    )
    .unwrap();
}
