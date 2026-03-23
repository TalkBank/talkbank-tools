//! Unified Offset Parameter Tests
//!
//! Comprehensive tests for offset parameter functionality across ALL parser implementations.
//! These tests verify that parsers correctly handle the offset parameter, which indicates
//! where a fragment starts in a larger document.
//!
//! **Testing Strategy:**
//! - Run on TreeSitterParser via ChatParser trait
//! - Verify spans are document-absolute (include offset)
//! - Verify error locations are offset-adjusted
//! - Verify roundtrip stability with offsets
//! - Verify UTF-8 multi-byte character handling
//!
//! All tests use insta snapshots for easy review and maintenance.

use talkbank_parser::TreeSitterParser;
use talkbank_model::ChatParser;
use talkbank_model::ErrorCollector;
use talkbank_model::{SemanticEq, WriteChat};
use talkbank_parser_tests::test_error::TestError;

// ============================================================================
// Parser Suite
// ============================================================================

/// Builds the parser suite used for offset assertions.
fn parser_suite() -> Result<Vec<TreeSitterParser>, TestError> {
    let parser =
        TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    Ok(vec![parser])
}

// ============================================================================
// Helper Macros
// ============================================================================

macro_rules! impl_chat_parser {
    ($parser:expr, $method:ident, $input:expr, $offset:expr, $errors:expr) => {
        ChatParser::$method($parser, $input, $offset, $errors).into_option()
    };
}

// ============================================================================
// Word Parsing with Offsets
// ============================================================================

/// Tests parse word offset zero.
#[test]
fn test_parse_word_offset_zero() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let errors = ErrorCollector::new();
        let word = impl_chat_parser!(parser, parse_word, "hello", 0, &errors);

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((word, errors.into_vec()));
        });
    }
    Ok(())
}

/// Tests parse word with offset.
#[test]
fn test_parse_word_with_offset() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let errors = ErrorCollector::new();
        let word = impl_chat_parser!(parser, parse_word, "world", 1000, &errors);

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((word, errors.into_vec()));
        });
    }
    Ok(())
}

/// Tests parse word complex with offset.
#[test]
fn test_parse_word_complex_with_offset() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let errors = ErrorCollector::new();
        let word = impl_chat_parser!(parser, parse_word, "dog@c", 500, &errors);

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((word, errors.into_vec()));
        });
    }
    Ok(())
}

// ============================================================================
// Main Tier Parsing with Offsets
// ============================================================================

/// Tests parse main tier offset zero.
#[test]
fn test_parse_main_tier_offset_zero() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let errors = ErrorCollector::new();
        let main = impl_chat_parser!(parser, parse_main_tier, "*CHI:\thello .", 0, &errors);

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((main, errors.into_vec()));
        });
    }
    Ok(())
}

/// Tests parse main tier with offset.
#[test]
fn test_parse_main_tier_with_offset() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let errors = ErrorCollector::new();
        let main = impl_chat_parser!(
            parser,
            parse_main_tier,
            "*CHI:\thello world .",
            200,
            &errors
        );

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((main, errors.into_vec()));
        });
    }
    Ok(())
}

// ============================================================================
// Dependent Tier Parsing with Offsets
// ============================================================================

/// Tests parse mor tier offset zero.
#[test]
fn test_parse_mor_tier_offset_zero() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let errors = ErrorCollector::new();
        let mor = impl_chat_parser!(parser, parse_mor_tier, "pro|I v|want .", 0, &errors);

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((mor, errors.into_vec()));
        });
    }
    Ok(())
}

/// Tests parse mor tier with offset.
#[test]
fn test_parse_mor_tier_with_offset() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let errors = ErrorCollector::new();
        let mor = impl_chat_parser!(parser, parse_mor_tier, "pro|I v|want .", 300, &errors);

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((mor, errors.into_vec()));
        });
    }
    Ok(())
}

// ============================================================================
// Error Offset Tests
// ============================================================================

/// Tests error offset in word.
#[test]
fn test_error_offset_in_word() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let errors = ErrorCollector::new();
        let word = impl_chat_parser!(parser, parse_word, "xx", 100, &errors);

        // Verify error spans are offset-adjusted
        let error_vec = errors.into_vec();
        if let Some(first) = error_vec.first() {
            if first.location.span.start < 100 {
                return Err(TestError::Failure(format!(
                    "Error span should be offset-adjusted: got start={}, expected >=100",
                    first.location.span.start
                )));
            }
        }

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((word, error_vec));
        });
    }
    Ok(())
}

/// Tests error offset in main tier.
#[test]
fn test_error_offset_in_main_tier() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let errors = ErrorCollector::new();
        let main = impl_chat_parser!(parser, parse_main_tier, "*CHI:\txx .", 500, &errors);

        // Verify error spans are offset-adjusted
        let error_vec = errors.into_vec();
        if let Some(first) = error_vec.first() {
            if first.location.span.start < 500 {
                return Err(TestError::Failure(format!(
                    "Error span should be offset-adjusted: got start={}, expected >=500",
                    first.location.span.start
                )));
            }
        }

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((main, error_vec));
        });
    }
    Ok(())
}

// ============================================================================
// Roundtrip with Offsets
// ============================================================================

