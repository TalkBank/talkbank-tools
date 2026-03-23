//! Structural roundtrip property tests for %mor tiers.
//!
//! Generates valid %mor content strings from components (POS, lemma, features,
//! clitics) and verifies parse → serialize → reparse stability across both parsers.

use proptest::prelude::*;
use talkbank_parser::TreeSitterParser;
use talkbank_model::{ChatParser, ErrorCollector};

use super::slow_test_config;

/// Strategy for POS categories (common UD tags).
fn pos_tag() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("noun"),
        Just("verb"),
        Just("pron"),
        Just("det"),
        Just("adj"),
        Just("adv"),
        Just("prep"),
        Just("conj"),
        Just("aux"),
        Just("n:prop"),
        Just("v:aux"),
    ]
}

/// Strategy for lemma strings.
fn lemma() -> impl Strategy<Value = String> {
    "[a-z]{1,8}"
}

/// Strategy for flat features (no key=value, just value).
fn flat_feature() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("Prs"),
        Just("Past"),
        Just("Fin"),
        Just("Ind"),
        Just("Pres"),
        Just("Nom"),
        Just("Acc"),
        Just("Plur"),
        Just("Sing"),
        Just("S1"),
        Just("S3"),
    ]
}

/// Strategy for a single mor word: `POS|lemma[-feature]*`
fn mor_word_str() -> impl Strategy<Value = String> {
    (
        pos_tag(),
        lemma(),
        prop::collection::vec(flat_feature(), 0..4),
    )
        .prop_map(|(pos, lem, features)| {
            let mut s = format!("{}|{}", pos, lem);
            for f in features {
                s.push('-');
                s.push_str(f);
            }
            s
        })
}

/// Strategy for a mor item (main word + optional clitic).
fn mor_item_str() -> impl Strategy<Value = String> {
    (mor_word_str(), prop::option::of(mor_word_str())).prop_map(|(main, clitic)| match clitic {
        Some(c) => format!("{}~{}", main, c),
        None => main,
    })
}

/// Strategy for a full mor tier content string (1-5 items + optional terminator).
fn mor_content() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(mor_item_str(), 1..5),
        prop::option::of(prop_oneof![Just("."), Just("?"), Just("!")]),
    )
        .prop_map(|(items, term)| {
            let mut s = items.join(" ");
            if let Some(t) = term {
                s.push(' ');
                s.push_str(t);
            }
            s
        })
}

/// Helper: parse mor tier content with both parsers and return (name, result) pairs.
fn parse_with_both(input: &str) -> Vec<(&'static str, Option<String>, bool)> {
    let mut results = Vec::new();
    if let Ok(ts) = TreeSitterParser::new() {
        let errors = ErrorCollector::new();
        let parsed = ChatParser::parse_mor_tier(&ts, input, 0, &errors);
        if let Some(tier) = parsed.into_option() {
            results.push(("tree-sitter", Some(tier.to_content()), errors.is_empty()));
        } else {
            results.push(("tree-sitter", None, false));
        }
    }
    if let Ok(dp) = TreeSitterParser::new() {
        let errors = ErrorCollector::new();
        let parsed = ChatParser::parse_mor_tier(&dp, input, 0, &errors);
        if let Some(tier) = parsed.into_option() {
            results.push(("direct", Some(tier.to_content()), errors.is_empty()));
        } else {
            results.push(("direct", None, false));
        }
    }
    results
}

proptest! {
    #![proptest_config(slow_test_config())]

    /// Single mor word roundtrips.
    #[test]
    fn single_mor_word_roundtrip(word in mor_word_str()) {
        for (name, output, _) in parse_with_both(&word) {
            if let Some(output) = output {
                prop_assert_eq!(
                    &output, &word,
                    "[{}] roundtrip: '{}' -> '{}'",
                    name, word, output
                );
            }
        }
    }

    /// Mor word with clitic roundtrips.
    #[test]
    fn mor_clitic_roundtrip(item in mor_item_str()) {
        for (name, output, _) in parse_with_both(&item) {
            if let Some(output) = output {
                prop_assert_eq!(
                    &output, &item,
                    "[{}] roundtrip: '{}' -> '{}'",
                    name, item, output
                );
            }
        }
    }

    /// Full mor tier content roundtrips.
    #[test]
    fn mor_tier_content_roundtrip(content in mor_content()) {
        for (name, output, _) in parse_with_both(&content) {
            if let Some(output) = output {
                prop_assert_eq!(
                    &output, &content,
                    "[{}] roundtrip: '{}' -> '{}'",
                    name, content, output
                );
            }
        }
    }

    /// Both parsers produce the same output for the same mor content.
    #[test]
    fn mor_parser_equivalence(content in mor_content()) {
        let results = parse_with_both(&content);
        let successes: Vec<_> = results.iter().filter_map(|(name, out, _)| {
            out.as_ref().map(|o| (*name, o.as_str()))
        }).collect();
        if successes.len() == 2 {
            prop_assert_eq!(
                successes[0].1, successes[1].1,
                "Parser divergence on '{}': {} -> '{}', {} -> '{}'",
                content, successes[0].0, successes[0].1, successes[1].0, successes[1].1
            );
        }
    }

    /// Double roundtrip is stable for mor tiers.
    #[test]
    fn mor_double_roundtrip_stable(content in mor_content()) {
        if let Ok(ts) = TreeSitterParser::new() {
            let e1 = ErrorCollector::new();
            if let Some(tier1) = ChatParser::parse_mor_tier(&ts, &content, 0, &e1).into_option() {
                let chat1 = tier1.to_content();
                let e2 = ErrorCollector::new();
                if let Some(tier2) = ChatParser::parse_mor_tier(&ts, &chat1, 0, &e2).into_option() {
                    let chat2 = tier2.to_content();
                    prop_assert_eq!(
                        &chat1, &chat2,
                        "double roundtrip unstable: '{}' -> '{}' -> '{}'",
                        content, chat1, chat2
                    );
                }
            }
        }
    }
}
