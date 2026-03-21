//! Test module for test parse health recovery in `talkbank-chat`.
//!
//! These tests document expected behavior and regressions.

use talkbank_direct_parser::DirectParser;
use talkbank_model::ChatParser;
use talkbank_model::ErrorCollector;
use talkbank_model::model::{
    ChatOptionFlag, DependentTier, ParseHealthState, ParseHealthTier, UtteranceContent,
};
use talkbank_model::parser_api::FragmentSemanticContext;
use talkbank_parser_tests::test_error::TestError;

/// Returns whether a specific parse-health tier remains clean.
fn tier_is_clean(health: ParseHealthState, tier: ParseHealthTier) -> bool {
    health.is_tier_clean(tier)
}

/// Parses utterance marks parse health for invalid dependent tier.
#[test]
fn parse_utterance_marks_parse_health_for_invalid_dependent_tier() -> Result<(), TestError> {
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let input = "*CHI:\thello .\n%mor:\tpro:sub|I .\n%gra no_tab_separator";
    let errors = ErrorCollector::new();

    let utterance = ChatParser::parse_utterance(&parser, input, 0, &errors)
        .into_option()
        .ok_or_else(|| TestError::Failure("Expected utterance recovery".to_string()))?;

    let diagnostics = errors.to_vec();
    assert!(
        !diagnostics.is_empty(),
        "Expected parse diagnostics for invalid dependent tier"
    );
    assert!(
        utterance.mor_tier().is_some(),
        "Valid %mor should be preserved"
    );
    assert!(
        utterance.gra_tier().is_none(),
        "Invalid %gra should be dropped"
    );

    let health = utterance.parse_health;
    assert!(health.is_tier_clean(ParseHealthTier::Main));
    assert!(health.is_tier_clean(ParseHealthTier::Mor));
    assert!(health.is_tier_tainted(ParseHealthTier::Gra));

    Ok(())
}

/// Verifies malformed standard dependent tiers taint only their own alignment domain.
#[test]
fn malformed_standard_dependent_tiers_taint_only_their_alignment_domain() -> Result<(), TestError> {
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let cases = [
        ("%mor no_tab_separator", ParseHealthTier::Mor),
        ("%gra no_tab_separator", ParseHealthTier::Gra),
        ("%pho no_tab_separator", ParseHealthTier::Pho),
        ("%wor no_tab_separator", ParseHealthTier::Wor),
    ];
    let dependent_tiers = [
        ParseHealthTier::Mor,
        ParseHealthTier::Gra,
        ParseHealthTier::Pho,
        ParseHealthTier::Wor,
        ParseHealthTier::Mod,
        ParseHealthTier::Sin,
    ];

    for (malformed_tier, expected_taint) in cases {
        let input = format!("*CHI:\thello .\n{malformed_tier}");
        let errors = ErrorCollector::new();

        let utterance = ChatParser::parse_utterance(&parser, &input, 0, &errors)
            .into_option()
            .ok_or_else(|| TestError::Failure(format!("Expected recovery for {malformed_tier}")))?;

        let diagnostics = errors.to_vec();
        assert!(
            !diagnostics.is_empty(),
            "Expected diagnostics for malformed tier {malformed_tier}"
        );

        let health = utterance.parse_health;

        assert!(
            health.is_tier_clean(ParseHealthTier::Main),
            "Main tier should remain clean for malformed dependent tier {malformed_tier}"
        );

        for tier in dependent_tiers {
            let expected_clean = tier != expected_taint;
            assert_eq!(
                tier_is_clean(health, tier),
                expected_clean,
                "Unexpected parse-health state for {tier:?} from malformed tier {malformed_tier}"
            );
        }
    }

    Ok(())
}

/// Verifies malformed unknown dependent tiers taint all alignment-dependent domains.
#[test]
fn malformed_unknown_dependent_tier_taints_all_alignment_dependents() -> Result<(), TestError> {
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let input = "*CHI:\thello .\n%xfoo no_tab_separator";
    let errors = ErrorCollector::new();

    let utterance = ChatParser::parse_utterance(&parser, input, 0, &errors)
        .into_option()
        .ok_or_else(|| TestError::Failure("Expected recovery for malformed %xfoo".to_string()))?;

    let diagnostics = errors.to_vec();
    assert!(
        !diagnostics.is_empty(),
        "Expected parse diagnostics for malformed %xfoo"
    );

    let health = utterance.parse_health;
    assert!(health.is_tier_clean(ParseHealthTier::Main));
    assert!(health.is_tier_tainted(ParseHealthTier::Mor));
    assert!(health.is_tier_tainted(ParseHealthTier::Gra));
    assert!(health.is_tier_tainted(ParseHealthTier::Pho));
    assert!(health.is_tier_tainted(ParseHealthTier::Wor));
    assert!(health.is_tier_tainted(ParseHealthTier::Mod));
    assert!(health.is_tier_tainted(ParseHealthTier::Sin));

    Ok(())
}

