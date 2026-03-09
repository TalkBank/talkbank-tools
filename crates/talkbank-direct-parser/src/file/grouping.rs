//! Tier content parsing, classification, and grouping into [`ChatFile`] lines.

use talkbank_model::ParseOutcome;
use talkbank_model::dependent_tier::DependentTier;
use talkbank_model::model::{Header, Line, MainTier, SpeakerCode, Utterance};
use talkbank_model::{ErrorCode, ErrorSink, ParseError, Severity, Span};

use crate::dependent_tier::{TierParseResult, classify_dependent_tier_parse_health};
use crate::recovery::extract_speaker_code;

use super::tier_parser::{RawTier, TierType};

/// Parsed tier with adjusted spans.
#[derive(Debug)]
pub(crate) enum ParsedTier {
    Header(Header, Span),
    MainTier(MainTier, Span),
    /// Main tier that failed content parsing but has a valid speaker code.
    /// Allows dependent tiers to still attach to the utterance.
    DegradedMainTier(MainTier, Span),
    DependentTier(DependentTier, Span),
    /// Dependent tier parsed with some items recovered (partial parse).
    DependentTierRecovered(DependentTier, Span, talkbank_model::model::ParseHealthTier),
    DependentTierParseError {
        span: Span,
        taint: Option<talkbank_model::model::ParseHealthTier>,
    },
}

/// Parse a single tier's content.
///
/// The content includes the prefix (@, *, %) and embedded continuation markers (\n\t, \r\n\t)
/// which tier parsers handle via ws_parser().
pub(crate) fn parse_tier_content(
    tier: &RawTier,
    errors: &impl ErrorSink,
) -> ParseOutcome<ParsedTier> {
    let content = tier.content.trim_end_matches('\n').trim_end_matches('\r');
    let offset = tier.offset; // Offset points to the prefix character

    match tier.tier_type {
        TierType::Header => {
            // Pass raw content - header parser expects literal tabs after colons
            match crate::header::parse_header_impl(content, offset, errors) {
                ParseOutcome::Parsed(header) => {
                    let span = Span::from_usize(tier.offset, tier.offset + tier.content.len());
                    ParseOutcome::parsed(ParsedTier::Header(header, span))
                }
                ParseOutcome::Rejected => ParseOutcome::rejected(),
            }
        }
        TierType::MainTier => {
            match crate::main_tier::parse_main_tier_impl(content, offset, errors) {
                ParseOutcome::Parsed(main) => {
                    let span = Span::from_usize(tier.offset, tier.offset + tier.content.len());
                    ParseOutcome::parsed(ParsedTier::MainTier(main, span))
                }
                ParseOutcome::Rejected => ParseOutcome::rejected(),
            }
        }
        TierType::DependentTier => {
            let span = Span::from_usize(tier.offset, tier.offset + tier.content.len());
            match crate::dependent_tier::parse_dependent_tier_internal(content, offset, errors) {
                TierParseResult::Clean(dep) => {
                    ParseOutcome::parsed(ParsedTier::DependentTier(dep, span))
                }
                TierParseResult::Recovered(dep, taint_tier) => {
                    ParseOutcome::parsed(ParsedTier::DependentTierRecovered(dep, span, taint_tier))
                }
                TierParseResult::Failed(_taint) => ParseOutcome::rejected(),
            }
        }
    }
}

/// Collect dependent tiers following a main tier into the utterance.
///
/// Handles clean, recovered, and failed dependent tier variants.
pub(crate) fn collect_dependent_tiers(
    parsed_tiers: &[ParsedTier],
    start: usize,
    utterance: &mut Utterance,
) -> usize {
    let mut j = start;
    while j < parsed_tiers.len() {
        match &parsed_tiers[j] {
            ParsedTier::DependentTier(dep, _) => {
                utterance.dependent_tiers.push(dep.clone());
                j += 1;
            }
            ParsedTier::DependentTierRecovered(dep, _, taint_tier) => {
                utterance.dependent_tiers.push(dep.clone());
                utterance.mark_parse_taint(*taint_tier);
                j += 1;
            }
            ParsedTier::DependentTierParseError { taint, .. } => {
                match taint {
                    Some(tier) => utterance.mark_parse_taint(*tier),
                    None => utterance.mark_all_dependent_alignment_taint(),
                }
                j += 1;
            }
            _ => break,
        }
    }
    j
}

