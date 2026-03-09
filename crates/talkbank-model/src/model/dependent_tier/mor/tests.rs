//! `%mor` item and tier behavior tests.
//!
//! Focuses on chunk counting, post-clitic expansion, and serialization shape
//! guarantees used by downstream `%gra` alignment logic.

use super::*;
use crate::WriteChat;

/// Builds a single-chunk `%mor` item.
#[test]
fn test_mor_simple_word() {
    let word = MorWord::new(PosCategory::new("pron"), "I");
    let mor = Mor::new(word);

    assert_eq!(mor.count_chunks(), 1);
    assert!(mor.post_clitics.is_empty());
}

/// Adds one post-clitic chunk to a `%mor` item.
#[test]
fn test_mor_with_post_clitic() {
    let main_word = MorWord::new(PosCategory::new("pron"), "it");
    let post_word =
        MorWord::new(PosCategory::new("aux"), "be").with_feature(MorFeature::new("Fin"));

    let mor = Mor::new(main_word).with_post_clitic(post_word);

    assert_eq!(mor.count_chunks(), 2); // main + post-clitic
    assert_eq!(mor.post_clitics.len(), 1);
}

/// Serializes a `%mor` item with inflectional features.
#[test]
fn test_mor_write_chat() {
    let word = MorWord::new(PosCategory::new("noun"), "dog").with_feature(MorFeature::new("Plur"));
    let mor = Mor::new(word);
    assert_eq!(mor.to_chat_string(), "noun|dog-Plur");
}

/// Serializes `%mor` item plus post-clitic in CHAT format.
#[test]
fn test_mor_write_chat_with_post_clitic() {
    let main = MorWord::new(PosCategory::new("pron"), "she")
        .with_feature(MorFeature::new("Prs"))
        .with_feature(MorFeature::new("Nom"))
        .with_feature(MorFeature::new("S3"));
    let post = MorWord::new(PosCategory::new("aux"), "be")
        .with_feature(MorFeature::new("Fin"))
        .with_feature(MorFeature::new("Ind"))
        .with_feature(MorFeature::new("Pres"))
        .with_feature(MorFeature::new("S3"));
    let mor = Mor::new(main).with_post_clitic(post);
    assert_eq!(
        mor.to_chat_string(),
        "pron|she-Prs-Nom-S3~aux|be-Fin-Ind-Pres-S3"
    );
}

/// Preserves feature ordering on `MorWord`.
#[test]
fn test_mor_word_features() {
    let word = MorWord::new(PosCategory::new("verb"), "run")
        .with_feature(MorFeature::new("Fin"))
        .with_feature(MorFeature::new("Ind"))
        .with_feature(MorFeature::new("Pres"))
        .with_feature(MorFeature::new("S3"));

    assert_eq!(word.pos.as_str(), "verb");
    assert_eq!(word.lemma.as_str(), "run");
    assert_eq!(word.features.len(), 4);
    assert_eq!(word.features[0].value(), "Fin");
}

/// Counts total `%mor` chunks across items and clitics.
#[test]
fn test_mor_tier_count_chunks() {
    // word1: 1 chunk, word2 with post-clitic: 2 chunks = 3 total
    let word1 = MorWord::new(PosCategory::new("verb"), "go");
    let mor1 = Mor::new(word1);

    let main = MorWord::new(PosCategory::new("pron"), "it");
    let post = MorWord::new(PosCategory::new("aux"), "be");
    let mor2 = Mor::new(main).with_post_clitic(post);

    let tier = MorTier::new_mor(vec![mor1, mor2]);

    assert_eq!(tier.count_chunks(), 3);
}
