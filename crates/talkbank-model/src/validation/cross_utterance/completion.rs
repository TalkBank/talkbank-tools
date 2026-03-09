//! Cross-utterance validation patterns for completion linkers
//!
//! ## Completion Patterns
//!
//! - **Self-completion (`+,`)**: Resumes an interrupted utterance by the same speaker.
//!   Requires preceding utterance from same speaker to end with interruption terminator (`+/.`).
//!
//! - **Other-completion (`++`)**: Completes another speaker's incomplete utterance.
//!   Requires preceding utterance from different speaker to end with trailing-off terminator (`+...`).
//!
//! ## Performance Optimization (2025-12-29)
//!
//! Self-completion validation was optimized from O(n²) to O(n) using a stack-based algorithm:
//!
//! **Before** (O(n²)): Each utterance with `+,` searched backward through all prior
//! same-speaker utterances to find matching `+/.` terminator.
//!
//! **After** (O(n)): Single forward pass maintains per-speaker stacks of interruption indices.
//! When `+,` is encountered, we pop from that speaker's stack for instant O(1) match.
//!
//! This is critical for large conversational files with many completion patterns.
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Utterance_Linkers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#QuotationFollows_Linker>
//! - <https://talkbank.org/0info/manuals/CHAT.html#QuotedNewLine_Terminator>
//! - <https://talkbank.org/0info/manuals/CHAT.html#OtherCompletion_Linker>
//! - <https://talkbank.org/0info/manuals/CHAT.html#SelfCompletion_Linker>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Scoped_Symbols>

use crate::model::{Linker, Terminator, Utterance};
use crate::{ErrorCode, ErrorContext, ErrorSink, ParseError, Severity, SourceLocation};
use std::collections::HashMap;

/// Validate all `+,` self-completion linkers in one forward pass.
///
/// This replaces the O(n²) per-utterance backward search with a stack-based
/// approach that processes all utterances in a single forward pass.
pub(super) fn check_self_completion_all(utterances: &[Utterance], errors: &impl ErrorSink) {
    // Stack of interruption indices per speaker
    let mut interruption_stacks: HashMap<&str, Vec<usize>> = HashMap::new();

    for (idx, utterance) in utterances.iter().enumerate() {
        let speaker = utterance.main.speaker.as_str();

        // Check if this has self-completion linker (+,)
        if has_self_completion_linker_internal(utterance) {
            // Try to match with previous interruption from same speaker
            match interruption_stacks.get_mut(speaker) {
                Some(stack) if !stack.is_empty() => {
                    if let Some(prev_idx) = stack.pop() {
                        let prev_utt = &utterances[prev_idx];

                        // Verify it ended with interruption terminator
                        let has_interruption = match prev_utt.main.content.terminator.as_ref() {
                            Some(term) => matches!(term, Terminator::Interruption { .. }),
                            None => false,
                        };

                        if !has_interruption {
                            errors.report(
                                ParseError::new(
                                    ErrorCode::MissingQuoteEnd,
                                    Severity::Error,
                                    SourceLocation::new(utterance.main.span),
                                    ErrorContext::new(
                                        format!("*{}: +, ...", speaker),
                                        utterance.main.span,
                                        "self-completion linker",
                                    ),
                                    format!(
                                        "Self-completion linker (+,) but preceding same-speaker utterance doesn't end with +/. (interruption terminator) from speaker {}",
                                        speaker
                                    ),
                                )
                                .with_suggestion("Change the preceding utterance terminator to +/. to mark it as interrupted")
                            );
                        }
                    }
                }
                _ => {
                    // No prior utterance from this speaker
                    errors.report(
                        ParseError::new(
                            ErrorCode::MissingQuoteBegin,
                            Severity::Error,
                            SourceLocation::new(utterance.main.span),
                            ErrorContext::new(
                                format!("*{}: +, ...", speaker),
                                utterance.main.span,
                                "self-completion linker",
                            ),
                            format!(
                                "Self-completion linker (+,) without any preceding utterance from same speaker ({})",
                                speaker
                            ),
                        )
                        .with_suggestion("Self-completion is used to resume an interrupted utterance; ensure there's a prior interrupted utterance with +/. terminator")
                    );
                }
            }
        }

        // Check if this utterance ends with interruption terminator (+/.)
        if let Some(ref term) = utterance.main.content.terminator
            && matches!(term, Terminator::Interruption { .. })
        {
            interruption_stacks.entry(speaker).or_default().push(idx);
        }
    }
}

/// Returns whether an utterance includes the self-completion linker (`+,`).
fn has_self_completion_linker_internal(utterance: &Utterance) -> bool {
    utterance
        .main
        .content
        .linkers
        .iter()
        .any(|linker| matches!(linker, Linker::SelfCompletion))
}

