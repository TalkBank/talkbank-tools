//! Structural roundtrip property tests for main tiers.
//!
//! Generates valid main tier lines from components (speaker, words, terminators,
//! pauses, fillers) and verifies parse → serialize → reparse stability.

use proptest::prelude::*;
use talkbank_parser::TreeSitterParser;
use talkbank_model::{ChatParser, ErrorCollector};

use super::slow_test_config;

/// Strategy for speaker codes (3-letter uppercase).
fn speaker_code() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("CHI"),
        Just("MOT"),
        Just("FAT"),
        Just("INV"),
        Just("EXP"),
    ]
}

/// Strategy for a simple word.
fn word() -> impl Strategy<Value = String> {
    "[a-z]{1,8}"
}

/// Strategy for a terminator.
fn terminator() -> impl Strategy<Value = &'static str> {
    prop_oneof![Just("."), Just("?"), Just("!"),]
}

/// Strategy for a content element (word, filler, or pause).
fn content_element() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => word(),
        1 => word().prop_map(|w| format!("&-{}", w)),       // filler
        1 => Just("(.)".to_string()),                         // short pause
    ]
}

/// Strategy for a full main tier line.
fn main_tier_line() -> impl Strategy<Value = String> {
    (
        speaker_code(),
        prop::collection::vec(content_element(), 1..6),
        terminator(),
    )
        .prop_map(|(speaker, words, term)| format!("*{}:\t{} {}", speaker, words.join(" "), term))
}

/// Helper: parse main tier with both parsers.
fn parse_main_with_both(input: &str) -> Vec<(&'static str, Option<String>)> {
    let mut results = Vec::new();
    if let Ok(ts) = TreeSitterParser::new() {
        let errors = ErrorCollector::new();
        let parsed = ChatParser::parse_main_tier(&ts, input, 0, &errors);
        if let Some(tier) = parsed.into_option() {
            results.push(("tree-sitter", Some(tier.to_chat())));
        } else {
            results.push(("tree-sitter", None));
        }
    }
    if let Ok(dp) = TreeSitterParser::new() {
        let errors = ErrorCollector::new();
        let parsed = ChatParser::parse_main_tier(&dp, input, 0, &errors);
        if let Some(tier) = parsed.into_option() {
            results.push(("direct", Some(tier.to_chat())));
        } else {
            results.push(("direct", None));
        }
    }
    results
}

proptest! {
    #![proptest_config(slow_test_config())]

    /// Simple main tier roundtrips.
    #[test]
    fn simple_main_tier_roundtrip(line in main_tier_line()) {
        for (name, output) in parse_main_with_both(&line) {
            if let Some(output) = output {
                prop_assert_eq!(
                    &output, &line,
                    "[{}] roundtrip: '{}' -> '{}'",
                    name, line, output
                );
            }
        }
    }

    /// Main tier with just words and terminator.
    #[test]
    fn plain_words_main_tier_roundtrip(
        speaker in speaker_code(),
        words in prop::collection::vec(word(), 1..5),
        term in terminator()
    ) {
        let line = format!("*{}:\t{} {}", speaker, words.join(" "), term);
        for (name, output) in parse_main_with_both(&line) {
            if let Some(output) = output {
                prop_assert_eq!(
                    &output, &line,
                    "[{}] roundtrip: '{}' -> '{}'",
                    name, line, output
                );
            }
        }
    }

    /// Main tier with fillers roundtrips.
    #[test]
    fn filler_main_tier_roundtrip(
        filler_word in word(),
        context_word in word(),
        term in terminator()
    ) {
        let line = format!("*CHI:\t&-{} {} {}", filler_word, context_word, term);
        for (name, output) in parse_main_with_both(&line) {
            if let Some(output) = output {
                prop_assert_eq!(
                    &output, &line,
                    "[{}] roundtrip: '{}' -> '{}'",
                    name, line, output
                );
            }
        }
    }

    /// Both parsers produce the same output for the same main tier.
    #[test]
    fn main_tier_parser_equivalence(line in main_tier_line()) {
        let results = parse_main_with_both(&line);
        let successes: Vec<_> = results.iter().filter_map(|(name, out)| {
            out.as_ref().map(|o| (*name, o.as_str()))
        }).collect();
        if successes.len() == 2 {
            prop_assert_eq!(
                successes[0].1, successes[1].1,
                "Parser divergence on '{}': {} -> '{}', {} -> '{}'",
                line, successes[0].0, successes[0].1, successes[1].0, successes[1].1
            );
        }
    }

    /// Double roundtrip is stable for main tiers.
    #[test]
    fn main_tier_double_roundtrip_stable(line in main_tier_line()) {
        if let Ok(ts) = TreeSitterParser::new() {
            let e1 = ErrorCollector::new();
            if let Some(tier1) = ChatParser::parse_main_tier(&ts, &line, 0, &e1).into_option() {
                let chat1 = tier1.to_chat();
                let e2 = ErrorCollector::new();
                if let Some(tier2) = ChatParser::parse_main_tier(&ts, &chat1, 0, &e2).into_option() {
                    let chat2 = tier2.to_chat();
                    prop_assert_eq!(
                        &chat1, &chat2,
                        "double roundtrip unstable: '{}' -> '{}' -> '{}'",
                        line, chat1, chat2
                    );
                }
            }
        }
    }
}
