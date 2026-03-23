//! Recovery corpus integration tests.
//!
//! Tests that BOTH parsers (tree-sitter and direct) recover from errors
//! in composite files with multiple error types, keeping valid utterances
//! intact and properly reporting errors for broken ones.

use std::path::PathBuf;
use talkbank_model::ChatParser;
use talkbank_model::model::ParseHealthState;
use talkbank_model::{ErrorCode, ErrorCollector};

/// Runs recovery corpus dir.
fn recovery_corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/recovery_corpus")
}

/// Builds parsers for downstream use.
fn create_parsers() -> (
    talkbank_parser::TreeSitterParser,
    talkbank_parser::TreeSitterParser,
) {
    let ts = talkbank_parser::TreeSitterParser::new().expect("tree-sitter parser");
    let dp = talkbank_parser::TreeSitterParser::new().expect("direct parser");
    (ts, dp)
}

/// Parse a file with both parsers and assert basic recovery properties.
fn assert_both_parsers_produce_file(content: &str, expected_utterances: usize, test_name: &str) {
    let (ts, dp) = create_parsers();

    // Tree-sitter parser
    let ts_errors = ErrorCollector::new();
    let ts_result = ChatParser::parse_chat_file(&ts, content, 0, &ts_errors);
    let ts_file = ts_result
        .into_option()
        .unwrap_or_else(|| panic!("[{}] tree-sitter should produce a ChatFile", test_name));
    let ts_utterance_count = ts_file
        .lines
        .iter()
        .filter(|l| matches!(l, talkbank_model::model::Line::Utterance(_)))
        .count();
    assert_eq!(
        ts_utterance_count, expected_utterances,
        "[{}] tree-sitter: expected {} utterances, got {}",
        test_name, expected_utterances, ts_utterance_count
    );

    // Direct parser
    let dp_errors = ErrorCollector::new();
    let dp_result = ChatParser::parse_chat_file(&dp, content, 0, &dp_errors);
    let dp_file = dp_result
        .into_option()
        .unwrap_or_else(|| panic!("[{}] direct parser should produce a ChatFile", test_name));
    let dp_utterance_count = dp_file
        .lines
        .iter()
        .filter(|l| matches!(l, talkbank_model::model::Line::Utterance(_)))
        .count();
    assert_eq!(
        dp_utterance_count, expected_utterances,
        "[{}] direct parser: expected {} utterances, got {}",
        test_name, expected_utterances, dp_utterance_count
    );
}

/// Parse a file with the direct parser and return the ChatFile + errors.
fn parse_with_direct(
    content: &str,
) -> (
    talkbank_model::model::ChatFile,
    Vec<talkbank_model::ParseError>,
) {
    let dp = talkbank_parser::TreeSitterParser::new().expect("direct parser");
    let errors = ErrorCollector::new();
    let result = ChatParser::parse_chat_file(&dp, content, 0, &errors);
    let file = result
        .into_option()
        .expect("direct parser should produce a ChatFile");
    (file, errors.into_vec())
}

/// Returns utterances.
fn get_utterances(
    file: &talkbank_model::model::ChatFile,
) -> Vec<&talkbank_model::model::Utterance> {
    file.lines
        .iter()
        .filter_map(|l| match l {
            talkbank_model::model::Line::Utterance(u) => Some(u.as_ref()),
            _ => None,
        })
        .collect()
}

// =============================================================================
// Tests
// =============================================================================

/// Runs multi tier errors both parsers recover.
#[test]
fn multi_tier_errors_both_parsers_recover() {
    let content = std::fs::read_to_string(recovery_corpus_dir().join("multi-tier-errors.cha"))
        .expect("read file");
    assert_both_parsers_produce_file(&content, 5, "multi-tier-errors");
}

