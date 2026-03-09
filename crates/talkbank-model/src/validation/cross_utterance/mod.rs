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

/// Returns whether both overlap endpoints of a given kind-pair are present.
///
/// Used to detect complete top/bottom overlap pair signatures in one utterance.
/// Partial markers are ignored here and handled by other validation rules.
fn has_overlap_kind(
    content: &[UtteranceContent],
    begin_kind: OverlapPointKind,
    end_kind: OverlapPointKind,
) -> bool {
    let mut has_begin = false;
    let mut has_end = false;

    for item in content {
        if let UtteranceContent::OverlapPoint(point) = item {
            if point.kind == begin_kind {
                has_begin = true;
            } else if point.kind == end_kind {
                has_end = true;
            }
        }
    }

    has_begin && has_end
}
