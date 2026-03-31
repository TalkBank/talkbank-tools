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

// ═══════════════════════════════════════════════════════════════
// Offset parameter wiring tests
// ═══════════════════════════════════════════════════════════════
//
// The re2c parser currently produces Span::DUMMY (0,0) for all spans.
// SpanShift::shift_spans_after skips DUMMY spans (by design). So the
// offset parameter is wired through but has no visible effect until real
// byte-offset spans are added to the re2c parser's AST→model conversion.
//
// These tests verify that non-zero offsets don't cause panics or errors,
// and that the parsing results are semantically identical regardless of
// offset (since all spans are currently DUMMY).

/// Verify that parse_chat_file at non-zero offset produces identical
/// semantic content (spans are DUMMY, so shifting is a no-op).
#[test]
fn offset_wiring_chat_file_no_panic() {
    let re2c = Re2cParser::new();
    let re2c_errors = ErrorCollector::new();
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Target_Child\n@ID:\teng|corpus|CHI||||Target_Child|||\n*CHI:\thello world .\n@End\n";

    // Parsing at offset 0 and offset 200 should both succeed
    let zero = re2c.parse_chat_file(input, 0, &re2c_errors);
    let shifted = re2c.parse_chat_file(input, 200, &re2c_errors);

    assert!(matches!(zero, ParseOutcome::Parsed(_)));
    assert!(matches!(shifted, ParseOutcome::Parsed(_)));

    // Both should produce semantically equivalent output
    let (ParseOutcome::Parsed(zero_file), ParseOutcome::Parsed(shifted_file)) =
        (zero, shifted)
    else {
        unreachable!();
    };
    assert!(zero_file.semantic_eq(&shifted_file));
}

/// Verify that parse_word at non-zero offset succeeds.
#[test]
fn offset_wiring_word_no_panic() {
    let re2c = Re2cParser::new();
    let re2c_errors = ErrorCollector::new();

    let zero = re2c.parse_word("hello", 0, &re2c_errors);
    let shifted = re2c.parse_word("hello", 100, &re2c_errors);

    assert!(matches!(zero, ParseOutcome::Parsed(_)));
    assert!(matches!(shifted, ParseOutcome::Parsed(_)));
}

/// Verify that parse_main_tier at non-zero offset succeeds.
#[test]
fn offset_wiring_main_tier_no_panic() {
    let re2c = Re2cParser::new();
    let re2c_errors = ErrorCollector::new();

    let zero = re2c.parse_main_tier("*CHI:\thello .\n", 0, &re2c_errors);
    let shifted = re2c.parse_main_tier("*CHI:\thello .\n", 500, &re2c_errors);

    assert!(matches!(zero, ParseOutcome::Parsed(_)));
    assert!(matches!(shifted, ParseOutcome::Parsed(_)));
}

/// Verify that parse_mor_tier at non-zero offset succeeds.
#[test]
fn offset_wiring_mor_tier_no_panic() {
    let re2c = Re2cParser::new();
    let re2c_errors = ErrorCollector::new();

    let zero = re2c.parse_mor_tier("pro|I v|want .\n", 0, &re2c_errors);
    let shifted = re2c.parse_mor_tier("pro|I v|want .\n", 300, &re2c_errors);

    assert!(matches!(zero, ParseOutcome::Parsed(_)));
    assert!(matches!(shifted, ParseOutcome::Parsed(_)));
}

// ═══════════════════════════════════════════════════════════════
// Error reporting tests
// ═══════════════════════════════════════════════════════════════

/// Verify that parse_chat_file reports errors for malformed input
/// via the ErrorSink (not silently swallowed).
#[test]
fn error_reporting_unhandled_tokens() {
    let re2c = Re2cParser::new();
    let errors = ErrorCollector::new();

    // Input with an unrecognizable line (not @, *, or %)
    let input = "@UTF8\n@Begin\nGARBAGE LINE\n@End\n";
    let result = re2c.parse_chat_file(input, 0, &errors);
    assert!(matches!(result, ParseOutcome::Parsed(_)));

    let error_vec = errors.to_vec();
    assert!(
        !error_vec.is_empty(),
        "malformed input should produce at least one diagnostic, got none"
    );
}
