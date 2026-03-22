//! Span offset correctness tests for dependent tier parsing.
//!
//! These tests verify that when the direct parser parses dependent tier
//! content with a non-zero offset, all span values in the resulting AST
//! are shifted by that offset. This catches arithmetic errors like
//! `+ with -` or `+ with *` in offset calculations.
//!
//! Mutation testing (2026-03-22) found 75 missed mutations in this area.

use talkbank_direct_parser::DirectParser;
use talkbank_model::ChatParser;
use talkbank_model::ErrorCollector;

const OFFSET: usize = 500;

fn dp() -> DirectParser {
    DirectParser::new().expect("direct parser")
}

/// Assert that a span's start is at least `offset` when parsing with
/// a non-zero offset. This catches `+ with -` mutations where spans
/// would wrap to near-zero values.
fn assert_span_shifted(span: talkbank_model::Span, offset: usize, context: &str) {
    assert!(
        span.start as usize >= offset,
        "{context}: span start ({}) should be >= offset ({offset})",
        span.start
    );
    assert!(
        span.end as usize >= offset,
        "{context}: span end ({}) should be >= offset ({offset})",
        span.end
    );
    assert!(
        span.end > span.start,
        "{context}: span end ({}) should be > start ({})",
        span.end, span.start
    );
}

/// Assert that zero-offset parsing produces spans starting near zero,
/// while non-zero-offset parsing produces spans shifted by the offset.
/// This catches `+ with *` mutations where offset=0 gives the same
/// result as correct arithmetic.
fn assert_offset_makes_difference(
    span_zero: talkbank_model::Span,
    span_offset: talkbank_model::Span,
    offset: usize,
    context: &str,
) {
    let shift = span_offset.start as isize - span_zero.start as isize;
    assert_eq!(
        shift, offset as isize,
        "{context}: span start should shift by exactly {offset}, got shift of {shift}"
    );
    let end_shift = span_offset.end as isize - span_zero.end as isize;
    assert_eq!(
        end_shift, offset as isize,
        "{context}: span end should shift by exactly {offset}, got shift of {end_shift}"
    );
}

// ============================================================================
// %mor tier span tests
// ============================================================================

#[test]
fn mor_tier_span_includes_offset() {
    let dp = dp();
    let input = "pro|I v|want det|the n|cookie .";
    let errors = ErrorCollector::new();
    let result = ChatParser::parse_mor_tier(&dp, input, OFFSET, &errors);
    let tier = result.into_option().expect("should parse");
    assert_span_shifted(tier.span, OFFSET, "mor tier");
}

#[test]
fn mor_tier_span_shift_is_exact() {
    let dp = dp();
    let input = "pro|I v|want .";
    let errors_0 = ErrorCollector::new();
    let tier_0 = ChatParser::parse_mor_tier(&dp, input, 0, &errors_0)
        .into_option().expect("parse at 0");
    let errors_n = ErrorCollector::new();
    let tier_n = ChatParser::parse_mor_tier(&dp, input, OFFSET, &errors_n)
        .into_option().expect("parse at offset");
    assert_offset_makes_difference(tier_0.span, tier_n.span, OFFSET, "mor tier");
}

// ============================================================================
// %gra tier span tests
// ============================================================================

#[test]
fn gra_tier_span_includes_offset() {
    let dp = dp();
    let input = "1|2|SUBJ 2|0|ROOT 3|2|OBJ .";
    let errors = ErrorCollector::new();
    let result = ChatParser::parse_gra_tier(&dp, input, OFFSET, &errors);
    let tier = result.into_option().expect("should parse");
    assert_span_shifted(tier.span, OFFSET, "gra tier");
}

#[test]
fn gra_tier_span_shift_is_exact() {
    let dp = dp();
    let input = "1|2|SUBJ 2|0|ROOT .";
    let errors_0 = ErrorCollector::new();
    let tier_0 = ChatParser::parse_gra_tier(&dp, input, 0, &errors_0)
        .into_option().expect("parse at 0");
    let errors_n = ErrorCollector::new();
    let tier_n = ChatParser::parse_gra_tier(&dp, input, OFFSET, &errors_n)
        .into_option().expect("parse at offset");
    assert_offset_makes_difference(tier_0.span, tier_n.span, OFFSET, "gra tier");
}

#[test]
fn gra_relation_parses_with_offset() {
    // GrammaticalRelation doesn't carry a span (it stores index/head/relation),
    // but parsing with offset should succeed and not corrupt the indices.
    let dp = dp();
    let input = "1|2|SUBJ";
    let errors = ErrorCollector::new();
    let result = ChatParser::parse_gra_relation(&dp, input, OFFSET, &errors);
    let rel = result.into_option().expect("should parse");
    assert_eq!(rel.index, 1);
    assert_eq!(rel.head, 2);
    assert!(errors.into_vec().is_empty());
}

