//! Snapshot Tests for DirectParser
//!
//! These tests capture the expected output of DirectParser for various CHAT
//! constructs. Snapshots provide:
//!
//! - **Visual documentation** of parser output format
//! - **Regression detection** when parser behavior changes
//! - **Easy review** of changes via `cargo insta review`
//!
//! ## Usage
//!
//! ```bash
//! # Run all snapshot tests
//! cargo test -p talkbank-direct-parser --test snapshot_tests
//!
//! # Review and accept new snapshots
//! cargo insta review
//!
//! # Update snapshots automatically (use with caution!)
//! cargo insta test --accept
//! ```
//!
//! ## When to Use
//!
//! - After implementing a new DirectParser feature
//! - When changing parser output format
//! - To document expected behavior visually
//! - Before committing parser changes
//!
//! ## Workflow
//!
//! 1. Implement DirectParser feature
//! 2. Add snapshot test for that feature
//! 3. Run test (creates snapshot)
//! 4. Review snapshot with `cargo insta review`
//! 5. Accept if output looks correct
//! 6. Commit both code and snapshot

use insta::assert_debug_snapshot;
use talkbank_direct_parser::DirectParser;
use talkbank_model::ChatParser;
use talkbank_model::ErrorCollector;
use talkbank_parser_tests::test_error::TestError;

/// Captures a debug snapshot for a single-word parse case.
fn snapshot_word_case(snapshot_name: &str, input: &str) -> Result<(), TestError> {
    let direct = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors = ErrorCollector::new();
    let word = direct.parse_word(input, 0, &errors).into_option();
    assert_debug_snapshot!(snapshot_name, (word, errors.to_vec()));
    Ok(())
}

/// Captures a debug snapshot for a full CHAT file parse case.
fn snapshot_file_case(snapshot_name: &str, input: &str) -> Result<(), TestError> {
    let direct = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors = ErrorCollector::new();
    let file = ChatParser::parse_chat_file(&direct, input, 0, &errors).into_option();
    assert_debug_snapshot!(snapshot_name, (file, errors.to_vec()));
    Ok(())
}

// =============================================================================
// Word-Level Snapshots
// =============================================================================

/// Snapshots a simple lexical word parse.
#[test]
fn snapshot_word_simple() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_simple", "hello")
}

/// Snapshots a compound-word parse.
#[test]
fn snapshot_word_compound() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_compound", "wai4+yu3")
}

/// Snapshots a triple-compound-word parse.
#[test]
fn snapshot_word_compound_triple() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_compound_triple", "hello+world+test")
}

/// Snapshots a word containing an opening overlap marker.
#[test]
fn snapshot_word_overlap_open() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_overlap_open", "hello⌈")
}

/// Snapshots a word containing a closing overlap marker.
#[test]
fn snapshot_word_overlap_close() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_overlap_close", "⌉world")
}

/// Snapshots a word containing an internal overlap marker.
#[test]
fn snapshot_word_overlap_internal() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_overlap_internal", "hel⌈lo")
}

/// Snapshots a word with primary stress.
#[test]
fn snapshot_word_stress_primary() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_stress_primary", "ˈstress")
}

/// Snapshots a word with secondary stress.
#[test]
fn snapshot_word_stress_secondary() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_stress_secondary", "ˌsecondary")
}

/// Snapshots a word combining primary and secondary stress markers.
#[test]
fn snapshot_word_stress_combined() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_stress_combined", "ˈpriˌmary")
}

/// Snapshots a word with a single lengthening marker.
#[test]
fn snapshot_word_lengthening_single() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_lengthening_single", "hello:")
}

/// Snapshots a word with repeated lengthening markers.
#[test]
fn snapshot_word_lengthening_double() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_lengthening_double", "wo::rld")
}

/// Snapshots a parenthesized shortening form.
#[test]
fn snapshot_word_shortening_paren() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_shortening_paren", "goin(g)")
}

/// Snapshots an apostrophe shortening form.
#[test]
fn snapshot_word_shortening_apostrophe() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_shortening_apostrophe", "doin'")
}

/// Snapshots a word containing a CA glottal marker.
#[test]
fn snapshot_word_ca_glottal() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_ca_glottal", "hel‡lo")
}

/// Snapshots a word containing a CA rising marker.
#[test]
fn snapshot_word_ca_rising() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_ca_rising", "wo↑rld")
}

/// Snapshots a word containing a syllable-pause marker.
#[test]
fn snapshot_word_syllable_pause() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_syllable_pause", "hel^lo")
}

/// Snapshots a word combining multiple CHAT markers.
#[test]
fn snapshot_word_complex_combination() -> Result<(), TestError> {
    snapshot_word_case("snapshot_word_complex_combination", "ˈhel⌈lo+wor⌉ld:")
}

// =============================================================================
// File-Level Snapshots
// =============================================================================

/// Snapshots a minimal valid CHAT file.
#[test]
fn snapshot_file_minimal() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n*CHI:\thello world .\n@End\n";
    snapshot_file_case("snapshot_file_minimal", input)
}

/// Snapshots a file containing compound words.
#[test]
fn snapshot_file_compound_words() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n*CHI:\twai4+yu3 hello+world .\n@End\n";
    snapshot_file_case("snapshot_file_compound_words", input)
}

/// Snapshots a file containing cross-speaker overlap points.
#[test]
fn snapshot_file_overlap_points() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n*CHI:\ta ⌈ here .\n*MOT:\thello⌈ world .\n@End\n";
    snapshot_file_case("snapshot_file_overlap_points", input)
}

/// Snapshots a file containing stress markers.
#[test]
fn snapshot_file_stress_markers() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n*CHI:\tˈstress ˌsecondary ˈpriˌmary .\n@End\n";
    snapshot_file_case("snapshot_file_stress_markers", input)
}

/// Snapshots a file containing lengthening markers.
#[test]
fn snapshot_file_lengthening() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n*CHI:\thello: wo::rld .\n@End\n";
    snapshot_file_case("snapshot_file_lengthening", input)
}

/// Snapshots a file containing a `%mor` tier.
#[test]
fn snapshot_file_with_mor_tier() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n*CHI:\tI want cookies .\n%mor:\tpro:sub|I v|want n|cookie-PL .\n@End\n";
    snapshot_file_case("snapshot_file_with_mor_tier", input)
}

/// Snapshots a file containing a `%gra` tier.
#[test]
fn snapshot_file_with_gra_tier() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n*CHI:\tI want cookies .\n%mor:\tpro:sub|I v|want n|cookie-PL .\n%gra:\t1|2|SUBJ 2|0|ROOT 3|2|OBJ 4|2|PUNCT\n@End\n";
    snapshot_file_case("snapshot_file_with_gra_tier", input)
}

/// Snapshots a file containing multiple utterances.
#[test]
fn snapshot_file_multiple_utterances() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n*CHI:\thello .\n*MOT:\thi there .\n*CHI:\thow are you ?\n@End\n";
    snapshot_file_case("snapshot_file_multiple_utterances", input)
}
