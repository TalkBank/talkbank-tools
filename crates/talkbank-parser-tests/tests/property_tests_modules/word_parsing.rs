//! Word Parsing Tests
//!
//! Tests for specific word parsing scenarios
//!
//! Tests BOTH TreeSitterParser and DirectParser to ensure parsing behavior is consistent.

use super::*;

/// Verifies overlap markers can appear inside lexical tokens.
#[test]
fn overlap_in_word() -> Result<(), TestError> {
    for parser in parser_suite()? {
        let result = parser.parse_word("hel⌈lo");
        let word = result.map_err(|err| {
            TestError::Failure(format!(
                "[{}] Should parse word with overlap point: {}",
                parser.name(),
                err
            ))
        })?;
        // Overlap markers are structural metadata and are removed from cleaned_text
        if word.cleaned_text() != "hello" {
            return Err(TestError::Failure(format!(
                "[{}] cleaned_text mismatch",
                parser.name()
            )));
        }
        if word.raw_text() != "hel⌈lo" {
            return Err(TestError::Failure(format!(
                "[{}] raw_text mismatch",
                parser.name()
            )));
        }
    }
    Ok(())
}

/// Verifies compound markers are preserved in raw token text.
#[test]
fn compound_marker() -> Result<(), TestError> {
    for parser in parser_suite()? {
        let result = parser.parse_word("ice+cream");
        let word = result.map_err(|err| {
            TestError::Failure(format!(
                "[{}] Should parse compound word: {}",
                parser.name(),
                err
            ))
        })?;
        if word.raw_text() != "ice+cream" {
            return Err(TestError::Failure(format!(
                "[{}] raw_text mismatch",
                parser.name()
            )));
        }
    }
    Ok(())
}
