//! Property-based tests for CHAT parser
//!
//! These tests use proptest to verify properties that should hold for ALL inputs,
//! not just hand-picked examples. This finds edge cases that example-based tests miss.
//!
//! Key properties we test:
//! 1. Parser never panics (always returns Ok or Err, never crashes)
//! 2. Categories are always detected when present
//! 3. Form types are always detected when present
//! 4. Cleaned text never contains special characters
//! 5. Error messages are always non-empty and helpful
//!
//! All tests run on BOTH TreeSitterParser and DirectParser to ensure equivalence.

use proptest::prelude::*;
use talkbank_direct_parser::DirectParser;
use talkbank_model::ChatParser;
use talkbank_model::model::Word;
use talkbank_model::{ErrorCollector, ErrorSink, ParseResult};
use talkbank_parser::TreeSitterParser;
use talkbank_parser_tests::test_error::TestError;

/// Enum wrapper to allow testing both parser implementations
pub(crate) enum ParserImpl {
    TreeSitter(TreeSitterParser),
    Direct(DirectParser),
}

impl ParserImpl {
    /// Short backend label for proptest assertion context.
    pub fn name(&self) -> &'static str {
        match self {
            ParserImpl::TreeSitter(_) => "tree-sitter",
            ParserImpl::Direct(_) => "direct",
        }
    }

    /// Parse a word using the ErrorSink API
    pub fn parse_word_streaming(&self, input: &str, errors: &impl ErrorSink) -> Option<Word> {
        match self {
            ParserImpl::TreeSitter(p) => ChatParser::parse_word(p, input, 0, errors).into(),
            ParserImpl::Direct(p) => ChatParser::parse_word(p, input, 0, errors).into(),
        }
    }

    /// Parse a word using the legacy ParseResult API (for compatibility with existing tests)
    pub fn parse_word(&self, input: &str) -> ParseResult<Word> {
        let errors = ErrorCollector::new();
        match self.parse_word_streaming(input, &errors) {
            Some(word) => {
                if errors.is_empty() {
                    Ok(word)
                } else {
                    Err(talkbank_model::ParseErrors {
                        errors: errors.to_vec(),
                    })
                }
            }
            None => Err(talkbank_model::ParseErrors {
                errors: errors.to_vec(),
            }),
        }
    }
}

/// Returns both parser implementations for testing
pub(crate) fn parser_suite() -> Result<Vec<ParserImpl>, TestError> {
    let tree_sitter =
        TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let direct = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    Ok(vec![
        ParserImpl::TreeSitter(tree_sitter),
        ParserImpl::Direct(direct),
    ])
}

/// Builds both parser backends for proptest-based checks.
pub(crate) fn parser_suite_for_proptest() -> Result<Vec<ParserImpl>, TestCaseError> {
    parser_suite().map_err(|err| TestCaseError::fail(err.to_string()))
}

// Configuration for slow property tests (reduced case count)
/// Returns a reduced-case proptest config for slower parser properties.
pub(crate) fn slow_test_config() -> ProptestConfig {
    ProptestConfig {
        cases: 20, // Reduced from default 256
        ..ProptestConfig::default()
    }
}

// Test modules
mod categories;
mod cleaned_text;
mod combinations;
mod error_messages;
mod error_scenarios;
mod form_types;
mod never_panics;
mod raw_text;
mod round_trip;
mod shortening;
mod structural_roundtrip;
mod structural_roundtrip_gra;
mod structural_roundtrip_main;
mod structural_roundtrip_mor;
mod word_parsing;
