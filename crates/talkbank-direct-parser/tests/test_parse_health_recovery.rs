//! Test module for test parse health recovery in `talkbank-chat`.
//!
//! These tests document expected behavior and regressions.

use talkbank_direct_parser::DirectParser;
use talkbank_model::ChatParser;
use talkbank_model::ErrorCollector;
use talkbank_model::model::{ParseHealth, ParseHealthTier};
use talkbank_parser_tests::test_error::TestError;

/// Returns whether a specific parse-health tier remains clean.
fn tier_is_clean(health: &ParseHealth, tier: ParseHealthTier) -> bool {
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

    let health = utterance
        .parse_health
        .ok_or_else(|| TestError::Failure("Expected parse_health taint".to_string()))?;
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

        let health = utterance.parse_health.ok_or_else(|| {
            TestError::Failure(format!("Expected parse_health for {malformed_tier}"))
        })?;

        assert!(
            health.is_tier_clean(ParseHealthTier::Main),
            "Main tier should remain clean for malformed dependent tier {malformed_tier}"
        );

        for tier in dependent_tiers {
            let expected_clean = tier != expected_taint;
            assert_eq!(
                tier_is_clean(&health, tier),
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

    let health = utterance
        .parse_health
        .ok_or_else(|| TestError::Failure("Expected parse_health taint".to_string()))?;
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
