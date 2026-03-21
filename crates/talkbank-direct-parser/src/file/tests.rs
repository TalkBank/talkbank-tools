use super::parse_chat_file_impl;
use std::error::Error;
use std::fmt;
use talkbank_model::model::{Header, Line, SpeakerCode};
use talkbank_model::{ErrorCode, ErrorCollector};

/// Local failures used by file-parser unit tests.
#[derive(Debug)]
enum TestError {
    ParseReturnedNone,
    MissingUtterance,
    MissingContent,
    UnexpectedContent,
}

impl fmt::Display for TestError {
    /// Render a short test failure description.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseReturnedNone => write!(f, "Parse returned None"),
            Self::MissingUtterance => write!(f, "Missing utterance"),
            Self::MissingContent => write!(f, "Missing word content"),
            Self::UnexpectedContent => write!(f, "Unexpected first content item"),
        }
    }
}

impl Error for TestError {}

/// Tests minimal file.
#[test]
fn test_minimal_file() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n*CHI:\thello .\n@End\n";
    let errors = ErrorCollector::new();
    let result = parse_chat_file_impl(input, 0, &errors);

    assert!(
        errors.is_empty(),
        "Should have no errors: {:?}",
        errors.into_vec()
    );
    let file = result.ok_or(TestError::ParseReturnedNone)?;
    assert_eq!(file.lines.len(), 4); // UTF8, Begin, Utterance, End

    Ok(())
}

/// Tests with dependent tiers.
#[test]
fn test_with_dependent_tiers() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n*CHI:\thello .\n%mor:\tpro|I .\n@End\n";
    let errors = ErrorCollector::new();
    let result = parse_chat_file_impl(input, 0, &errors);

    let file = result.ok_or(TestError::ParseReturnedNone)?;

    // Find utterance and verify dependent tier attached
    let utterances: Vec<_> = file
        .lines
        .iter()
        .filter_map(|line| match line {
            Line::Utterance(utterance) => Some(utterance),
            _ => None,
        })
        .collect();

    assert_eq!(utterances.len(), 1);
    assert_eq!(utterances[0].dependent_tiers.len(), 1);

    Ok(())
}

/// Tests participants header retained and participant map built.
#[test]
fn test_participants_header_retained_and_participant_map_built() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Ruth Target_Child, INV Chiat Investigator\n@ID:\teng|chiat|CHI|10;03.||||Target_Child|||\n@ID:\teng|chiat|INV|||||Investigator|||\n*CHI:\thello .\n@End\n";
    let errors = ErrorCollector::new();
    let result = parse_chat_file_impl(input, 0, &errors);
    let file = result.ok_or(TestError::ParseReturnedNone)?;

    assert!(
        errors.is_empty(),
        "Unexpected errors: {:?}",
        errors.into_vec()
    );
    assert!(
        file.lines.iter().any(|line| matches!(
            line,
            Line::Header { header, .. } if matches!(header.as_ref(), Header::Participants { .. })
        )),
        "Expected @Participants header to be preserved in line stream"
    );
    assert_eq!(file.participants.len(), 2);
    assert!(file.participants.contains_key(&SpeakerCode::new("CHI")));
    assert!(file.participants.contains_key(&SpeakerCode::new("INV")));

    Ok(())
}

/// Tests amp prefixed words are preserved in file parse.
#[test]
fn test_amp_prefixed_words_are_preserved_in_file_parse() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child, MOT Mother\n@ID:\teng|bates|CHI|1;08.|female|normal||Child|||\n@ID:\teng|bates|MOT|||||Mother|||\n*CHI:\tfoo &~word↑ &~ba↑r stuff .\n@End\n";
    let errors = ErrorCollector::new();
    let file = parse_chat_file_impl(input, 0, &errors).ok_or(TestError::ParseReturnedNone)?;
    assert!(
        errors.is_empty(),
        "unexpected parse errors: {:?}",
        errors.to_vec()
    );
    assert!(
        file.lines
            .iter()
            .any(|line| matches!(line, Line::Utterance(_))),
        "expected utterance line to be present"
    );
    Ok(())
}

