//! Structural roundtrip property tests for %gra tiers.
//!
//! Generates valid dependency graphs (index|head|relation triples) and verifies
//! parse → serialize → reparse stability across both parsers.

use proptest::prelude::*;
use talkbank_direct_parser::DirectParser;
use talkbank_model::{ChatParser, ErrorCollector};
use talkbank_parser::TreeSitterParser;

use super::slow_test_config;

/// Strategy for grammatical relation labels (common UD relations).
fn relation_label() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        Just("ROOT"),
        Just("SUBJ"),
        Just("OBJ"),
        Just("DET"),
        Just("PUNCT"),
        Just("NMOD"),
        Just("AMOD"),
        Just("IOBJ"),
        Just("COORD"),
        Just("JCT"),
        Just("PRED"),
        Just("AUX"),
        Just("NEG"),
        Just("QUANT"),
        Just("COM"),
    ]
}

/// Generate a valid gra tier content string with N relations.
///
/// Constraints:
/// - Indices are 1..=N (sequential)
/// - Exactly one ROOT (head=0 or relation=ROOT)
/// - Other heads are in 0..=N (0 means ROOT)
/// - Last relation is typically PUNCT
fn gra_content(n: usize) -> impl Strategy<Value = String> {
    // Generate a root position (1-indexed)
    let root_idx = 1..=n;
    // Generate heads for non-root positions
    let heads = prop::collection::vec(0..n, n);
    // Generate relation labels for non-root positions
    let labels = prop::collection::vec(relation_label(), n);

    (root_idx, heads, labels).prop_map(move |(root_pos, raw_heads, raw_labels)| {
        let mut parts = Vec::with_capacity(n);
        for i in 1..=n {
            if i == root_pos {
                parts.push(format!("{}|0|ROOT", i));
            } else {
                // Pick a head that isn't the current index (avoid trivial self-loops)
                let mut head = raw_heads[i - 1];
                if head == i {
                    head = 0; // Fall back to ROOT-linked
                }
                let label = raw_labels[i - 1];
                // Don't use ROOT label for non-root positions
                let label = if label == "ROOT" { "SUBJ" } else { label };
                parts.push(format!("{}|{}|{}", i, head, label));
            }
        }
        parts.join(" ")
    })
}

/// Helper: parse gra tier content with both parsers.
fn parse_gra_with_both(input: &str) -> Vec<(&'static str, Option<String>)> {
    let mut results = Vec::new();
    if let Ok(ts) = TreeSitterParser::new() {
        let errors = ErrorCollector::new();
        let parsed = ChatParser::parse_gra_tier(&ts, input, 0, &errors);
        if let Some(tier) = parsed.into_option() {
            results.push(("tree-sitter", Some(tier.to_content())));
        } else {
            results.push(("tree-sitter", None));
        }
    }
    if let Ok(dp) = DirectParser::new() {
        let errors = ErrorCollector::new();
        let parsed = ChatParser::parse_gra_tier(&dp, input, 0, &errors);
        if let Some(tier) = parsed.into_option() {
            results.push(("direct", Some(tier.to_content())));
        } else {
            results.push(("direct", None));
        }
    }
    results
}

proptest! {
    #![proptest_config(slow_test_config())]

    /// Small dependency graph (2-3 relations) roundtrips.
    #[test]
    fn small_gra_roundtrip(content in gra_content(2).prop_union(gra_content(3))) {
        for (name, output) in parse_gra_with_both(&content) {
            if let Some(output) = output {
                prop_assert_eq!(
                    &output, &content,
                    "[{}] roundtrip: '{}' -> '{}'",
                    name, content, output
                );
            }
        }
    }

    /// Medium dependency graph (4-6 relations) roundtrips.
    #[test]
    fn medium_gra_roundtrip(
        content in (4..=6usize).prop_flat_map(gra_content)
    ) {
        for (name, output) in parse_gra_with_both(&content) {
            if let Some(output) = output {
                prop_assert_eq!(
                    &output, &content,
                    "[{}] roundtrip: '{}' -> '{}'",
                    name, content, output
                );
            }
        }
    }

    /// Both parsers produce the same output for the same gra content.
    #[test]
    fn gra_parser_equivalence(content in gra_content(3).prop_union(gra_content(4))) {
        let results = parse_gra_with_both(&content);
        let successes: Vec<_> = results.iter().filter_map(|(name, out)| {
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

    /// Double roundtrip is stable for gra tiers.
    #[test]
    fn gra_double_roundtrip_stable(content in gra_content(3)) {
        if let Ok(ts) = TreeSitterParser::new() {
            let e1 = ErrorCollector::new();
            if let Some(tier1) = ChatParser::parse_gra_tier(&ts, &content, 0, &e1).into_option() {
                let chat1 = tier1.to_content();
                let e2 = ErrorCollector::new();
                if let Some(tier2) = ChatParser::parse_gra_tier(&ts, &chat1, 0, &e2).into_option() {
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
