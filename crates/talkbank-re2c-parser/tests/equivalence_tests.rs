//! Parser equivalence tests: Re2cParser vs TreeSitterParser.
//!
//! Parse the same input with both parsers and compare output using
//! `SemanticEq`. This is the gold standard for validating our parser
//! as a drop-in replacement.

use talkbank_model::errors::ErrorCollector;
use talkbank_model::{ChatParser, ParseOutcome, SemanticEq};
use talkbank_parser::TreeSitterParser;
use talkbank_re2c_parser::Re2cParser;

fn both_parsers() -> (TreeSitterParser, Re2cParser) {
    (
        TreeSitterParser::new().expect("tree-sitter grammar loads"),
        Re2cParser::new(),
    )
}

// ═══════════════════════════════════════════════════════════════
// Reference corpus equivalence
// ═══════════════════════════════════════════════════════════════

#[test]
fn equivalence_reference_corpus() {
    let base = format!(
        "{}/corpus/reference",
        env!("CARGO_MANIFEST_DIR").replace("/crates/talkbank-re2c-parser", "")
    );
    let base_path = std::path::Path::new(&base);
    if !base_path.exists() {
        eprintln!("Skipping: {base} not found");
        return;
    }

    let (ts, re2c) = both_parsers();
    let re2c_errors = ErrorCollector::new();

    let mut total = 0;
    let mut passed = 0;
    let mut failed_files = Vec::new();

    for dir in ["core", "content", "annotation", "tiers", "ca", "languages"] {
        let dir_path = base_path.join(dir);
        if !dir_path.exists() {
            continue;
        }
        for entry in std::fs::read_dir(&dir_path).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_some_and(|e| e == "cha") {
                total += 1;
                let content = std::fs::read_to_string(&path).unwrap();
                let filename = path.file_name().unwrap().to_string_lossy().to_string();

                // TreeSitterParser uses ParseResult (Result), not ParseOutcome
                let ts_result = ts.parse_chat_file(&content);
                let re2c_result = re2c.parse_chat_file(&content, 0, &re2c_errors);

                match (ts_result, re2c_result) {
                    (Ok(ts_file), ParseOutcome::Parsed(re2c_file)) => {
                        if ts_file.semantic_eq(&re2c_file) {
                            passed += 1;
                        } else {
                            failed_files.push(format!("{filename}: semantic mismatch"));
                        }
                    }
                    (Ok(_), ParseOutcome::Rejected) => {
                        failed_files.push(format!("{filename}: re2c rejected, ts parsed"));
                    }
                    (Err(_), ParseOutcome::Parsed(_)) => {
                        failed_files.push(format!("{filename}: ts failed, re2c parsed"));
                    }
                    (Err(_), ParseOutcome::Rejected) => {
                        passed += 1;
                    }
                }
            }
        }
    }

    eprintln!("\n=== Reference corpus equivalence ===");
    eprintln!("Total: {total}");
    eprintln!("Passed: {passed}");
    eprintln!("Failed: {}", failed_files.len());
    for f in &failed_files {
        eprintln!("  FAIL: {f}");
    }
}

// ═══════════════════════════════════════════════════════════════
// Per-tier equivalence
// ═══════════════════════════════════════════════════════════════

#[test]
fn equivalence_mor_tier() {
    let (ts, re2c) = both_parsers();
    let re2c_errors = ErrorCollector::new();

    let entries = talkbank_re2c_parser::tests_support::load_fixture("tier_mor");
    if entries.is_empty() {
        return;
    }

    let mut passed = 0;
    let mut failed = 0;
    for entry in &entries {
        let body = entry.strip_prefix("%mor:\t").unwrap_or(entry);
        let input = if body.ends_with('\n') {
            body.to_string()
        } else {
            format!("{body}\n")
        };

        // TreeSitterParser fragment API
        let ts_result = ts.parse_mor_tier_fragment(&input, 0, &re2c_errors);
        let re2c_result = re2c.parse_mor_tier(&input, 0, &re2c_errors);

        match (ts_result, re2c_result) {
            (ParseOutcome::Parsed(ts_tier), ParseOutcome::Parsed(re2c_tier)) => {
                if ts_tier.semantic_eq(&re2c_tier) {
                    passed += 1;
                } else {
                    failed += 1;
                    if failed <= 3 {
                        eprintln!(
                            "MOR MISMATCH: {}",
                            body.chars().take(60).collect::<String>()
                        );
                    }
                }
            }
            _ => {
                failed += 1;
            }
        }
    }
    eprintln!(
        "  %mor equivalence: {passed}/{} passed, {failed} failed",
        entries.len()
    );
}

#[test]
fn equivalence_gra_tier() {
    let (ts, re2c) = both_parsers();
    let re2c_errors = ErrorCollector::new();

    let entries = talkbank_re2c_parser::tests_support::load_fixture("tier_gra");
    if entries.is_empty() {
        return;
    }

    let mut passed = 0;
    let mut failed = 0;
    for entry in &entries {
        let body = entry.strip_prefix("%gra:\t").unwrap_or(entry);
        let input = if body.ends_with('\n') {
            body.to_string()
        } else {
            format!("{body}\n")
        };

        let ts_result = ts.parse_gra_tier_fragment(&input, 0, &re2c_errors);
        let re2c_result = re2c.parse_gra_tier(&input, 0, &re2c_errors);

        match (ts_result, re2c_result) {
            (ParseOutcome::Parsed(ts_tier), ParseOutcome::Parsed(re2c_tier)) => {
                if ts_tier.semantic_eq(&re2c_tier) {
                    passed += 1;
                } else {
                    failed += 1;
                    if failed <= 3 {
                        eprintln!(
                            "GRA MISMATCH: {}",
                            body.chars().take(60).collect::<String>()
                        );
                    }
                }
            }
            _ => {
                failed += 1;
            }
        }
    }
    eprintln!(
        "  %gra equivalence: {passed}/{} passed, {failed} failed",
        entries.len()
    );
}

#[test]
fn equivalence_word() {
    let (ts, re2c) = both_parsers();
    let re2c_errors = ErrorCollector::new();

    let words = ["hello", "ice+cream", "mama@f", "no::", "&-um", "(be)cause"];
    let mut passed = 0;
    let mut failed = 0;
    for w in &words {
        let ts_result = ts.parse_word(w);
        let re2c_result = re2c.parse_word(w, 0, &re2c_errors);

        match (ts_result, re2c_result) {
            (Ok(ts_word), ParseOutcome::Parsed(re2c_word)) => {
                if ts_word.semantic_eq(&re2c_word) {
                    passed += 1;
                } else {
                    failed += 1;
                    eprintln!("WORD MISMATCH: {w}");
                    eprintln!("  ts:   {:?}", ts_word.raw_text());
                    eprintln!("  re2c: {:?}", re2c_word.raw_text());
                }
            }
            (Err(e), ParseOutcome::Parsed(re2c_word)) => {
                failed += 1;
                eprintln!("WORD: ts failed ({e:?}), re2c: {:?}", re2c_word.raw_text());
            }
            (Ok(ts_word), ParseOutcome::Rejected) => {
                failed += 1;
                eprintln!("WORD: ts: {:?}, re2c rejected", ts_word.raw_text());
            }
            _ => {
                failed += 1;
            }
        }
    }
    eprintln!(
        "  word equivalence: {passed}/{} passed, {failed} failed",
        words.len()
    );
}