/// Tests file recovers degraded main tier on content parse failure.
#[test]
fn test_file_recovers_degraded_main_tier_on_content_parse_failure() -> Result<(), TestError> {
    // Main tier has malformed content (`foo [`) but valid speaker code.
    // Recovery extracts speaker "CHI" and creates a degraded main tier
    // with empty content and main-tier parse taint.
    let input = "@UTF8\n@Begin\n*CHI:\tfoo [\n@End\n";
    let errors = ErrorCollector::new();
    let file = parse_chat_file_impl(input, 0, &errors).ok_or(TestError::ParseReturnedNone)?;

    assert!(
        !errors.is_empty(),
        "expected parse errors for malformed main tier"
    );

    let utterance = file
        .lines
        .iter()
        .find_map(|line| match line {
            Line::Utterance(u) => Some(u),
            _ => None,
        })
        .ok_or(TestError::MissingUtterance)?;

    // Degraded main tier should have extracted speaker code
    assert_eq!(utterance.main.speaker.as_str(), "CHI");
    // Content is empty (degraded)
    assert!(utterance.main.content.content.is_empty());

    // Main tier parse taint should be set
    let health = utterance.parse_health;
    assert!(
        health.is_tier_tainted(talkbank_model::model::ParseHealthTier::Main),
        "degraded main tier should have main parse taint"
    );

    Ok(())
}

/// Tests degraded main tier keeps later valid dependent tiers attached.
#[test]
fn test_degraded_main_tier_keeps_later_valid_dependent_tiers() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n*CHI:\tfoo [\n%com:\tkept comment\n@End\n";
    let errors = ErrorCollector::new();
    let file = parse_chat_file_impl(input, 0, &errors).ok_or(TestError::ParseReturnedNone)?;

    assert!(
        !errors.is_empty(),
        "expected parse errors for degraded main tier"
    );

    let utterance = file
        .lines
        .iter()
        .find_map(|line| match line {
            Line::Utterance(u) => Some(u),
            _ => None,
        })
        .ok_or(TestError::MissingUtterance)?;

    assert_eq!(utterance.main.speaker.as_str(), "CHI");
    assert!(utterance.main.content.content.is_empty());
    assert!(
        utterance
            .dependent_tiers
            .iter()
            .any(|tier| matches!(tier, talkbank_model::dependent_tier::DependentTier::Com(_))),
        "valid %com tier should still attach to degraded main tier shell"
    );
    assert!(
        utterance
            .parse_health
            .is_tier_tainted(talkbank_model::model::ParseHealthTier::Main),
        "degraded main tier should remain explicitly tainted"
    );

    Ok(())
}

/// Tests file recovers from dependent tier parse error and marks parse health.
#[test]
fn test_file_recovers_from_dependent_tier_parse_error_and_marks_parse_health()
-> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n*CHI:\thello .\n%mor:\tpro:sub|I .\n%gra no_tab_separator\n@End\n";
    let errors = ErrorCollector::new();
    let file = parse_chat_file_impl(input, 0, &errors).ok_or(TestError::ParseReturnedNone)?;

    let diagnostics = errors.to_vec();
    assert!(
        diagnostics
            .iter()
            .any(|err| err.code == ErrorCode::InvalidDependentTier),
        "expected InvalidDependentTier error, got: {:?}",
        diagnostics
    );

    let utterance = file
        .lines
        .iter()
        .find_map(|line| match line {
            Line::Utterance(utt) => Some(utt),
            _ => None,
        })
        .ok_or(TestError::MissingUtterance)?;

    assert_eq!(
        utterance.dependent_tiers.len(),
        1,
        "invalid %gra tier should be dropped while keeping valid %mor tier"
    );
    assert!(utterance.mor_tier().is_some(), "expected parsed %mor tier");
    assert!(
        utterance.gra_tier().is_none(),
        "invalid %gra tier must not be attached"
    );

    let health = utterance.parse_health;
    assert!(health.is_tier_clean(talkbank_model::model::ParseHealthTier::Main));
    assert!(health.is_tier_clean(talkbank_model::model::ParseHealthTier::Mor));
    assert!(
        health.is_tier_tainted(talkbank_model::model::ParseHealthTier::Gra),
        "failed %gra parse must taint gra alignment"
    );

    Ok(())
}

