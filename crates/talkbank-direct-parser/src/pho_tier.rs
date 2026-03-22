//! %pho tier parsing using pure Chumsky combinators.
//!
//! The phonological tier (%pho, %mod) provides phonetic transcription
//! aligned with words in the main tier, using IPA or UNIBET notation.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Phonology>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Phonology_Tier>
//!
//! # Format
//!
//! Each %pho tier contains space-separated phonological items:
//! - **Simple words**: phonetic tokens (e.g., `ˈɑmɪ`, `hɛˈloʊ`)
//! - **Groups**: Multiple words in ‹ › brackets (e.g., `‹a b›`)
//!
//! # Examples
//!
//! Simple phonological transcription:
//! ```text
//! *CHI: hello there .
//! %pho: hɛˈloʊ ðɛr .
//! ```
//!
//! With groups (multiple phonological words for one main tier word):
//! ```text
//! *CHI: goodbye .
//! %pho: ‹gʊd baɪ› .
//! ```

use crate::whitespace::ws_parser;
use chumsky::prelude::*;
use talkbank_model::ParseOutcome;
use talkbank_model::dependent_tier::{PhoGroupWords, PhoItem, PhoTier, PhoTierType, PhoWord};
use talkbank_model::{
    ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation, Span,
};

/// Parse a single PhoWord using chumsky combinators.
///
/// This is the entry point for the ChatParser::parse_pho_word API.
pub fn parse_pho_word_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<PhoWord> {
    let parser = pho_word_parser();
    match parser.parse(input).into_result() {
        Ok(pho_word) => ParseOutcome::parsed(pho_word),
        Err(parse_errors) => {
            for err in parse_errors {
                let span = err.span();
                let msg = format!("Pho word parse error: {}", err.reason());
                errors.report(ParseError::new(
                    ErrorCode::PhoParseError,
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

/// Parse %pho tier content (without %pho:\t prefix) using chumsky combinators.
///
/// This is the entry point that integrates with ErrorSink and matches
/// the ChatParser API which expects content-only input.
pub fn parse_pho_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<PhoTier> {
    parse_pho_tier_content_with_type(input, offset, errors, PhoTierType::Pho)
}

/// Parse phonology tier content with a specific tier type.
///
/// Used by dependent_tier dispatcher to parse %pho, %mod, etc. with correct tier type.
pub fn parse_pho_tier_content_with_type(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
    tier_type: PhoTierType,
) -> ParseOutcome<PhoTier> {
    let (outcome, _had_errors) = parse_pho_tier_recovering(input, offset, errors, tier_type);
    outcome
}

/// Parse %pho tier content with item-level recovery.
///
/// Splits the tier into segments (treating `‹...›` groups as atomic) and
/// parses each item independently. Good items are collected; bad ones are
/// skipped with error reports.
pub(crate) fn parse_pho_tier_recovering(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
    tier_type: PhoTierType,
) -> (ParseOutcome<PhoTier>, bool) {
    use crate::recovery::split_pho_tier_segments;

    let segments = split_pho_tier_segments(input);
    if segments.is_empty() {
        return (ParseOutcome::rejected(), false);
    }

    let mut items = Vec::new();
    let mut had_errors = false;
    let parser = pho_item_parser();

    for seg in &segments {
        match parser.parse(seg.text).into_result() {
            Ok(item) => items.push(item),
            Err(parse_errors) => {
                had_errors = true;
                for err in parse_errors {
                    let span = err.span();
                    let abs_start = seg.offset + span.start + offset;
                    let abs_end = seg.offset + span.end + offset;
                    let msg = format!("Pho tier parse error: {}", err.reason());
                    errors.report(ParseError::new(
                        ErrorCode::PhoParseError,
                        Severity::Error,
                        SourceLocation::new(Span::from_usize(abs_start, abs_end)),
                        ErrorContext::new(input, Span::from_usize(abs_start, abs_end), input),
                        msg,
                    ));
                }
            }
        }
    }

    if items.is_empty() {
        return (ParseOutcome::rejected(), had_errors);
    }

    let tier = PhoTier::new(tier_type, items)
        .with_span(Span::from_usize(offset, offset + input.len()));
    (ParseOutcome::parsed(tier), had_errors)
}

/// Parse a full %pho tier line (with %pho:\t prefix) using chumsky combinators.
///
/// Used for parsing complete tier lines from files.
#[allow(dead_code)]
pub fn parse_pho_tier_impl(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<PhoTier> {
    let parser = pho_tier_parser();
    match parser.parse(input).into_result() {
        Ok(pho_tier) => ParseOutcome::parsed(pho_tier),
        Err(parse_errors) => {
            // Report chumsky errors to ErrorSink
            for err in parse_errors {
                let span = err.span();
                let msg = format!("Pho tier parse error: {}", err.reason());
                errors.report(ParseError::new(
                    ErrorCode::PhoParseError,
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
// Pho Tier Parser - Top Level
// ============================================================================

/// Parse pho tier content only (items), without the prefix.
///
/// This matches the ChatParser API which expects content-only input like:
/// `hɛˈloʊ ðɛr .`
#[allow(dead_code)]
fn pho_content_parser<'a>() -> impl Parser<'a, &'a str, PhoTier, extra::Err<Rich<'a, char>>> {
    // Parse phonological items separated by whitespace
    let items = pho_item_parser()
        .separated_by(ws_parser())
        .at_least(1)
        .collect::<Vec<_>>();

    items.map(|items| {
        // Default to PhoTierType::Pho since we don't have the prefix
        PhoTier::new(PhoTierType::Pho, items)
    })
}

/// Parse complete pho tier: %pho:\t items
#[allow(dead_code)]
fn pho_tier_parser<'a>() -> impl Parser<'a, &'a str, PhoTier, extra::Err<Rich<'a, char>>> {
    // Parse tier type: %pho or %mod
    let tier_type = choice((
        just("%pho").to(PhoTierType::Pho),
        just("%mod").to(PhoTierType::Mod),
    ));

    // Parse colon and tab
    let separator = just(":\t");

    // Parse phonological items separated by whitespace
    let items = pho_item_parser()
        .separated_by(ws_parser())
        .at_least(1)
        .collect::<Vec<_>>();

    tier_type
        .then_ignore(separator)
        .then(items)
        .map(|(tier_type, items)| PhoTier::new(tier_type, items))
}

// ============================================================================
// Pho Item Parser
// ============================================================================

/// Parse a single phonological item: either a simple word or a group
///
/// Examples:
/// - `ˈɑmɪ` - Simple phonological word
/// - `hɛˈloʊ` - Simple phonological word
/// - `‹gʊd baɪ›` - Phonological group (multiple words for one main tier word)
fn pho_item_parser<'a>() -> impl Parser<'a, &'a str, PhoItem, extra::Err<Rich<'a, char>>> {
    choice((
        pho_group_parser().map(PhoItem::Group),
        pho_word_parser().map(PhoItem::Word),
    ))
}

// ============================================================================
// Pho Word Parser
// ============================================================================

/// Parse a simple phonological word
///
/// A phonological word is any sequence of non-whitespace, non-bracket characters.
/// Examples: `ˈɑmɪ`, `hɛˈloʊ`, `ðɛr`, `.`
fn pho_word_parser<'a>() -> impl Parser<'a, &'a str, PhoWord, extra::Err<Rich<'a, char>>> {
    // Match any non-whitespace, non-bracket character
    none_of(" \t\n\r\u{2039}\u{203A}")
        .repeated()
        .at_least(1)
        .to_slice()
        .map(PhoWord::new)
}

// ============================================================================
// Pho Group Parser
// ============================================================================

/// Parse a phonological group: ‹words›
///
/// Groups use special Unicode brackets:
/// - Opening: ‹ (U+2039 SINGLE LEFT-POINTING ANGLE QUOTATION MARK)
/// - Closing: › (U+203A SINGLE RIGHT-POINTING ANGLE QUOTATION MARK)
///
/// Examples:
/// - `‹gʊd baɪ›` - Two phonological words
/// - `‹a b c›` - Three phonological words
fn pho_group_parser<'a>() -> impl Parser<'a, &'a str, PhoGroupWords, extra::Err<Rich<'a, char>>> {
    // Opening bracket: ‹ (U+2039)
    let open = just('\u{2039}');

    // Closing bracket: › (U+203A)
    let close = just('\u{203A}');

    // Parse space-separated phonological words inside the group
    let words = pho_word_parser()
        .separated_by(ws_parser())
        .at_least(1)
        .collect::<Vec<_>>();

    open.ignore_then(words)
        .then_ignore(close)
        .map(PhoGroupWords::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use talkbank_model::ErrorCollector;
    use talkbank_model::dependent_tier::PhoTierType;

    /// Tests clean pho tier no errors.
    #[test]
    fn clean_pho_tier_no_errors() {
        let input = "hɛˈloʊ ðɛr";
        let errors = ErrorCollector::new();
        let (result, had_errors) = parse_pho_tier_recovering(input, 0, &errors, PhoTierType::Pho);
        assert!(result.is_parsed());
        assert!(!had_errors);
        assert!(errors.is_empty());
        let tier = result.into_option().unwrap();
        assert_eq!(tier.items.len(), 2);
    }

    /// Tests pho tier with group.
    #[test]
    fn pho_tier_with_group() {
        let input = "hɛˈloʊ ‹gʊd baɪ›";
        let errors = ErrorCollector::new();
        let result = parse_pho_tier_content(input, 0, &errors);
        let tier = result.into_option().expect("should parse group");
        assert_eq!(tier.items.len(), 2);
        assert!(errors.is_empty());
    }

    /// Tests empty input rejects.
    #[test]
    fn empty_input_rejects() {
        let input = "";
        let errors = ErrorCollector::new();
        let result = parse_pho_tier_content(input, 0, &errors);
        assert!(result.is_rejected());
    }
}
