//! %mor tier parsing using pure Chumsky combinators.
//!
//! The morphological tier (%mor) provides word-by-word UD-style morphological
//! annotation aligned with the main tier.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
//!
//! # Mor Item Structure
//!
//! Each mor item aligns with one main tier word and has the structure:
//! - Required main word: `POS|lemma[-Feature]*`
//! - Optional post-clitics (marked with `~`): `~POS|lemma[-Feature]*`
//!
//! # CHAT Examples
//!
//! Simple words:
//! ```text
//! pron|I verb|see det|the noun|dog .
//! ```
//!
//! With post-clitics (contraction "she's" = "she is"):
//! ```text
//! pron|she~aux|be-Fin-Ind-Pres-S3 adj|red .
//! ```

use crate::whitespace::ws_parser;
use chumsky::prelude::*;
use smallvec::SmallVec;
use talkbank_model::ParseOutcome;
use talkbank_model::dependent_tier::{
    Mor, MorFeature, MorStem, MorTier, MorTierType, MorWord, PosCategory,
};
use talkbank_model::{
    ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation, Span,
};

/// Parse a single MorWord using chumsky combinators.
///
/// This is the entry point for the ChatParser::parse_mor_word API.
pub fn parse_mor_word_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<MorWord> {
    let parser = mor_word_parser();
    match parser.parse(input).into_result() {
        Ok(mor_word) => ParseOutcome::parsed(mor_word),
        Err(parse_errors) => {
            for err in parse_errors {
                let span = err.span();
                let msg = format!("Mor word parse error: {}", err.reason());
                errors.report(ParseError::new(
                    ErrorCode::MorParseError,
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

/// Parse %mor tier content (without %mor:\t prefix) using chumsky combinators.
///
/// This is the entry point that integrates with ErrorSink and matches
/// the ChatParser API which expects content-only input.
pub fn parse_mor_tier_content(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<MorTier> {
    parse_mor_tier_content_with_type(input, offset, errors, MorTierType::Mor)
}

/// Parse %mor tier content with explicit tier type.
///
/// This is the entry point that integrates with ErrorSink and allows
/// specifying the tier type based on the prefix.
pub fn parse_mor_tier_content_with_type(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
    tier_type: MorTierType,
) -> ParseOutcome<MorTier> {
    let (outcome, _had_errors) = parse_mor_tier_recovering(input, offset, errors, tier_type);
    outcome
}

/// Parse %mor tier content with item-level recovery.
///
/// Splits the tier into whitespace-delimited segments and parses each
/// independently. Good items are collected; bad items are skipped with
/// error reports. Returns the parse outcome and whether any errors occurred.
pub(crate) fn parse_mor_tier_recovering(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
    tier_type: MorTierType,
) -> (ParseOutcome<MorTier>, bool) {
    use crate::recovery::{is_mor_terminator, split_tier_segments};

    let segments = split_tier_segments(input);
    if segments.is_empty() {
        return (ParseOutcome::rejected(), false);
    }

    let mut items = Vec::new();
    let mut terminator: Option<smol_str::SmolStr> = None;
    let mut had_errors = false;
    let parser = mor_item_parser();

    for (i, seg) in segments.iter().enumerate() {
        // Last segment might be a terminator
        if i == segments.len() - 1 && is_mor_terminator(seg.text) {
            terminator = Some(smol_str::SmolStr::from(seg.text));
            continue;
        }

        // Second-to-last might be terminator if last was already consumed
        // (not needed — terminators only appear at end)

        match parser.parse(seg.text).into_result() {
            Ok(item) => items.push(item),
            Err(parse_errors) => {
                had_errors = true;
                for err in parse_errors {
                    let span = err.span();
                    let abs_start = seg.offset + span.start + offset;
                    let abs_end = seg.offset + span.end + offset;
                    let msg = format!("Mor tier parse error: {}", err.reason());
                    errors.report(ParseError::new(
                        ErrorCode::MorParseError,
                        Severity::Error,
                        SourceLocation::new(Span::from_usize(abs_start, abs_end)),
                        ErrorContext::new(input, Span::from_usize(abs_start, abs_end), input),
                        msg,
                    ));
                }
            }
        }
    }

    if items.is_empty() && terminator.is_none() {
        return (ParseOutcome::rejected(), had_errors);
    }

    let tier = MorTier::new(tier_type, items).with_terminator(terminator);
    (ParseOutcome::parsed(tier), had_errors)
}

/// Parse a full %mor tier line (with %mor:\t prefix) using chumsky combinators.
///
/// Used for parsing complete tier lines from files.
#[allow(dead_code)]
pub fn parse_mor_tier_impl(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<MorTier> {
    let parser = mor_tier_parser();
    match parser.parse(input).into_result() {
        Ok(mor_tier) => ParseOutcome::parsed(mor_tier),
        Err(parse_errors) => {
            for err in parse_errors {
                let span = err.span();
                let msg = format!("Mor tier parse error: {}", err.reason());
                errors.report(ParseError::new(
                    ErrorCode::MorParseError,
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
// Mor Tier Parser - Top Level
// ============================================================================

/// Parse mor items and optional terminator without building the tier.
///
/// Returns (items, terminator) tuple for building tier with specific type.
fn mor_items_parser<'a>()
-> impl Parser<'a, &'a str, (Vec<Mor>, Option<smol_str::SmolStr>), extra::Err<Rich<'a, char>>> {
    // Case 1: Just a terminator (no items)
    let terminator_only = terminator_parser().map(|t| (vec![], Some(t)));

    // Case 2: Items with optional terminator
    let items_with_terminator = mor_item_parser()
        .separated_by(ws_parser())
        .at_least(1)
        .collect::<Vec<_>>()
        .then(ws_parser().ignore_then(terminator_parser()).or_not());

    // Try items first (more common), then terminator-only
    choice((items_with_terminator, terminator_only))
}

/// Parse complete mor tier: %mor:\t items terminator
#[allow(dead_code)]
fn mor_tier_parser<'a>() -> impl Parser<'a, &'a str, MorTier, extra::Err<Rich<'a, char>>> {
    let tier_type = just("%mor").to(MorTierType::Mor);
    let separator = just(":\t");

    tier_type
        .then_ignore(separator)
        .then(mor_items_parser())
        .map(|(tier_type, (items, terminator))| {
            MorTier::new(tier_type, items).with_terminator(terminator)
        })
}

// ============================================================================
// Mor Item Parser
// ============================================================================

/// Parse a single mor item: main_word[~post_clitic]*
///
/// Examples:
/// - `pron|I` - simple word
/// - `pron|she~aux|be-Fin-Ind-Pres-S3` - word with post-clitic
fn mor_item_parser<'a>() -> impl Parser<'a, &'a str, Mor, extra::Err<Rich<'a, char>>> {
    // Post-clitics: ~word
    let post_clitic = just('~').ignore_then(mor_word_parser());

    // Full mor item: main_word [~post_clitic]*
    mor_word_parser()
        .then(post_clitic.repeated().collect::<Vec<_>>())
        .map(|(main, post_clitics)| {
            let mut mor = Mor::new(main);
            if !post_clitics.is_empty() {
                mor = mor.with_post_clitics(SmallVec::from_vec(post_clitics));
            }
            mor
        })
}

// ============================================================================
// MorWord Parser
// ============================================================================

/// Parse a mor word: POS|lemma[-Feature]*
///
/// Examples:
/// - `pron|I` - pronoun
/// - `verb|go-Past` - verb with feature
/// - `noun|cookie-Plur` - noun with feature
/// - `pron|I-Prs-Nom-S1` - pronoun with multiple features
fn mor_word_parser<'a>() -> impl Parser<'a, &'a str, MorWord, extra::Err<Rich<'a, char>>> {
    // POS tag: everything up to the pipe
    let pos = none_of("| \t\n\r~-+.?!")
        .repeated()
        .at_least(1)
        .to_slice()
        .map(PosCategory::new);

    // Pipe separator
    let pipe = just('|');

    // Lemma: everything up to hyphen, space, tilde, or end
    // Note: ! is allowed in lemmas (Basque derivational boundary, e.g. partxi!se)
    let lemma = none_of("| \t\n\r~-+.?")
        .repeated()
        .at_least(1)
        .to_slice()
        .map(MorStem::new);

    // Feature: -Value (commas allowed within value, ! allowed for Basque boundaries)
    let feature = just('-').ignore_then(
        none_of("| \t\n\r~-+.?")
            .repeated()
            .at_least(1)
            .to_slice()
            .map(MorFeature::new),
    );

    pos.then_ignore(pipe)
        .then(lemma)
        .then(feature.repeated().collect::<Vec<_>>())
        .map(|((pos, lemma), features)| {
            MorWord::new(pos, lemma).with_features(SmallVec::from_vec(features))
        })
}

// ============================================================================
// Terminator Parser
// ============================================================================

/// Parse terminator: ., ?, !, or other symbols
fn terminator_parser<'a>() -> impl Parser<'a, &'a str, smol_str::SmolStr, extra::Err<Rich<'a, char>>>
{
    choice((
        just('.').to("."),
        just('?').to("?"),
        just('!').to("!"),
        just("+...").to("+..."),
        just("+//.").to("+//."),
        just("+/.").to("+/."),
        just("+//?").to("+//?"),
        just("+/?").to("+/?"),
        just("+!?").to("+!?"),
        just("+\"/.").to("+\"/."),
        just("+\".").to("+\"."),
        just("+..?").to("+..?"),
        just("+.").to("+."),
    ))
    .map(|s: &str| smol_str::SmolStr::from(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use talkbank_model::ErrorCollector;
    use talkbank_model::dependent_tier::MorTierType;

    /// Tests recovers good items around bad item.
    #[test]
    fn recovers_good_items_around_bad_item() {
        let input = "pron|I BROKEN det|a .";
        let errors = ErrorCollector::new();
        let result = parse_mor_tier_content(input, 0, &errors);
        let tier = result.into_option().expect("should produce partial tier");
        assert_eq!(tier.items.len(), 2); // pron|I and det|a
        assert_eq!(tier.terminator.as_deref(), Some("."));
        assert!(!errors.is_empty(), "error reported for BROKEN");
    }

    /// Tests all items bad rejects tier.
    #[test]
    fn all_items_bad_rejects_tier() {
        let input = "BAD1 BAD2 BAD3";
        let errors = ErrorCollector::new();
        let result = parse_mor_tier_content(input, 0, &errors);
        assert!(result.is_rejected());
        assert!(!errors.is_empty());
    }

    /// Tests single bad item with terminator recovers.
    #[test]
    fn single_bad_item_with_terminator_recovers() {
        let input = "BROKEN .";
        let errors = ErrorCollector::new();
        let result = parse_mor_tier_content(input, 0, &errors);
        let tier = result
            .into_option()
            .expect("terminator-only should succeed");
        assert!(tier.items.is_empty());
        assert_eq!(tier.terminator.as_deref(), Some("."));
        assert!(!errors.is_empty());
    }

    /// Tests clean tier reports no errors.
    #[test]
    fn clean_tier_reports_no_errors() {
        let input = "pron|I verb|see det|the noun|dog .";
        let errors = ErrorCollector::new();
        let (result, had_errors) = parse_mor_tier_recovering(input, 0, &errors, MorTierType::Mor);
        assert!(result.is_parsed());
        assert!(!had_errors);
        assert!(errors.is_empty());
        let tier = result.into_option().unwrap();
        assert_eq!(tier.items.len(), 4);
    }

    /// Tests had errors flag set on recovery.
    #[test]
    fn had_errors_flag_set_on_recovery() {
        let input = "pron|I BROKEN verb|see .";
        let errors = ErrorCollector::new();
        let (result, had_errors) = parse_mor_tier_recovering(input, 0, &errors, MorTierType::Mor);
        assert!(result.is_parsed());
        assert!(had_errors);
        let tier = result.into_option().unwrap();
        assert_eq!(tier.items.len(), 2);
    }

    /// Tests error spans are offset adjusted.
    #[test]
    fn error_spans_are_offset_adjusted() {
        // Simulate content at offset 10 (e.g., after "%mor:\t")
        let input = "pron|I BROKEN verb|see .";
        let errors = ErrorCollector::new();
        let _result = parse_mor_tier_content_with_type(input, 10, &errors, MorTierType::Mor);
        let errs = errors.into_vec();
        assert!(!errs.is_empty());
        // The BROKEN segment starts at byte 7 in input, so absolute offset = 10 + 7 = 17
        let first_err = &errs[0];
        assert!(
            first_err.location.span.start >= 17,
            "span start {} should be >= 17",
            first_err.location.span.start
        );
    }
}
