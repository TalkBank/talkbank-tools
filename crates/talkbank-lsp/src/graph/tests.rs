//! Regression tests for `%mor`/`%gra` DOT graph generation.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use super::generate_dot_graph;

/// Tests missing alignment metadata.
#[test]
fn test_missing_alignment_metadata() {
    let main_tier = talkbank_model::model::MainTier::new("CHI", vec![], None);
    let mor_tier = talkbank_model::model::MorTier::new_mor(vec![]);
    let gra_tier = talkbank_model::model::GraTier::new_gra(vec![]);

    let utterance = talkbank_model::model::Utterance::new(main_tier)
        .with_mor(mor_tier)
        .with_gra(gra_tier);

    let result = generate_dot_graph(&utterance);
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.contains("alignment"));
    }
}

/// Tests missing gra alignment.
#[test]
fn test_missing_gra_alignment() {
    let main_tier = talkbank_model::model::MainTier::new("CHI", vec![], None);
    let mor_tier = talkbank_model::model::MorTier::new_mor(vec![]);
    let gra_tier = talkbank_model::model::GraTier::new_gra(vec![]);

    let mut utterance = talkbank_model::model::Utterance::new(main_tier)
        .with_mor(mor_tier)
        .with_gra(gra_tier);

    utterance.alignments = Some(talkbank_model::model::AlignmentSet::new(
        talkbank_model::model::AlignmentUnits::default(),
    ));

    let result = generate_dot_graph(&utterance);
    assert!(result.is_err());
    if let Err(err) = result {
        assert!(err.contains("%gra alignment"));
    }
}

/// Tests dot format structure with alignment.
#[test]
fn test_dot_format_structure_with_alignment() -> Result<(), String> {
    let main_tier = talkbank_model::model::MainTier::new("CHI", vec![], None);

    let word = talkbank_model::model::MorWord::new("n", "cat");
    let mor = talkbank_model::model::Mor::new(word);
    let mor_tier = talkbank_model::model::MorTier::new_mor(vec![mor]);

    let gra_tier = talkbank_model::model::GraTier::new_gra(vec![
        talkbank_model::model::GrammaticalRelation::new(1, 0, "ROOT"),
    ]);

    use talkbank_model::alignment::{GraAlignment, GraAlignmentPair};

    let gra_alignment = GraAlignment::new().with_pair(GraAlignmentPair::new(Some(0), Some(0)));

    let mut alignment_metadata =
        talkbank_model::model::AlignmentSet::new(talkbank_model::model::AlignmentUnits::default());
    alignment_metadata.gra = Some(gra_alignment);

    let mut utterance = talkbank_model::model::Utterance::new(main_tier)
        .with_mor(mor_tier)
        .with_gra(gra_tier);
    utterance.alignments = Some(alignment_metadata);

    let dot =
        generate_dot_graph(&utterance).map_err(|err| format!("Failed to generate graph: {err}"))?;

    assert!(dot.contains("digraph utterance"));
    assert!(dot.contains("rankdir=LR"));
    assert!(dot.contains("cat"));
    assert!(
        dot.contains("1 -> 0"),
        "Expected edge from node 1 to ROOT (0)"
    );
    assert!(
        dot.contains("label=\"ROOT\""),
        "Expected ROOT label on edge"
    );
    Ok(())
}
