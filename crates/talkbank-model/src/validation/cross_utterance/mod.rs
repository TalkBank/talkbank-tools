//! Cross-utterance validation for quotation patterns and completion linkers
//!
//! This module validates relationships between utterances that require looking
//! at sequences of utterances, particularly:
//! - Quotation patterns (Pattern A: +"/. and Pattern B: +". )
//! - Completion linkers (+, and ++)
//!
//! ## Disabled Validations
//!
//! **NOTE**: All quotation marker cross-utterance validations are currently **DISABLED** (2025-12-28).
//!
//! ### Rationale
//! The legacy CHAT system (Java) never performed validation of these cross-utterance patterns.
//! These validations were implemented fresh in the Rust version to enforce strict CHAT
//! conventions for quoted passages and completion sequences. However, analysis of real-world
//! corpora (particularly CORAAL, OralArguments) shows that these strict sequential patterns
//! don't match natural conversational flow:
//!
//! - **E341**: Quotation follows (`+"/. `) - Requires next same-speaker utterance to have `+"` linker.
//!   Fails when speakers don't continue with quoted content or other speakers interject.
//!
//! - **E344**: Quotation precedes (`+".`) - Requires preceding same-speaker utterances with `+"` linkers.
//!   Fails when quoted content appears without preceding marked utterances.
//!
//! - **E346**: Quoted utterance linker (`+"`) - Requires upstream/downstream same-speaker
//!   utterances to end with `+"/` or `+".` respectively. Fails when attorneys interrupt
//!   or continue quoted passages in non-canonical ways.
//!
//! - **E352**: Self-completion linker (`+,`) - Requires preceding same-speaker utterance
//!   to end with `+/.` (interruption). Fails when speakers resume utterances in
//!   natural conversation without interruption markers.
//!
//! ### Decision
//! Rather than relax or modify these validations (which could introduce new bugs), we're
//! temporarily disabling them to allow corpus-wide roundtrip testing. This lets us:
//!
//! 1. Make progress on roundtrip testing across all 807 corpora
//! 2. Collect real data on how these patterns actually appear in various CHAT corpora
//! 3. Later make an informed decision on validation approach:
//!    - Should these be warnings, not errors?
//!    - Should they be context-sensitive (different rules for different corpus types)?
//!    - Should we relax the sequential pattern matching?
//!    - Should we restore the old behavior (no validation)?
//!
//! ### Files with Disabled Logic
//! - `quotation_follows.rs`: check_quotation_follows() - DISABLED (E341)
//! - `quotation_precedes.rs`: check_quotation_precedes() - DISABLED (E344)
//! - `quoted_linker.rs`: check_quoted_linker() - DISABLED (E346)
//! - `completion.rs`: check_self_completion() - DISABLED (E352)
//! - `completion.rs`: check_other_completion() - DISABLED (E349/E350)
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Utterance_Linkers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#QuotationFollows_Linker>
//! - <https://talkbank.org/0info/manuals/CHAT.html#QuotedNewLine_Terminator>
//! - <https://talkbank.org/0info/manuals/CHAT.html#OtherCompletion_Linker>
//! - <https://talkbank.org/0info/manuals/CHAT.html#SelfCompletion_Linker>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Scoped_Symbols>

mod completion;
mod helpers;
mod quotation_follows;
mod quotation_precedes;
mod quoted_linker;
mod scoped_markers;
#[cfg(test)]
mod tests;

use crate::alignment::helpers::overlap::{OverlapRegionKind, extract_overlap_info};
use crate::model::{OverlapPointKind, Terminator, Utterance, UtteranceContent};
use crate::{ErrorCollector, ErrorSink, ParseError};
use helpers::has_quoted_linker;

/// Validates cross-utterance constraints and returns collected diagnostics.
///
/// This is the allocation-friendly convenience entrypoint used by callers that
/// do not need to reuse a custom error sink. It always runs scoped-marker and
/// overlap checks, while quotation-specific checks depend on runtime context flags.
pub fn check_cross_utterance_patterns(
    utterances: &[Utterance],
    context: &crate::validation::ValidationContext,
) -> Vec<ParseError> {
    let errors = ErrorCollector::new();
    check_cross_utterance_patterns_with_sink(utterances, context, &errors);
    errors.into_vec()
}

