#![allow(unused_imports)]

//! Property 1: Parser Never Panics
//!
//! The parser should gracefully handle ANY input, even garbage.
//! It may return Err, but should never panic.
//!
//! Tests BOTH TreeSitterParser and TreeSitterParser to ensure both are robust.

use super::*;
use proptest::prelude::*;

proptest! {
    #![proptest_config(slow_test_config())]

    /// Parses word never panics.
    #[test]
    fn parse_word_never_panics(s in "\\PC*") {
        // Property: parse_word accepts any string and doesn't panic
        // Test BOTH parsers to ensure both are robust
        for parser in parser_suite_for_proptest().unwrap() {
            let _ = parser.parse_word(&s);  // May be Ok or Err, but shouldn't panic
        }
    }

    /// Parses word handles unicode.
    #[test]
    fn parse_word_handles_unicode(s in "[\\p{Any}]{0,50}") {
        // Property: parse_word handles unicode characters gracefully
        // Test BOTH parsers to ensure both handle unicode correctly
        for parser in parser_suite_for_proptest().unwrap() {
            let _ = parser.parse_word(&s);
        }
    }
}