/// Tests orphaned dependent tiers report errors but do not poison later utterances.
#[test]
fn test_orphaned_dependent_tier_does_not_poison_later_utterance() -> Result<(), TestError> {
    let input = "@UTF8\n@Begin\n%com:\torphaned comment\n*CHI:\thello .\n@End\n";
    let errors = ErrorCollector::new();
    let file = parse_chat_file_impl(input, 0, &errors).ok_or(TestError::ParseReturnedNone)?;

    let diagnostics = errors.to_vec();
    assert!(
        diagnostics
            .iter()
            .any(|err| err.code == ErrorCode::OrphanedDependentTier),
        "expected orphaned dependent tier diagnostic, got: {:?}",
        diagnostics
    );

    let utterances: Vec<_> = file
        .lines
        .iter()
        .filter_map(|line| match line {
            Line::Utterance(utt) => Some(utt),
            _ => None,
        })
        .collect();
    assert_eq!(
        utterances.len(),
        1,
        "later valid utterance should still parse"
    );
    assert!(
        utterances[0].dependent_tiers.is_empty(),
        "orphaned tier must not silently attach to later utterance"
    );
    assert!(
        utterances[0]
            .parse_health
            .is_tier_clean(talkbank_model::model::ParseHealthTier::Main),
        "later valid utterance should not inherit orphan-tier taint"
    );

    Ok(())
}

/// Tests unrecoverable main tiers still fail the whole file parse.
#[test]
fn test_unrecoverable_main_tier_still_rejects_whole_file() {
    let input = "@UTF8\n@Begin\n*:\tfoo [\n@End\n";
    let errors = ErrorCollector::new();
    let result = parse_chat_file_impl(input, 0, &errors);

    assert!(
        result.is_none(),
        "main tier without recoverable speaker code must still reject whole file"
    );
    assert!(
        !errors.is_empty(),
        "fatal main-tier rejection should still emit diagnostics"
    );
}

/// Test that word spans are correct file-absolute byte offsets.
///
/// This test verifies the span calculation is correct:
/// - Spans should match actual byte positions in the input file
/// - Spans should NOT be double-applied (offset added twice)
///
/// Bug reproduced: spans were being offset-adjusted twice:
/// 1. Once in the word_parser() via `Span::from_usize(span.start + offset, span.end + offset)`
/// 2. Again in parse_single_line() via `shift_spans_after(0, offset as i32)`
#[test]
fn test_word_spans_are_correct_file_offsets() -> Result<(), TestError> {
    // Input file with known byte positions:
    // @UTF8        = bytes 0-5 (5 bytes) + newline = 6
    // @Begin       = bytes 6-12 (6 bytes) + newline = 7
    // *CHI:\thello = bytes 13-24: *=13, C=14, H=15, I=16, :=17, \t=18, h=19, e=20, l=21, l=22, o=23
    //                hello is bytes 19-24, " ." is 24-26, newline = 27
    // @End         = bytes 27-31 + newline
    let input = "@UTF8\n@Begin\n*CHI:\thello .\n@End\n";

    // Verify our expected positions match actual bytes
    assert_eq!(
        &input[19..24],
        "hello",
        "Sanity check: 'hello' at bytes 19-24"
    );

    let errors = ErrorCollector::new();
    let result = parse_chat_file_impl(input, 0, &errors);
    let file = result.ok_or(TestError::ParseReturnedNone)?;

    // Find the utterance and extract the word
    let utterance = file
        .lines
        .iter()
        .find_map(|line| match line {
            Line::Utterance(u) => Some(u),
            _ => None,
        })
        .ok_or(TestError::MissingUtterance)?;

    // Get the first word (should be "hello")
    let word = utterance
        .main
        .content
        .content
        .first()
        .ok_or(TestError::MissingContent)?;

    // Extract span from word
    let span = match word {
        talkbank_model::model::UtteranceContent::Word(w) => w.span,
        _ => return Err(TestError::UnexpectedContent),
    };

    // Verify span points to correct bytes in input
    let start = span.start as usize;
    let end = span.end as usize;
    let extracted = &input[start..end];

    assert_eq!(
        extracted, "hello",
        "Span {:?} should extract 'hello' from input, but got '{}'",
        span, extracted
    );

    // Span should be 19..24 (start of 'hello' to end of 'hello')
    assert_eq!(
        start, 19,
        "Word 'hello' should start at byte 19, got {}",
        start
    );
    assert_eq!(end, 24, "Word 'hello' should end at byte 24, got {}", end);

    Ok(())
}