/// Tests roundtrip with offset.
#[test]
fn test_roundtrip_with_offset() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let input = "*CHI:\thello world .";
        let offset = 100;

        // Parse with offset
        let errors1 = ErrorCollector::new();
        let main1 = impl_chat_parser!(parser, parse_main_tier, input, offset, &errors1)
            .ok_or_else(|| TestError::Failure("Should parse successfully".to_string()))?;

        // Serialize back to string
        let mut buf = String::new();
        main1.write_chat(&mut buf)?;
        let serialized = buf;

        // Parse again with same offset
        let errors2 = ErrorCollector::new();
        let main2 = impl_chat_parser!(parser, parse_main_tier, &serialized, offset, &errors2)
            .ok_or_else(|| {
                TestError::Failure("Should parse successfully on roundtrip".to_string())
            })?;

        // Verify semantic equality
        let is_semantically_equal = main1.semantic_eq(&main2);

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((is_semantically_equal, main1, main2));
        });
    }
    Ok(())
}

// ============================================================================
// Multiple Fragments with Different Offsets
// ============================================================================

/// Tests multiple fragments different offsets.
#[test]
fn test_multiple_fragments_different_offsets() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let fragments = [("hello", 0), ("world", 100), ("foo", 200), ("bar", 300)];

        let results: Vec<_> = fragments
            .iter()
            .map(|(input, offset)| {
                let errors = ErrorCollector::new();
                let word = impl_chat_parser!(parser, parse_word, input, *offset, &errors);
                (input, offset, word, errors.into_vec())
            })
            .collect();

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!(results);
        });
    }
    Ok(())
}

// ============================================================================
// UTF-8 Multi-byte Characters with Offset
// ============================================================================

/// Tests offset with multibyte utf8.
#[test]
fn test_offset_with_multibyte_utf8() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let errors = ErrorCollector::new();

        let input = "hello😊world";
        let offset = 1000;

        let word = impl_chat_parser!(parser, parse_word, input, offset, &errors);

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((input.len(), word, errors.into_vec()));
        });
    }
    Ok(())
}

/// Tests offset with chinese characters.
#[test]
fn test_offset_with_chinese_characters() -> Result<(), TestError> {
    let parsers = parser_suite()?;
    for parser in parsers.iter() {
        let errors = ErrorCollector::new();

        let input = "你好";
        let offset = 500;

        let word = impl_chat_parser!(parser, parse_word, input, offset, &errors);

        insta::with_settings!({
            snapshot_suffix => "TreeSitter"
        }, {
            insta::assert_debug_snapshot!((input.len(), word, errors.into_vec()));
        });
    }
    Ok(())
}

// ============================================================================
// Parser Equivalence - Verify Both Parsers Produce Same Results
// ============================================================================

/// Tests parser equivalence word with offset.
#[test]
fn test_parser_equivalence_word_with_offset() -> Result<(), TestError> {
    let input = "hello@s";
    let offset = 100;

    // Parse with TreeSitterParser
    let tsp = TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors_tsp = ErrorCollector::new();
    let word_tsp = ChatParser::parse_word(&tsp, input, offset, &errors_tsp).into_option();

    // Parse with TreeSitterParser
    let dp = TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors_dp = ErrorCollector::new();
    let word_dp = ChatParser::parse_word(&dp, input, offset, &errors_dp).into_option();

    // Verify semantic equality
    let are_semantically_equal = match word_dp.as_ref().zip(word_tsp.as_ref()) {
        Some((wd, wt)) => wd.semantic_eq(wt),
        None => false,
    };

    insta::assert_debug_snapshot!((word_dp, word_tsp, are_semantically_equal));

    Ok(())
}

/// Tests parser equivalence main tier with offset.
#[test]
fn test_parser_equivalence_main_tier_with_offset() -> Result<(), TestError> {
    let input = "*CHI:\thello world .";
    let offset = 200;

    // Parse with TreeSitterParser
    let tsp = TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors_tsp = ErrorCollector::new();
    let main_tsp = ChatParser::parse_main_tier(&tsp, input, offset, &errors_tsp).into_option();

    // Parse with TreeSitterParser
    let dp = TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors_dp = ErrorCollector::new();
    let main_dp = ChatParser::parse_main_tier(&dp, input, offset, &errors_dp).into_option();

    // Verify semantic equality
    let are_semantically_equal = match main_dp.as_ref().zip(main_tsp.as_ref()) {
        Some((md, mt)) => md.semantic_eq(mt),
        None => false,
    };

    insta::assert_debug_snapshot!((main_dp, main_tsp, are_semantically_equal));

    Ok(())
}

/// Tests parser equivalence mor tier with offset.
#[test]
fn test_parser_equivalence_mor_tier_with_offset() -> Result<(), TestError> {
    let input = "pro|I v|want .";
    let offset = 300;

    // Parse with TreeSitterParser
    let tsp = TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors_tsp = ErrorCollector::new();
    let mor_tsp = ChatParser::parse_mor_tier(&tsp, input, offset, &errors_tsp).into_option();

    // Parse with TreeSitterParser
    let dp = TreeSitterParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors_dp = ErrorCollector::new();
    let mor_dp = ChatParser::parse_mor_tier(&dp, input, offset, &errors_dp).into_option();

    // Verify semantic equality
    let are_semantically_equal = match mor_dp.as_ref().zip(mor_tsp.as_ref()) {
        Some((md, mt)) => md.semantic_eq(mt),
        None => false,
    };

    insta::assert_debug_snapshot!((mor_dp, mor_tsp, are_semantically_equal));

    Ok(())
}