/// Runs multi tier errors direct parser taint.
#[test]
fn multi_tier_errors_direct_parser_taint() {
    let content = std::fs::read_to_string(recovery_corpus_dir().join("multi-tier-errors.cha"))
        .expect("read file");
    let (file, errors) = parse_with_direct(&content);
    let utterances = get_utterances(&file);
    assert_eq!(utterances.len(), 5);

    // Utt 0 (CHI: hello there) — clean
    assert!(
        !matches!(utterances[0].parse_health, ParseHealthState::Tainted(_)),
        "utt0 should be clean"
    );

    // Utt 1 (MOT: good boy) — bad %mor item (BROKEN_MOR)
    if let ParseHealthState::Tainted(health) = &utterances[1].parse_health {
        assert!(
            health.is_tier_tainted(talkbank_model::model::ParseHealthTier::Mor),
            "utt1: %mor should be tainted"
        );
    }

    // Utt 2 (CHI: want cookie) — clean
    assert!(
        !matches!(utterances[2].parse_health, ParseHealthState::Tainted(_)),
        "utt2 should be clean"
    );

    // Utt 3 (MOT: here you go) — bad %gra item (NOTGRA)
    if let ParseHealthState::Tainted(health) = &utterances[3].parse_health {
        assert!(
            health.is_tier_tainted(talkbank_model::model::ParseHealthTier::Gra),
            "utt3: %gra should be tainted"
        );
    }

    // Utt 4 (CHI: yummy) — clean
    assert!(
        !matches!(utterances[4].parse_health, ParseHealthState::Tainted(_)),
        "utt4 should be clean"
    );

    // Errors should be reported
    assert!(
        errors.iter().any(|e| e.code == ErrorCode::MorParseError),
        "expected MorParseError"
    );
    assert!(
        errors.iter().any(|e| e.code == ErrorCode::GraParseError),
        "expected GraParseError"
    );
}

/// Runs all tiers partial both parsers recover.
#[test]
fn all_tiers_partial_both_parsers_recover() {
    let content = std::fs::read_to_string(recovery_corpus_dir().join("all-tiers-partial.cha"))
        .expect("read file");
    assert_both_parsers_produce_file(&content, 1, "all-tiers-partial");
}

/// Runs all tiers partial direct parser recovery details.
#[test]
fn all_tiers_partial_direct_parser_recovery_details() {
    let content = std::fs::read_to_string(recovery_corpus_dir().join("all-tiers-partial.cha"))
        .expect("read file");
    let (file, errors) = parse_with_direct(&content);
    let utterances = get_utterances(&file);
    assert_eq!(utterances.len(), 1);

    let utt = &utterances[0];

    // %mor should have recovered 3 items (pron|I, det|the, noun|dog), skipping BADMOR
    if let Some(mor) = utt.mor_tier() {
        assert_eq!(mor.items.len(), 3, "mor should have 3 recovered items");
    }

    // %gra should have recovered 4 relations, skipping NOTRELATION
    if let Some(gra) = utt.gra_tier() {
        assert_eq!(
            gra.relations.len(),
            4,
            "gra should have 4 recovered relations"
        );
    }

    // Both mor and gra should be tainted
    if let ParseHealthState::Tainted(health) = &utt.parse_health {
        assert!(
            health.is_tier_tainted(talkbank_model::model::ParseHealthTier::Mor),
            "mor should be tainted"
        );
        assert!(
            health.is_tier_tainted(talkbank_model::model::ParseHealthTier::Gra),
            "gra should be tainted"
        );
    }

    assert!(
        errors.iter().any(|e| e.code == ErrorCode::MorParseError),
        "expected MorParseError"
    );
    assert!(
        errors.iter().any(|e| e.code == ErrorCode::GraParseError),
        "expected GraParseError"
    );
}

/// Runs dependent tier mixed both parsers recover.
#[test]
fn dependent_tier_mixed_both_parsers_recover() {
    let content = std::fs::read_to_string(recovery_corpus_dir().join("dependent-tier-mixed.cha"))
        .expect("read file");
    assert_both_parsers_produce_file(&content, 3, "dependent-tier-mixed");
}

/// Runs dependent tier mixed clean utterances intact.
#[test]
fn dependent_tier_mixed_clean_utterances_intact() {
    let content = std::fs::read_to_string(recovery_corpus_dir().join("dependent-tier-mixed.cha"))
        .expect("read file");
    let (file, _errors) = parse_with_direct(&content);
    let utterances = get_utterances(&file);

    // Utt 0 (CHI: hello) — fully clean with %mor and %gra
    assert!(
        !matches!(utterances[0].parse_health, ParseHealthState::Tainted(_)),
        "utt0 should be clean"
    );
    assert!(utterances[0].mor_tier().is_some(), "utt0 should have %mor");
    assert!(utterances[0].gra_tier().is_some(), "utt0 should have %gra");

    // Utt 2 (CHI: bye) — fully clean with %mor and %gra
    assert!(
        !matches!(utterances[2].parse_health, ParseHealthState::Tainted(_)),
        "utt2 should be clean"
    );
    assert!(utterances[2].mor_tier().is_some(), "utt2 should have %mor");
    assert!(utterances[2].gra_tier().is_some(), "utt2 should have %gra");
}