/// Group parsed tiers into a ChatFile structure.
///
/// Groups main tiers with following dependent tiers into Utterances.
pub(crate) fn group_tiers_into_file(
    parsed_tiers: Vec<ParsedTier>,
    errors: &impl ErrorSink,
) -> Vec<Line> {
    let mut result = Vec::new();
    let mut i = 0;

    while i < parsed_tiers.len() {
        match &parsed_tiers[i] {
            ParsedTier::Header(header, span) => {
                result.push(Line::header_with_span(header.clone(), *span));
                i += 1;
            }
            ParsedTier::MainTier(main, _main_span) => {
                let mut utterance = Utterance::new(main.clone());
                let j = collect_dependent_tiers(&parsed_tiers, i + 1, &mut utterance);
                result.push(Line::utterance(utterance));
                i = j;
            }
            ParsedTier::DegradedMainTier(main, _span) => {
                let mut utterance = Utterance::new(main.clone());
                utterance.mark_parse_taint(talkbank_model::model::ParseHealthTier::Main);
                let j = collect_dependent_tiers(&parsed_tiers, i + 1, &mut utterance);
                result.push(Line::utterance(utterance));
                i = j;
            }
            ParsedTier::DependentTier(_, span) | ParsedTier::DependentTierRecovered(_, span, _) => {
                // Orphaned dependent tier - must be preceded by a main tier
                errors.report(ParseError::at_span(
                    ErrorCode::OrphanedDependentTier, // E404
                    Severity::Error,
                    *span,
                    "Dependent tier must follow a main tier".to_string(),
                ));
                i += 1;
            }
            ParsedTier::DependentTierParseError { span, .. } => {
                // Malformed orphaned dependent tier - parse error already reported by tier parser.
                errors.report(ParseError::at_span(
                    ErrorCode::OrphanedDependentTier, // E404
                    Severity::Error,
                    *span,
                    "Dependent tier must follow a main tier".to_string(),
                ));
                i += 1;
            }
        }
    }

    result
}

/// Process phase 2 of file parsing: parse each tier's content, with degraded recovery.
///
/// Returns `None` if a fatal tier parse failure occurs (header or unrecoverable main tier).
pub(crate) fn parse_all_tier_contents(
    raw_tiers: &[RawTier],
    errors: &impl ErrorSink,
) -> Option<Vec<ParsedTier>> {
    let mut parsed_tiers = Vec::new();
    let mut fatal_tier_parse_failed = false;
    for tier in raw_tiers {
        match parse_tier_content(tier, errors) {
            ParseOutcome::Parsed(parsed) => parsed_tiers.push(parsed),
            ParseOutcome::Rejected => match tier.tier_type {
                TierType::Header => {
                    fatal_tier_parse_failed = true;
                }
                TierType::MainTier => {
                    // Try degraded recovery: extract speaker code to create a
                    // shell utterance that dependent tiers can still attach to.
                    let content = tier.content.trim_end_matches('\n').trim_end_matches('\r');
                    let span = Span::from_usize(tier.offset, tier.offset + tier.content.len());
                    if let Some(speaker) = extract_speaker_code(content) {
                        let degraded = MainTier::new(
                            SpeakerCode::new(speaker),
                            Vec::new(),
                            None::<talkbank_model::model::Terminator>,
                        );
                        parsed_tiers.push(ParsedTier::DegradedMainTier(degraded, span));
                    } else {
                        fatal_tier_parse_failed = true;
                    }
                }
                TierType::DependentTier => {
                    let content = tier.content.trim_end_matches('\n').trim_end_matches('\r');
                    let span = Span::from_usize(tier.offset, tier.offset + tier.content.len());
                    parsed_tiers.push(ParsedTier::DependentTierParseError {
                        span,
                        taint: classify_dependent_tier_parse_health(content),
                    });
                }
            },
        }
    }

    if fatal_tier_parse_failed {
        return None;
    }

    Some(parsed_tiers)
}
