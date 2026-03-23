//! Legacy word-level parser equivalence tests.
//!
//! Tests each minimal golden word individually, reporting which specific words fail.
//! Uses the minimal list (~47 words) with ONE word per feature signature for fast core tests.
//!
//! This file is still useful for spotting divergence, but it should not be
//! treated as the oracle for direct-parser fragment semantics. Tree-sitter
//! fragment behavior is itself synthetic in some paths.
//!
//! ## Usage
//!
//! ```bash
//! # Run all word tests
//! cargo test parser_equivalence_words
//!
//! # Show failing words
//! cargo test parser_equivalence_words -- --nocapture
//! ```

use talkbank_parser::TreeSitterParser;
use talkbank_model::ErrorCollector;
use talkbank_model::model::SemanticEq;
use talkbank_model::{ChatParser, ParseOutcome};
use talkbank_parser_tests::golden::golden_words_minimal;
use talkbank_parser_tests::test_error::TestError;

/// Compare TreeSitterParser and TreeSitterParser word models for each golden word.
///
/// ## Test Strategy
///
/// For each word in golden_words.txt:
/// 1. Parse with TreeSitterParser (legacy comparison baseline)
/// 2. Parse with TreeSitterParser
/// 3. Compare using SemanticEq
/// 4. Report EACH failure individually (not just a count)
///
/// ## Failure Cases
///
/// - **TreeSitterParser fails, TreeSitter succeeds**: investigate; may still be a bug
/// - **Both parsers fail**: Not testing TreeSitterParser failures (OK to skip)
/// - **Both succeed but differ**: investigate semantic divergence
/// - **TreeSitter fails, TreeSitterParser succeeds**: may be valid direct-parser leniency
#[test]
fn all_words_equivalence() -> Result<(), TestError> {
    let ts = TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let direct = TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;

    let words = golden_words_minimal();
    let mut failures = Vec::new();

    for word in &words {
        let errors_ts = ErrorCollector::new();
        let errors_direct = ErrorCollector::new();

        let ts_result = ChatParser::parse_word(&ts, word, 0, &errors_ts);
        let direct_result = ChatParser::parse_word(&direct, word, 0, &errors_direct);

        match (ts_result, direct_result) {
            (ParseOutcome::Parsed(ts_word), ParseOutcome::Parsed(direct_word)) => {
                // Both succeeded - check semantic equivalence
                if !ts_word.semantic_eq(&direct_word) {
                    failures.push(format!(
                        "Word: {}
  MISMATCH: TreeSitter and Direct produced different models
  TreeSitter: {:?}
  Direct: {:?}",
                        word, ts_word, direct_word
                    ));
                }
            }
            (ParseOutcome::Parsed(_), ParseOutcome::Rejected) => {
                // TreeSitterParser failed but TreeSitter succeeded - THIS IS A BUG
                let errors = errors_direct.to_vec();
                failures.push(format!(
                    "Word: {}
  ERROR: TreeSitterParser failed but TreeSitter succeeded
  Errors: {:?}",
                    word, errors
                ));
            }
            (ParseOutcome::Rejected, ParseOutcome::Parsed(_)) => {
                // TreeSitter failed but TreeSitterParser succeeded - TreeSitterParser is more lenient (OK)
            }
            (ParseOutcome::Rejected, ParseOutcome::Rejected) => {
                // Both failed - not testing TreeSitterParser failures (OK)
            }
        }
    }

    if !failures.is_empty() {
        return Err(TestError::Failure(format!(
            "{} out of {} words failed equivalence test:\n\n{}\n",
            failures.len(),
            words.len(),
            failures.join("\n\n")
        )));
    }

    Ok(())
}
