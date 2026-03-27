//! Utilities for loading test fixtures.

/// Load fixture lines from a file in tests/fixtures/.
/// Returns a Vec of logical CHAT lines (entries separated by blank lines).
/// Skips comment lines starting with #.
pub fn load_fixture(name: &str) -> Vec<String> {
    let path = format!("{}/tests/fixtures/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Skipping fixture {name}: {e}");
            return vec![];
        }
    };

    let mut entries = Vec::new();
    let mut current = String::new();

    for line in content.lines() {
        if line.starts_with('#') {
            continue;
        }
        if line.is_empty() {
            if !current.is_empty() {
                entries.push(std::mem::take(&mut current));
            }
        } else {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }
    if !current.is_empty() {
        entries.push(current);
    }

    entries
}

/// Load fixtures and verify all lex cleanly (zero error tokens).
/// Returns the entries for further testing.
pub fn load_and_verify_lex(name: &str) -> Vec<String> {
    let entries = load_fixture(name);
    if entries.is_empty() {
        eprintln!("  {name}: no fixtures (skipped)");
        return entries;
    }
    let mut errors = 0;
    for entry in &entries {
        let input = if entry.ends_with('\n') {
            entry.clone()
        } else {
            format!("{entry}\n")
        };
        let result = talkbank_re2c_parser::lex(&input);
        if !result.is_clean() {
            errors += 1;
            if errors <= 3 {
                let snippet = entry.chars().take(60).collect::<String>();
                eprintln!("  LEX ERROR in {name}: {}", snippet.escape_debug());
                eprint!("{}", result.error_report(&input));
            }
        }
    }
    assert_eq!(
        errors,
        0,
        "{name}: {errors}/{} entries had lex errors",
        entries.len()
    );
    eprintln!("  {name}: {} entries, all lex clean", entries.len());
    entries
}
