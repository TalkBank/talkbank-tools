//! Domain-specific alignment-unit counting tests.
//!
//! These cases lock down domain policy differences (`%mor/%pho/%sin/%wor`) so
//! helper-level behavior remains stable across aligner refactors.

use super::{TierDomain, count_tier_positions};
use crate::Span;
use crate::model::{
    Annotated, BracketedContent, BracketedItem, Group, Pause, PauseDuration, PhoGroup,
    ReplacedWord, Replacement, Retrace, RetraceKind, Separator, SinGroup, UtteranceContent, Word,
    WordCategory,
};

/// Confirms `%mor` skips retraced items while `%pho` and `%wor` still count them.
#[test]
fn mor_skips_retrace_content() {
    let word = Word::new_unchecked("hello", "hello");
    let bracketed = BracketedContent::new(vec![BracketedItem::Word(Box::new(word))]);
    let retrace = Retrace::new(bracketed, RetraceKind::Full);
    let items = vec![UtteranceContent::Retrace(Box::new(retrace))];

    assert_eq!(count_tier_positions(&items, TierDomain::Mor), 0);
    assert_eq!(count_tier_positions(&items, TierDomain::Pho), 1);
    assert_eq!(count_tier_positions(&items, TierDomain::Wor), 1);
}

/// Confirms replacement branches contribute replacement-word counts in `%mor` and `%wor`.
#[test]
fn mor_counts_replacement_words() {
    let base = Word::new_unchecked("goed", "goed");
    let replacement = Replacement::new(vec![
        Word::new_unchecked("went", "went"),
        Word::new_unchecked("home", "home"),
    ]);
    let replaced = UtteranceContent::ReplacedWord(Box::new(ReplacedWord::new(base, replacement)));

    assert_eq!(
        count_tier_positions(std::slice::from_ref(&replaced), TierDomain::Mor),
        2
    );
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&replaced), TierDomain::Pho),
        1
    );
    // Wor uses replacement words (like Mor), matching Python batchalign's lexer
    // which substitutes replacement text. "goed [: went home]" → 2 wor items.
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&replaced), TierDomain::Wor),
        2
    );
}

/// Confirms untranscribed tokens are excluded from `%mor/%wor` but kept for `%pho`.
#[test]
fn mor_skips_untranscribed_but_pho_counts() {
    let word = Word::new_unchecked("xxx", "xxx");
    let items = vec![UtteranceContent::Word(Box::new(word))];

    // Mor skips untranscribed material (xxx, yyy, www) - no linguistic content
    assert_eq!(count_tier_positions(&items, TierDomain::Mor), 0);
    // Wor excludes untranscribed (matching Python batchalign's lexer filtering)
    assert_eq!(count_tier_positions(&items, TierDomain::Wor), 0);
    // Pho counts everything that was phonologically produced
    assert_eq!(count_tier_positions(&items, TierDomain::Pho), 1);
}

/// Confirms timestamp-shaped tokens are excluded from `%wor` counts.
#[test]
fn wor_skips_timestamp_tokens() {
    let items = vec![UtteranceContent::Word(Box::new(Word::new_unchecked(
        "100_200", "100_200",
    )))];

    // Timestamp-shaped tokens are %wor alignment metadata, not lexical tokens.
    assert_eq!(count_tier_positions(&items, TierDomain::Wor), 0);
    // Keep existing morphological behavior unchanged.
    assert_eq!(count_tier_positions(&items, TierDomain::Mor), 1);
}

/// Confirms `%mor` counts tag-marker separators (comma/tag/vocative) as alignable.
#[test]
fn mor_counts_tag_markers_including_comma() {
    let comma = UtteranceContent::Separator(Separator::Comma { span: Span::DUMMY });
    let colon = UtteranceContent::Separator(Separator::Colon { span: Span::DUMMY });
    let tag = UtteranceContent::Separator(Separator::Tag { span: Span::DUMMY });
    let vocative = UtteranceContent::Separator(Separator::Vocative { span: Span::DUMMY });

    // Comma counts as tag marker for mor (cm|cm in mor tier)
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&comma), TierDomain::Mor),
        1
    );
    // Colon does not count as tag marker
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&colon), TierDomain::Mor),
        0
    );
    // Tag and vocative count
    assert_eq!(count_tier_positions(&[tag, vocative], TierDomain::Mor), 2);
    // None count for Pho
    assert_eq!(
        count_tier_positions(&[comma.clone(), colon.clone()], TierDomain::Pho),
        0
    );
    assert_eq!(
        count_tier_positions(&[comma.clone(), colon], TierDomain::Wor),
        0
    );
}