/// Validates cross-utterance constraints using a caller-provided error sink.
///
/// This function centralizes the full rule dispatch order, including feature-
/// gated quotation checks and always-on scoped-marker balancing. The ordering
/// is intentionally deterministic so diagnostics remain stable across runs.
pub(crate) fn check_cross_utterance_patterns_with_sink(
    utterances: &[Utterance],
    context: &crate::validation::ValidationContext,
    errors: &impl ErrorSink,
) {
    for (idx, utterance) in utterances.iter().enumerate() {
        // Quotation follows pattern (Pattern A - E341)
        // Gated behind runtime flag - disabled by default
        // See module-level documentation for rationale
        if context.shared.enable_quotation_validation
            && let Some(ref term) = utterance.main.content.terminator
            && matches!(term, Terminator::QuotedNewLine { .. })
        {
            errors.report_all(quotation_follows::check_quotation_follows(utterances, idx));
        }

        // Quotation precedes pattern (Pattern B - E344)
        // Gated behind runtime flag - disabled by default
        // See module-level documentation for rationale
        if context.shared.enable_quotation_validation
            && let Some(ref term) = utterance.main.content.terminator
            && matches!(term, Terminator::QuotedPeriodSimple { .. })
        {
            errors.report_all(quotation_precedes::check_quotation_precedes(
                utterances, idx,
            ));
        }

        // Quoted utterance linker (E346)
        // Gated behind runtime flag - disabled by default
        // See module-level documentation for rationale
        if context.shared.enable_quotation_validation && has_quoted_linker(utterance) {
            errors.report_all(quoted_linker::check_quoted_linker(utterances, idx));
        }

        // Other-completion linker (++) - temporarily disabled.
        // See module-level documentation for rationale.
    }

    // Self-completion linker (E352) - O(n) batch validation
    // Gated behind runtime flag - disabled by default
    // See module-level documentation for rationale
    if context.shared.enable_quotation_validation {
        completion::check_self_completion_all(utterances, errors);
    }

    // Validate scoped markers that can span across utterances
    scoped_markers::check_long_feature_balance(utterances, errors);
    scoped_markers::check_nonvocal_balance(utterances, errors);

    // E704: Same speaker must not encode top+bottom overlap pair with self.
    check_self_overlap_markers(utterances, errors);

    // E347: Cross-utterance overlap balance — top regions should have
    // matching bottom regions on a different speaker.
    check_cross_utterance_overlap_balance(utterances, errors);
}

/// Rejects overlap pairs that imply a speaker overlapping with themself.
///
/// Adjacent utterances by the same speaker must not encode a top-overlap pair
/// followed by a bottom-overlap pair, which semantically implies self-overlap.
/// This catches annotation slips that are easy to miss when overlap brackets
/// are edited manually across turn boundaries.
fn check_self_overlap_markers(utterances: &[Utterance], errors: &impl ErrorSink) {
    use crate::{ErrorCode, ErrorContext, Severity, SourceLocation};

    for pair in utterances.windows(2) {
        let first = &pair[0];
        let second = &pair[1];

        if first.main.speaker != second.main.speaker {
            continue;
        }

        let first_has_top = has_overlap_kind(
            &first.main.content.content,
            OverlapPointKind::TopOverlapBegin,
            OverlapPointKind::TopOverlapEnd,
        );
        let second_has_bottom = has_overlap_kind(
            &second.main.content.content,
            OverlapPointKind::BottomOverlapBegin,
            OverlapPointKind::BottomOverlapEnd,
        );

        if first_has_top && second_has_bottom {
            let span = second.main.span;
            let speaker = second.main.speaker.as_str();
            errors.report(
                crate::ParseError::new(
                    ErrorCode::SpeakerSelfOverlap,
                    Severity::Error,
                    SourceLocation::new(span),
                    ErrorContext::new(speaker, span, speaker),
                    format!(
                        "Speaker '{}' has overlapping top/bottom overlap markers across adjacent utterances",
                        speaker
                    ),
                )
                .with_suggestion(
                    "Overlap markers should represent overlap between different speakers",
                ),
            );
        }
    }
}

/// Returns whether the utterance has a well-paired overlap region of the given kind.
///
/// Uses `extract_overlap_info` to check all content levels (including intra-word
/// markers). Requires both begin and end markers to be present.
fn has_overlap_kind(
    content: &[UtteranceContent],
    begin_kind: OverlapPointKind,
    _end_kind: OverlapPointKind,
) -> bool {
    use crate::alignment::helpers::overlap::{OverlapRegionKind, extract_overlap_info};

    let target_kind = match begin_kind {
        OverlapPointKind::TopOverlapBegin => OverlapRegionKind::Top,
        OverlapPointKind::BottomOverlapBegin => OverlapRegionKind::Bottom,
        _ => return false,
    };

    let info = extract_overlap_info(content);
    info.regions
        .iter()
        .any(|r| r.kind == target_kind && r.is_well_paired())
}