/// Parses chat file result api remains error when recovery emits diagnostics.
#[test]
fn parse_chat_file_result_api_remains_error_when_recovery_emits_diagnostics()
-> Result<(), TestError> {
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let input = "@UTF8\n@Begin\n*CHI:\thello .\n%gra no_tab_separator\n@End\n";

    let result = parser.parse_chat_file(input);
    assert!(
        result.is_err(),
        "Result API must remain Err when parse diagnostics are emitted"
    );

    Ok(())
}

/// Verifies a malformed standard tier does not discard later valid sibling tiers.
#[test]
fn malformed_standard_tier_preserves_later_valid_siblings() -> Result<(), TestError> {
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let input = "*CHI:\thello .\n%mor:\tintj|hello .\n%gra no_tab_separator\n%com:\tkept comment\n";
    let errors = ErrorCollector::new();

    let utterance = ChatParser::parse_utterance(&parser, input, 0, &errors)
        .into_option()
        .ok_or_else(|| TestError::Failure("Expected utterance recovery".to_string()))?;

    let diagnostics = errors.to_vec();
    assert!(
        !diagnostics.is_empty(),
        "Expected diagnostics for malformed %gra tier"
    );
    assert!(
        utterance.mor_tier().is_some(),
        "Valid %mor tier should be preserved"
    );
    assert!(
        utterance.gra_tier().is_none(),
        "Malformed %gra tier should not be fabricated"
    );
    assert!(
        utterance
            .dependent_tiers
            .iter()
            .any(|tier| matches!(tier, DependentTier::Com(_))),
        "Later valid %com tier should still be preserved"
    );

    let health = utterance.parse_health;
    assert!(health.is_tier_clean(ParseHealthTier::Main));
    assert!(health.is_tier_clean(ParseHealthTier::Mor));
    assert!(health.is_tier_tainted(ParseHealthTier::Gra));

    Ok(())
}

/// Verifies malformed unknown tiers do not fabricate placeholder user-defined tiers.
#[test]
fn malformed_unknown_tier_does_not_fabricate_user_defined_placeholder() -> Result<(), TestError> {
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let input = "*CHI:\thello .\n%xfoo no_tab_separator\n%com:\tkept comment\n";
    let errors = ErrorCollector::new();

    let utterance = ChatParser::parse_utterance(&parser, input, 0, &errors)
        .into_option()
        .ok_or_else(|| TestError::Failure("Expected utterance recovery".to_string()))?;

    let diagnostics = errors.to_vec();
    assert!(
        !diagnostics.is_empty(),
        "Expected diagnostics for malformed unknown tier"
    );
    assert!(
        utterance.dependent_tiers.iter().all(
            |tier| !matches!(tier, DependentTier::UserDefined(ud) if ud.label.as_ref() == "xfoo")
        ),
        "Malformed unknown tier should not fabricate a placeholder tier"
    );
    assert!(
        utterance
            .dependent_tiers
            .iter()
            .any(|tier| matches!(tier, DependentTier::Com(_))),
        "Later valid %com tier should still be preserved"
    );

    let health = utterance.parse_health;
    assert!(health.is_tier_clean(ParseHealthTier::Main));
    assert!(health.is_tier_tainted(ParseHealthTier::Mor));
    assert!(health.is_tier_tainted(ParseHealthTier::Gra));
    assert!(health.is_tier_tainted(ParseHealthTier::Pho));
    assert!(health.is_tier_tainted(ParseHealthTier::Wor));
    assert!(health.is_tier_tainted(ParseHealthTier::Mod));
    assert!(health.is_tier_tainted(ParseHealthTier::Sin));

    Ok(())
}