/// Confirms retraced groups are skipped in `%mor` but counted for `%pho/%sin/%wor`.
#[test]
fn mor_skips_retrace_group_but_pho_sin_wor_count() {
    // Retraced groups skip ONLY for Mor (no morphological analysis for false starts)
    // but Pho/Sin/Wor count them (the content WAS produced phonologically and gets timed)
    let inner_words = vec![
        BracketedItem::Word(Box::new(Word::new_unchecked("hi", "hi"))),
        BracketedItem::Word(Box::new(Word::new_unchecked("there", "there"))),
    ];
    let bracketed = BracketedContent::new(inner_words);
    let retrace = Retrace::new(bracketed, RetraceKind::Full).as_group();
    let items = vec![UtteranceContent::Retrace(Box::new(retrace))];

    assert_eq!(count_tier_positions(&items, TierDomain::Mor), 0);
    assert_eq!(count_tier_positions(&items, TierDomain::Pho), 2);
    assert_eq!(count_tier_positions(&items, TierDomain::Sin), 2);
    assert_eq!(count_tier_positions(&items, TierDomain::Wor), 2); // Wor includes retraced content!
}

/// Confirms pauses contribute only to `%pho` unit counts.
#[test]
fn pho_counts_pause_but_other_domains_ignore() {
    let pause = UtteranceContent::Pause(Pause::new(PauseDuration::Short));
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&pause), TierDomain::Pho),
        1
    );
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&pause), TierDomain::Mor),
        0
    );
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&pause), TierDomain::Sin),
        0
    );
    // Pauses don't appear in %wor tiers - only words with timing bullets appear there
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&pause), TierDomain::Wor),
        0
    );
}

/// Confirms `%pho` groups count as one `%pho` unit but expand for `%mor/%wor`.
#[test]
fn pho_group_counts_as_single_unit_for_pho() {
    let inner = vec![
        BracketedItem::Word(Box::new(Word::new_unchecked("hi", "hi"))),
        BracketedItem::Word(Box::new(Word::new_unchecked("there", "there"))),
    ];
    let group = UtteranceContent::PhoGroup(PhoGroup::new(BracketedContent::new(inner)));
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&group), TierDomain::Pho),
        1
    );
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&group), TierDomain::Wor),
        2
    );
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&group), TierDomain::Mor),
        2
    );
}

/// Confirms `%sin` groups count as one `%sin` unit but expand for `%mor/%wor`.
#[test]
fn sin_group_counts_as_single_unit_for_sin() {
    let inner = vec![
        BracketedItem::Word(Box::new(Word::new_unchecked("hi", "hi"))),
        BracketedItem::Word(Box::new(Word::new_unchecked("there", "there"))),
    ];
    let group = UtteranceContent::SinGroup(SinGroup::new(BracketedContent::new(inner)));
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&group), TierDomain::Sin),
        1
    );
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&group), TierDomain::Wor),
        2
    );
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&group), TierDomain::Mor),
        2
    );
}

/// Confirms phonological fragments with replacements are excluded from `%pho`.
#[test]
fn pho_skips_fragment_with_replacement() {
    let base = Word::new_unchecked("&+fr", "fr").with_category(WordCategory::PhonologicalFragment);
    let replacement = Replacement::new(vec![Word::new_unchecked("word", "word")]);
    let replaced = UtteranceContent::ReplacedWord(Box::new(ReplacedWord::new(base, replacement)));

    assert_eq!(
        count_tier_positions(std::slice::from_ref(&replaced), TierDomain::Pho),
        0
    );
    // Wor uses replacement words (like Mor). For "&+fr [: friend]", the replacement
    // "friend" is a regular word that IS alignable, so Wor counts 1.
    assert_eq!(count_tier_positions(&[replaced], TierDomain::Wor), 1);
}

/// Confirms `%wor` excludes nonwords/fragments but keeps fillers.
#[test]
fn wor_excludes_nonwords_and_fragments_but_includes_fillers() {
    // Nonwords (&~gaga) are excluded from Wor (Python batchalign TokenType.ANNOT)
    let nonword = UtteranceContent::Word(Box::new(
        Word::new_unchecked("&~gaga", "gaga").with_category(WordCategory::Nonword),
    ));
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&nonword), TierDomain::Wor),
        0
    );

    // Fragments (&+fr) are excluded from Wor
    let fragment = UtteranceContent::Word(Box::new(
        Word::new_unchecked("&+fr", "fr").with_category(WordCategory::PhonologicalFragment),
    ));
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&fragment), TierDomain::Wor),
        0
    );

    // Fillers (&-um) are INCLUDED in Wor — they appear in %wor tiers
    let filler = UtteranceContent::Word(Box::new(
        Word::new_unchecked("&-um", "um").with_category(WordCategory::Filler),
    ));
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&filler), TierDomain::Wor),
        1
    );

    // Pho includes all of these (everything phonologically produced)
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&nonword), TierDomain::Pho),
        1
    );
    assert_eq!(
        count_tier_positions(std::slice::from_ref(&fragment), TierDomain::Pho),
        1
    );
}