/// Validate cross-utterance overlap balance (E347).
///
/// For each top region (⌈...⌉) on utterance N by speaker A, checks that
/// there is a matching bottom region (⌊...⌋) with the same index on a
/// nearby utterance by a *different* speaker. Similarly, orphaned bottom
/// regions without a matching top region are flagged.
///
/// The search window is limited to ±5 utterances to keep this O(n) and
/// to avoid false matches across distant parts of the transcript.
fn check_cross_utterance_overlap_balance(utterances: &[Utterance], errors: &impl ErrorSink) {
    use crate::{ErrorCode, ErrorContext, Severity, SourceLocation};

    // Collect per-utterance overlap regions with speaker info.
    struct UttRegionInfo {
        speaker: String,
        top_regions: Vec<Option<crate::model::OverlapIndex>>,
        bottom_regions: Vec<Option<crate::model::OverlapIndex>>,
        span: crate::Span,
    }

    let mut utt_infos: Vec<UttRegionInfo> = Vec::new();
    for utt in utterances {
        let info = extract_overlap_info(&utt.main.content.content.0);
        let mut top_indices = Vec::new();
        let mut bottom_indices = Vec::new();
        for region in &info.regions {
            if region.has_begin() {
                match region.kind {
                    OverlapRegionKind::Top => top_indices.push(region.index),
                    OverlapRegionKind::Bottom => bottom_indices.push(region.index),
                }
            }
        }
        if !top_indices.is_empty() || !bottom_indices.is_empty() {
            utt_infos.push(UttRegionInfo {
                speaker: utt.main.speaker.to_string(),
                top_regions: top_indices,
                bottom_regions: bottom_indices,
                span: utt.main.span,
            });
        }
    }

    // For each top region, search nearby utterances for a matching bottom
    // region from a different speaker with the same index.
    const SEARCH_WINDOW: usize = 5;

    for (i, info) in utt_infos.iter().enumerate() {
        'top_loop: for top_idx in &info.top_regions {
            // Search forward for a matching bottom region from different speaker.
            for candidate in utt_infos.iter().skip(i + 1).take(SEARCH_WINDOW) {
                if candidate.speaker == info.speaker {
                    continue; // Same speaker — not a cross-speaker match.
                }
                if candidate.bottom_regions.contains(top_idx) {
                    continue 'top_loop; // Found a match.
                }
            }
            // No matching bottom region found.
            let index_label = match top_idx {
                Some(idx) => format!(" (index {})", idx.0),
                None => String::new(),
            };
            errors.report(
                ParseError::new(
                    ErrorCode::UnbalancedOverlap,
                    Severity::Warning,
                    SourceLocation::new(info.span),
                    ErrorContext::new(&info.speaker, info.span, &info.speaker),
                    format!(
                        "Top overlap ⌈{index_label} on speaker '{}' has no matching \
                         bottom overlap ⌊ from a different speaker within {} utterances",
                        info.speaker, SEARCH_WINDOW
                    ),
                )
                .with_suggestion(
                    "Check that the overlapping speaker's utterance has a matching ⌊ marker \
                     with the same index",
                ),
            );
        }

        // Check orphaned bottom regions (⌊ without preceding ⌈).
        'bottom_loop: for bottom_idx in &info.bottom_regions {
            // Search backward for a matching top region from different speaker.
            let start = i.saturating_sub(SEARCH_WINDOW);
            for candidate in utt_infos[start..i].iter().rev() {
                if candidate.speaker == info.speaker {
                    continue;
                }
                if candidate.top_regions.contains(bottom_idx) {
                    continue 'bottom_loop; // Found a match.
                }
            }
            // No matching top region found.
            let index_label = match bottom_idx {
                Some(idx) => format!(" (index {})", idx.0),
                None => String::new(),
            };
            errors.report(
                ParseError::new(
                    ErrorCode::UnbalancedOverlap,
                    Severity::Warning,
                    SourceLocation::new(info.span),
                    ErrorContext::new(&info.speaker, info.span, &info.speaker),
                    format!(
                        "Bottom overlap ⌊{index_label} on speaker '{}' has no matching \
                         top overlap ⌈ from a different speaker within {} utterances",
                        info.speaker, SEARCH_WINDOW
                    ),
                )
                .with_suggestion(
                    "Check that the other speaker's utterance has a matching ⌈ marker \
                     with the same index",
                ),
            );
        }
    }
}