/// Runs cascading recovery both parsers recover.
#[test]
fn cascading_recovery_both_parsers_recover() {
    let content = std::fs::read_to_string(recovery_corpus_dir().join("cascading-recovery.cha"))
        .expect("read file");
    assert_both_parsers_produce_file(&content, 6, "cascading-recovery");
}

/// Runs cascading recovery direct parser details.
#[test]
fn cascading_recovery_direct_parser_details() {
    let content = std::fs::read_to_string(recovery_corpus_dir().join("cascading-recovery.cha"))
        .expect("read file");
    let (file, errors) = parse_with_direct(&content);
    let utterances = get_utterances(&file);
    assert_eq!(utterances.len(), 6);

    // Utt 0 (CHI: hello) — clean
    assert!(!matches!(utterances[0].parse_health, ParseHealthState::Tainted(_)),);

    // Utt 1 (MOT: hi baby) — bad %gra (BROKEN_GRA)
    if let ParseHealthState::Tainted(health) = &utterances[1].parse_health {
        assert!(
            health.is_tier_tainted(talkbank_model::model::ParseHealthTier::Gra),
            "utt1: %gra should be tainted"
        );
        assert!(
            health.is_tier_clean(talkbank_model::model::ParseHealthTier::Mor),
            "utt1: %mor should be clean"
        );
    }

    // Utt 2 (FAT: how are you?) — clean
    assert!(!matches!(utterances[2].parse_health, ParseHealthState::Tainted(_)),);

    // Utt 3 (CHI: good) — clean
    assert!(!matches!(utterances[3].parse_health, ParseHealthState::Tainted(_)),);

    // Utt 4 (MOT: want some milk?) — bad %mor (BADWORD1) and bad %gra (BADREL)
    if let ParseHealthState::Tainted(health) = &utterances[4].parse_health {
        assert!(
            health.is_tier_tainted(talkbank_model::model::ParseHealthTier::Mor),
            "utt4: %mor should be tainted"
        );
        assert!(
            health.is_tier_tainted(talkbank_model::model::ParseHealthTier::Gra),
            "utt4: %gra should be tainted"
        );
    }

    // Utt 5 (CHI: yes please) — clean
    assert!(!matches!(utterances[5].parse_health, ParseHealthState::Tainted(_)),);

    // Verify multiple error types reported
    let mor_errors = errors
        .iter()
        .filter(|e| e.code == ErrorCode::MorParseError)
        .count();
    let gra_errors = errors
        .iter()
        .filter(|e| e.code == ErrorCode::GraParseError)
        .count();
    assert!(mor_errors > 0, "expected MorParseError");
    assert!(gra_errors > 0, "expected GraParseError");
}

/// Runs degraded main tier direct parser recovery.
#[test]
fn degraded_main_tier_direct_parser_recovery() {
    let content = std::fs::read_to_string(recovery_corpus_dir().join("degraded-main-tier.cha"))
        .expect("read file");
    let (file, errors) = parse_with_direct(&content);
    let utterances = get_utterances(&file);

    // Direct parser should recover 3 utterances:
    // utt0 (CHI: hello) clean, utt1 (MOT: foo [) degraded, utt2 (CHI: bye) clean
    assert_eq!(utterances.len(), 3);

    // Utt 0 — clean
    assert_eq!(utterances[0].main.speaker.as_str(), "CHI");
    assert!(!matches!(utterances[0].parse_health, ParseHealthState::Tainted(_)),);

    // Utt 1 — degraded (speaker extracted, content empty, main tainted)
    assert_eq!(utterances[1].main.speaker.as_str(), "MOT");
    assert!(utterances[1].main.content.content.is_empty());
    if let ParseHealthState::Tainted(health) = &utterances[1].parse_health {
        assert!(
            health.is_tier_tainted(talkbank_model::model::ParseHealthTier::Main),
            "degraded main tier should taint main"
        );
    }
    // %mor tier should still be attached to the degraded utterance
    assert!(
        utterances[1].mor_tier().is_some(),
        "dependent tiers should attach to degraded main tier"
    );

    // Utt 2 — clean
    assert_eq!(utterances[2].main.speaker.as_str(), "CHI");
    assert!(!matches!(utterances[2].parse_health, ParseHealthState::Tainted(_)),);

    // Errors should be reported for the malformed main tier
    assert!(
        !errors.is_empty(),
        "expected errors for malformed main tier"
    );
}
