//! Tests for same-speaker self-completion (`+/`) sequences.
//!
//! Includes active happy-path coverage plus ignored fixtures for deferred
//! `E351/E352` rule enforcement.

use super::helpers::{check_cross_utterance_patterns, make_utterance};
use crate::Span;
use crate::model::{Linker, Terminator};

/// Valid self-completion sequence emits no cross-utterance errors.
///
/// The interrupted utterance and later `+/` continuation from same speaker should pair cleanly.
#[test]
fn test_self_completion_valid() {
    let utterances = vec![
        make_utterance(
            "CHI",
            vec!["so", "after", "the", "tower"],
            vec![],
            Terminator::Interruption { span: Span::DUMMY },
        ),
        make_utterance(
            "EXP",
            vec!["yeah"],
            vec![],
            Terminator::Period { span: Span::DUMMY },
        ),
        make_utterance(
            "CHI",
            vec!["I", "go", "straight", "ahead"],
            vec![Linker::SelfCompletion],
            Terminator::Period { span: Span::DUMMY },
        ),
    ];

    let errors = check_cross_utterance_patterns(&utterances);
    assert_eq!(
        errors.len(),
        0,
        "Valid self-completion should have no errors"
    );
}

/// Self-completion with no prior utterance triggers `E351` (currently ignored).
///
/// Kept as a TDD regression fixture for eventual rule reactivation.
#[test]
#[ignore = "E351 validation disabled (2025-12-24) - see cross_utterance/mod.rs for rationale"]
fn test_e351_self_completion_no_preceding_utterance() {
    let utterances = vec![make_utterance(
        "CHI",
        vec!["I", "go", "ahead"],
        vec![Linker::SelfCompletion],
        Terminator::Period { span: Span::DUMMY },
    )];

    let errors = check_cross_utterance_patterns(&utterances);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code.as_str(), "E351");
    assert!(
        errors[0]
            .message
            .contains("without any preceding utterance")
    );
}

/// Wrong terminator on source utterance triggers `E352` (currently ignored).
///
/// Self-completion source is expected to end with interruption marker `+/.`.
#[test]
#[ignore = "E352 validation disabled (2025-12-24) - see cross_utterance/mod.rs for rationale"]
fn test_e352_self_completion_wrong_terminator() {
    let utterances = vec![
        make_utterance(
            "CHI",
            vec!["so", "after", "the", "tower"],
            vec![],
            Terminator::Period { span: Span::DUMMY },
        ),
        make_utterance(
            "EXP",
            vec!["yeah"],
            vec![],
            Terminator::Period { span: Span::DUMMY },
        ),
        make_utterance(
            "CHI",
            vec!["I", "go", "ahead"],
            vec![Linker::SelfCompletion],
            Terminator::Period { span: Span::DUMMY },
        ),
    ];

    let errors = check_cross_utterance_patterns(&utterances);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].code.as_str(), "E352");
    assert!(errors[0].message.contains("doesn't end with +/. "));
}
