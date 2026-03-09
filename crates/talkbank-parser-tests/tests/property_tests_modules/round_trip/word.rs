//! Test module for word in `talkbank-chat`.
//!
//! These tests document expected behavior and regressions.

use rstest::rstest;
use talkbank_model::ChatParser;
use talkbank_model::ErrorCollector;
use talkbank_parser_tests::test_error::TestError;

use super::helpers::{ParserImpl, parser_suite};

/// Verifies representative word forms round-trip across both parser backends.
#[rstest]
#[case::simple("hello")]
#[case::with_form_type_b("hello@b")]
#[case::with_form_type_fp("hello@fp")]
#[case::with_shortening("te(le)phone")]
#[case::filler("&-um")]
#[case::nonword("&~gaga")]
#[case::omission("0is")]
#[case::phonological_fragment("&+fr")]
#[case::ca_pitch_up("pitch↑")]
#[case::ca_delimiter("∆faster∆")]
fn word_round_trip(#[case] input: &str) -> Result<(), TestError> {
    // Test BOTH parsers to ensure round-trip consistency
    for parser_impl in parser_suite()? {
        let errors = ErrorCollector::new();
        let word = match &parser_impl {
            ParserImpl::TreeSitter(p) => ChatParser::parse_word(p, input, 0, &errors),
            ParserImpl::Direct(p) => ChatParser::parse_word(p, input, 0, &errors),
        };

        let word = word.ok_or_else(|| {
            TestError::Failure(format!(
                "[{}] Failed to parse '{}'",
                parser_impl.name(),
                input
            ))
        })?;
        if !errors.is_empty() {
            return Err(TestError::Failure(format!(
                "[{}] Parse errors for '{}': {:?}",
                parser_impl.name(),
                input,
                errors.to_vec()
            )));
        }
        let output = word.to_chat();
        if output != input {
            return Err(TestError::Failure(format!(
                "[{}] Round-trip failed: '{}' -> '{}'",
                parser_impl.name(),
                input,
                output
            )));
        }
    }
    Ok(())
}