/// Verifies isolated parse_word remains strict even though main-tier parsing can recover.
#[test]
fn isolated_parse_word_rejects_malformed_word() -> Result<(), TestError> {
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors = ErrorCollector::new();

    let word = ChatParser::parse_word(&parser, "he(llo", 0, &errors);

    assert!(
        word.is_rejected(),
        "isolated parse_word should remain strict for malformed input"
    );
    assert!(
        !errors.is_empty(),
        "isolated parse_word should emit diagnostics for malformed input"
    );

    Ok(())
}

/// Verifies main-tier parsing can preserve a malformed word token as raw text.
#[test]
fn main_tier_recovery_preserves_malformed_word_as_raw_text() -> Result<(), TestError> {
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors = ErrorCollector::new();

    let main = ChatParser::parse_main_tier(&parser, "*CHI:\thello he(llo world .", 0, &errors)
        .into_option()
        .ok_or_else(|| TestError::Failure("Expected main-tier recovery".to_string()))?;

    let words: Vec<_> = main
        .content
        .content
        .iter()
        .filter_map(|item| match item {
            UtteranceContent::Word(word) => Some(word),
            _ => None,
        })
        .collect();

    assert_eq!(
        words.len(),
        3,
        "expected recovered main tier to keep surrounding valid words"
    );
    assert_eq!(words[0].raw_text(), "hello");
    assert_eq!(words[1].raw_text(), "he(llo");
    assert_eq!(words[2].raw_text(), "world");

    Ok(())
}

/// Verifies CA omission shorthand is rejected without CA semantic context.
#[test]
fn main_tier_rejects_ca_omission_shorthand_without_context() -> Result<(), TestError> {
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors = ErrorCollector::new();

    let main = ChatParser::parse_main_tier(&parser, "*CHI:\t(word) .", 0, &errors);

    assert!(
        main.is_rejected(),
        "CA omission shorthand should be rejected without CA context"
    );
    assert!(
        !errors.is_empty(),
        "CA omission shorthand should emit diagnostics without CA context"
    );

    Ok(())
}

/// Verifies CA omission shorthand is accepted when CA semantic context is explicit.
#[test]
fn main_tier_accepts_ca_omission_shorthand_with_context() -> Result<(), TestError> {
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let errors = ErrorCollector::new();
    let context = FragmentSemanticContext::new().with_option_flag(ChatOptionFlag::Ca);

    let main =
        ChatParser::parse_main_tier_with_context(&parser, "*CHI:\t(word) .", 0, &context, &errors)
            .into_option()
            .ok_or_else(|| {
                TestError::Failure("Expected CA omission shorthand to parse".to_string())
            })?;

    assert!(
        errors.is_empty(),
        "CA-context parsing should not emit diagnostics for CA omission shorthand"
    );
    assert!(
        main.content
            .content
            .iter()
            .any(|item| matches!(item, UtteranceContent::Word(_))),
        "parsed main tier should contain a word item"
    );

    Ok(())
}

/// Verifies utterance-level fragment context follows the same CA contract as main-tier parsing.
#[test]
fn parse_utterance_rejects_and_accepts_ca_omission_based_on_context() -> Result<(), TestError> {
    let parser = DirectParser::new().map_err(|err| TestError::ParserInit(err.to_string()))?;
    let input = "*CHI:\t(word) .\n%com:\tkept comment\n";
    let default_errors = ErrorCollector::new();

    let rejected = ChatParser::parse_utterance(&parser, input, 0, &default_errors);
    assert!(
        rejected.is_rejected(),
        "utterance parsing should reject CA omission shorthand without context"
    );
    assert!(
        !default_errors.is_empty(),
        "utterance parsing should emit diagnostics without CA context"
    );

    let context = FragmentSemanticContext::new().with_option_flag(ChatOptionFlag::Ca);
    let context_errors = ErrorCollector::new();
    let utterance = ChatParser::parse_utterance_with_context(
        &parser,
        input,
        0,
        &context,
        &context_errors,
    )
    .into_option()
    .ok_or_else(|| {
        TestError::Failure("Expected CA-context utterance parsing to succeed".to_string())
    })?;

    assert!(
        context_errors.is_empty(),
        "CA-context utterance parsing should not emit diagnostics"
    );
    assert!(
        utterance
            .dependent_tiers
            .iter()
            .any(|tier| matches!(tier, DependentTier::Com(_))),
        "contextual utterance parsing should preserve later valid dependent tiers"
    );

    Ok(())
}