/// Legacy per-utterance validation (O(n) per call, O(n²) total).
///
/// Kept for backward compatibility but prefer `check_self_completion_all`.
#[allow(dead_code)]
pub(super) fn check_self_completion(utterances: &[Utterance], idx: usize) -> Vec<ParseError> {
    let mut errors = Vec::new();
    let utterance = &utterances[idx];
    let speaker = utterance.main.speaker.as_str();

    // Find most recent utterance by same speaker
    let prev_same_speaker = utterances[..idx]
        .iter()
        .rev()
        .find(|u| u.main.speaker.as_str() == speaker);

    match prev_same_speaker {
        None => {
            errors.push(
                ParseError::new(
                    ErrorCode::MissingQuoteBegin,
                    Severity::Error,
                    SourceLocation::new(utterance.main.span),
                    ErrorContext::new(
                        format!("*{}: +, ...", speaker),
                        utterance.main.span,
                        "self-completion linker",
                    ),
                    format!(
                        "Self-completion linker (+,) without any preceding utterance from same speaker ({})",
                        speaker
                    ),
                )
                .with_suggestion("Self-completion is used to resume an interrupted utterance; ensure there's a prior interrupted utterance with +/. terminator")
            );
        }
        Some(prev_utt) => {
            // Check if it ended with +/. (interruption)
            let has_interruption = if let Some(ref term) = prev_utt.main.content.terminator {
                matches!(term, Terminator::Interruption { .. })
            } else {
                false
            };

            if !has_interruption {
                errors.push(
                    ParseError::new(
                        ErrorCode::MissingQuoteEnd,
                        Severity::Error,
                        SourceLocation::new(utterance.main.span),
                        ErrorContext::new(
                            format!("*{}: +, ...", speaker),
                            utterance.main.span,
                            "self-completion linker",
                        ),
                        format!(
                            "Self-completion linker (+,) but preceding same-speaker utterance doesn't end with +/. (interruption terminator) from speaker {}",
                            speaker
                        ),
                    )
                    .with_suggestion("Change the preceding utterance terminator to +/. to mark it as interrupted")
                );
            }
        }
    }

    errors
}

/// Validate one `++` other-completion linker usage.
///
/// Requires: Most recent utterance by DIFFERENT speaker ended with +... (trailing off)
#[allow(dead_code)]
pub(super) fn check_other_completion(utterances: &[Utterance], idx: usize) -> Vec<ParseError> {
    let mut errors = Vec::new();
    let utterance = &utterances[idx];
    let speaker = utterance.main.speaker.as_str();

    // Check if there's any preceding utterance at all
    if idx == 0 {
        errors.push(
            ParseError::new(
                ErrorCode::MissingOtherCompletionContext,
                Severity::Error,
                SourceLocation::new(utterance.main.span),
                ErrorContext::new(
                    format!("*{}: ++ ...", speaker),
                    utterance.main.span,
                    "other-completion linker",
                ),
                "Other-completion linker (++) without any preceding utterance from different speaker",
            )
            .with_suggestion("Other-completion is used to finish another speaker's incomplete thought; ensure there's a prior incomplete utterance with +... terminator from a different speaker")
        );
        return errors;
    }

    // Get most recent utterance (regardless of speaker)
    let prev_utt = &utterances[idx - 1];

    // Check if same speaker - should use +, instead
    if prev_utt.main.speaker.as_str() == speaker {
        errors.push(
            ParseError::new(
                ErrorCode::InterleavedScopedAnnotations,
                Severity::Error,
                SourceLocation::new(utterance.main.span),
                ErrorContext::new(
                    format!("*{}: ++ ...", speaker),
                    utterance.main.span,
                    "other-completion linker",
                ),
                format!(
                    "Other-completion linker (++) but preceding utterance is from same speaker ({}); use +, for self-completion",
                    speaker
                ),
            )
            .with_suggestion("Change ++ to +, when completing your own interrupted utterance")
        );
        return errors;
    }

    // Now check if it ended with +... (trailing off)
    let has_trailing_off = if let Some(ref term) = prev_utt.main.content.terminator {
        matches!(term, Terminator::TrailingOff { .. })
    } else {
        false
    };

    if !has_trailing_off {
        let prev_speaker = prev_utt.main.speaker.as_str();
        errors.push(
            ParseError::new(
                ErrorCode::MissingTrailingOffTerminator,
                Severity::Error,
                SourceLocation::new(utterance.main.span),
                ErrorContext::new(
                    format!("*{}: ++ ...", speaker),
                    utterance.main.span,
                    "other-completion linker",
                ),
                format!(
                    "Other-completion linker (++) but preceding different-speaker utterance (by {}) doesn't end with +... (trailing off terminator)",
                    prev_speaker
                ),
            )
            .with_suggestion(format!(
                "Change the preceding utterance by {} to end with +... to mark it as trailing off/incomplete",
                prev_speaker
            ))
        );
    }

    errors
}
