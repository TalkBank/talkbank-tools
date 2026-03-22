//! %gra tier parsing using pure Chumsky combinators.
//!
//! The grammatical relations tier (%gra) provides Universal Dependencies
//! syntax annotations aligned with morphological chunks in the %mor tier.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Grammatical_Relations>
//! - <https://talkbank.org/0info/manuals/CHAT.html#GrammaticalRelations_Tier>
//!
//! # Format
//!
//! Each relation has format: `index|head|relation`
//! - **index**: Position of dependent word (1-indexed)
//! - **head**: Position of parent word (0 = ROOT)
//! - **relation**: Universal Dependencies relation type
//!
//! # Examples
//!
//! Simple sentence:
//! ```text
//! %mor:  det:art|the n|dog v|bark-3S .
//! %gra:  1|2|DET 2|3|SUBJ 3|0|ROOT 4|3|PUNCT
//! ```
//!
//! Dependency tree:
//! ```text
//!        barks (3, ROOT)
//!       /    |    \
//!      /     |     \
//!   dog(2) SUBJ   .(4)
//!    |            PUNCT
//!  the(1)
//!  DET
//! ```

use crate::whitespace::ws_parser;
use chumsky::prelude::*;
use talkbank_model::ParseOutcome;
use talkbank_model::dependent_tier::{
    GraTier, GraTierType, GrammaticalRelation, GrammaticalRelationType,
};
use talkbank_model::{
    ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation, Span,
};

/// Parse a single GrammaticalRelation using chumsky combinators.
///
/// This is the entry point for the ChatParser::parse_gra_relation API.
pub fn parse_gra_relation_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<GrammaticalRelation> {
    let parser = grammatical_relation_parser();
    match parser.parse(input).into_result() {
        Ok(relation) => ParseOutcome::parsed(relation),
        Err(parse_errors) => {
            for err in parse_errors {
                let span = err.span();
                let msg = format!("Gra relation parse error: {}", err.reason());
                errors.report(ParseError::new(
                    ErrorCode::GraParseError,
                    Severity::Error,
                    SourceLocation::new(Span::from_usize(span.start + offset, span.end + offset)),
                    ErrorContext::new(
                        input,
                        Span::from_usize(span.start + offset, span.end + offset),
                        input,
                    ),
                    msg,
                ));
            }
            ParseOutcome::rejected()
        }
    }
}

/// Parse %gra tier content (without %gra:\t prefix) using chumsky combinators.
///
/// This is the entry point that integrates with ErrorSink and matches
/// the ChatParser API which expects content-only input.
pub fn parse_gra_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<GraTier> {
    parse_gra_tier_content_with_type(input, offset, errors, GraTierType::Gra)
}

/// Parse %gra tier content with explicit tier type.
///
/// This is the entry point that integrates with ErrorSink and allows
/// specifying the tier type based on the prefix.
pub fn parse_gra_tier_content_with_type(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
    tier_type: GraTierType,
) -> ParseOutcome<GraTier> {
    let (outcome, _had_errors) = parse_gra_tier_recovering(input, offset, errors, tier_type);
    outcome
}

/// Parse %gra tier content with item-level recovery.
///
/// Splits the tier into whitespace-delimited segments and parses each
/// relation independently. Good relations are collected; bad ones are
/// skipped with error reports.
pub(crate) fn parse_gra_tier_recovering(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
    tier_type: GraTierType,
) -> (ParseOutcome<GraTier>, bool) {
    use crate::recovery::split_tier_segments;

    let segments = split_tier_segments(input);
    if segments.is_empty() {
        return (ParseOutcome::rejected(), false);
    }

    let mut relations = Vec::new();
    let mut had_errors = false;
    let parser = grammatical_relation_parser();

    for seg in &segments {
        match parser.parse(seg.text).into_result() {
            Ok(relation) => relations.push(relation),
            Err(parse_errors) => {
                had_errors = true;
                for err in parse_errors {
                    let span = err.span();
                    let abs_start = seg.offset + span.start + offset;
                    let abs_end = seg.offset + span.end + offset;
                    let msg = format!("Gra tier parse error: {}", err.reason());
                    errors.report(ParseError::new(
                        ErrorCode::GraParseError,
                        Severity::Error,
                        SourceLocation::new(Span::from_usize(abs_start, abs_end)),
                        ErrorContext::new(input, Span::from_usize(abs_start, abs_end), input),
                        msg,
                    ));
                }
            }
        }
    }

    if relations.is_empty() {
        return (ParseOutcome::rejected(), had_errors);
    }

    let tier = GraTier::new(tier_type, relations)
        .with_span(Span::from_usize(offset, offset + input.len()));
    (ParseOutcome::parsed(tier), had_errors)
}