// ============================================================================
// %pho tier span tests
// ============================================================================

#[test]
fn pho_tier_span_includes_offset() {
    let dp = dp();
    let input = "aI want D@ kUkI";
    let errors = ErrorCollector::new();
    let result = ChatParser::parse_pho_tier(&dp, input, OFFSET, &errors);
    let tier = result.into_option().expect("should parse");
    assert_span_shifted(tier.span, OFFSET, "pho tier");
}

#[test]
fn pho_tier_span_shift_is_exact() {
    let dp = dp();
    let input = "aI want";
    let errors_0 = ErrorCollector::new();
    let tier_0 = ChatParser::parse_pho_tier(&dp, input, 0, &errors_0)
        .into_option().expect("parse at 0");
    let errors_n = ErrorCollector::new();
    let tier_n = ChatParser::parse_pho_tier(&dp, input, OFFSET, &errors_n)
        .into_option().expect("parse at offset");
    assert_offset_makes_difference(tier_0.span, tier_n.span, OFFSET, "pho tier");
}

// ============================================================================
// %mor word (individual item) span tests
// ============================================================================

#[test]
fn mor_word_span_includes_offset() {
    let dp = dp();
    let input = "pro|I";
    let errors = ErrorCollector::new();
    let result = ChatParser::parse_mor_word(&dp, input, OFFSET, &errors);
    let item = result.into_option().expect("should parse");
    // MorWord doesn't have its own span, but the parse should succeed
    // with offset and not panic
    assert!(errors.into_vec().is_empty(), "no errors expected");
    let _ = item; // just verify it parsed
}

// ============================================================================
// Recovery path span tests (error offsets)
// ============================================================================

#[test]
fn mor_tier_recovery_error_spans_include_offset() {
    let dp = dp();
    let input = "pro|I BROKEN_MOR v|want .";
    let errors = ErrorCollector::new();
    let _result = ChatParser::parse_mor_tier(&dp, input, OFFSET, &errors);
    let err_vec = errors.into_vec();
    for err in &err_vec {
        let loc = &err.location;
        assert!(
            loc.span.start as usize >= OFFSET,
            "error span start ({}) should be >= offset ({OFFSET})",
            loc.span.start
        );
    }
}

#[test]
fn gra_tier_recovery_error_spans_include_offset() {
    let dp = dp();
    let input = "1|2|SUBJ NOTGRA 2|0|ROOT .";
    let errors = ErrorCollector::new();
    let _result = ChatParser::parse_gra_tier(&dp, input, OFFSET, &errors);
    let err_vec = errors.into_vec();
    for err in &err_vec {
        let loc = &err.location;
        assert!(
            loc.span.start as usize >= OFFSET,
            "error span start ({}) should be >= offset ({OFFSET})",
            loc.span.start
        );
    }
}

// ============================================================================
// Main tier span tests
// ============================================================================

#[test]
fn main_tier_span_includes_offset() {
    let dp = dp();
    let input = "*CHI:\thello world .";
    let errors = ErrorCollector::new();
    let result = ChatParser::parse_main_tier(&dp, input, OFFSET, &errors);
    let tier = result.into_option().expect("should parse");
    assert_span_shifted(tier.span, OFFSET, "main tier");
}

#[test]
fn main_tier_span_shift_is_exact() {
    let dp = dp();
    let input = "*CHI:\thello world .";
    let errors_0 = ErrorCollector::new();
    let tier_0 = ChatParser::parse_main_tier(&dp, input, 0, &errors_0)
        .into_option().expect("parse at 0");
    let errors_n = ErrorCollector::new();
    let tier_n = ChatParser::parse_main_tier(&dp, input, OFFSET, &errors_n)
        .into_option().expect("parse at offset");
    assert_offset_makes_difference(tier_0.span, tier_n.span, OFFSET, "main tier");
}

// ============================================================================
// Recovery segment boundary tests
// ============================================================================

#[test]
fn recovery_splitting_boundary_conditions() {
    // These test the recovery.rs < vs <= and > vs >= boundaries
    // that mutation testing flagged (5 mutations).
    let dp = dp();

    // Single-item tier: boundary is the whole content
    let single_mor = "pro|I .";
    let errors = ErrorCollector::new();
    let result = ChatParser::parse_mor_tier(&dp, single_mor, 0, &errors);
    let tier = result.into_option().expect("single item should parse");
    assert_eq!(tier.items.len(), 1, "should have exactly 1 item");

    // Empty content after tier marker should not crash
    let empty = "";
    let errors = ErrorCollector::new();
    let _ = ChatParser::parse_mor_tier(&dp, empty, 0, &errors);
    // Just verify no panic
}