/// Parse a full %gra tier line (with %gra:\t prefix) using chumsky combinators.
///
/// Used for parsing complete tier lines from files.
#[allow(dead_code)]
pub fn parse_gra_tier_impl(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<GraTier> {
    let parser = gra_tier_parser();
    match parser.parse(input).into_result() {
        Ok(gra_tier) => ParseOutcome::parsed(gra_tier),
        Err(parse_errors) => {
            // Report chumsky errors to ErrorSink
            for err in parse_errors {
                let span = err.span();
                let msg = format!("Gra tier parse error: {}", err.reason());
                errors.report(ParseError::new(
                    ErrorCode::GraParseError,
                    Severity::Error,
                    SourceLocation::new(Span::from_usize(span.start + offset, span.end + offset)),
                    ErrorContext::new(
                        input,
                        Span::from_usize(span.start + offset, span.end + offset),
                        input,
                    ),
                    msg,
                ));
            }
            ParseOutcome::rejected()
        }
    }
}

// ============================================================================
// Gra Tier Parser - Top Level
// ============================================================================

/// Parse gra tier content only (relations), without the prefix.
///
/// This matches the ChatParser API which expects content-only input like:
/// `1|2|SUBJ 2|0|ROOT 3|2|OBJ 4|2|PUNCT`
/// Parse grammatical relations separated by whitespace.
fn gra_relations_parser<'a>()
-> impl Parser<'a, &'a str, Vec<GrammaticalRelation>, extra::Err<Rich<'a, char>>> {
    grammatical_relation_parser()
        .separated_by(ws_parser())
        .at_least(1)
        .collect::<Vec<_>>()
}

/// Parse complete gra tier: %gra:\t relations
#[allow(dead_code)]
fn gra_tier_parser<'a>() -> impl Parser<'a, &'a str, GraTier, extra::Err<Rich<'a, char>>> {
    // Parse tier type: %gra
    let tier_type = just("%gra").to(GraTierType::Gra);

    // Parse colon and tab
    let separator = just(":\t");

    tier_type
        .then_ignore(separator)
        .then(gra_relations_parser())
        .map(|(tier_type, relations)| GraTier::new(tier_type, relations))
}

// ============================================================================
// Grammatical Relation Parser
// ============================================================================

/// Parse a single grammatical relation: index|head|relation
///
/// Examples:
/// - `1|2|DET` - Word 1 is determiner of word 2
/// - `2|0|ROOT` - Word 2 is root of sentence
/// - `3|2|SUBJ` - Word 3 is subject of word 2
/// - `4|3|PUNCT` - Word 4 is punctuation attached to word 3
fn grammatical_relation_parser<'a>()
-> impl Parser<'a, &'a str, GrammaticalRelation, extra::Err<Rich<'a, char>>> {
    // Parse index (1-indexed position)
    let index = text::int(10).try_map(|s: &str, span| {
        s.parse::<usize>()
            .map_err(|_| Rich::custom(span, "Invalid index number"))
    });

    // Parse head (0 for ROOT, or 1-indexed position)
    let head = text::int(10).try_map(|s: &str, span| {
        s.parse::<usize>()
            .map_err(|_| Rich::custom(span, "Invalid head number"))
    });

    // Parse relation type (uppercase letters, may include underscore or colon)
    let relation = one_of("ABCDEFGHIJKLMNOPQRSTUVWXYZ_:-")
        .repeated()
        .at_least(1)
        .to_slice()
        .map(GrammaticalRelationType::new);

    index
        .then_ignore(just('|'))
        .then(head)
        .then_ignore(just('|'))
        .then(relation)
        .map(|((index, head), relation)| GrammaticalRelation::new(index, head, relation))
}

#[cfg(test)]
mod tests {
    use super::*;
    use talkbank_model::ErrorCollector;
    use talkbank_model::dependent_tier::GraTierType;

    /// Tests recovers good relations around bad one.
    #[test]
    fn recovers_good_relations_around_bad_one() {
        let input = "1|2|DET BROKEN 2|0|ROOT 3|2|PUNCT";
        let errors = ErrorCollector::new();
        let result = parse_gra_tier_content(input, 0, &errors);
        let tier = result.into_option().expect("should produce partial tier");
        assert_eq!(tier.relations.len(), 3);
        assert!(!errors.is_empty(), "error reported for BROKEN");
    }

    /// Tests all relations bad rejects tier.
    #[test]
    fn all_relations_bad_rejects_tier() {
        let input = "BAD1 BAD2 BAD3";
        let errors = ErrorCollector::new();
        let result = parse_gra_tier_content(input, 0, &errors);
        assert!(result.is_rejected());
        assert!(!errors.is_empty());
    }

    /// Tests clean tier reports no errors.
    #[test]
    fn clean_tier_reports_no_errors() {
        let input = "1|2|DET 2|0|ROOT 3|2|PUNCT";
        let errors = ErrorCollector::new();
        let (result, had_errors) = parse_gra_tier_recovering(input, 0, &errors, GraTierType::Gra);
        assert!(result.is_parsed());
        assert!(!had_errors);
        assert!(errors.is_empty());
    }

    /// Tests had errors flag set on recovery.
    #[test]
    fn had_errors_flag_set_on_recovery() {
        let input = "1|2|DET NOTGRA 2|0|ROOT";
        let errors = ErrorCollector::new();
        let (result, had_errors) = parse_gra_tier_recovering(input, 0, &errors, GraTierType::Gra);
        assert!(result.is_parsed());
        assert!(had_errors);
        let tier = result.into_option().unwrap();
        assert_eq!(tier.relations.len(), 2);
    }
}
