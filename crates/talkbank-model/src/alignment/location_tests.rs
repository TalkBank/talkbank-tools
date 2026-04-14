//! Regression tests for alignment error-location reporting.
//!
//! The alignment layer intentionally reports count mismatches at the main-tier
//! span so editors can highlight the primary utterance that needs repair.
//!
//! CHAT references:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Phonology_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Word_Timing_Tier>

use super::*;
use crate::model::{
    Bullet, MainTier, Mor, MorTier, MorTierType, MorWord, PhoItem, PhoTier, PhoTierType, PhoWord,
    PosCategory, SinItem, SinTier, SinToken, Terminator, UtteranceContent, WorTier, Word,
    WordCategory,
};
use crate::{ErrorCode, Span};

/// Anchors `%pho` underflow errors to the main-tier span.
///
/// When `%pho` has too few items, diagnostics should still point at the primary
/// tier as the canonical repair location.
#[test]
fn test_pho_alignment_error_has_proper_location() {
    // Main tier has 2 words, pho tier has only 1 - should error
    let main = MainTier::new(
        "CHI",
        vec![
            UtteranceContent::Word(Box::new(Word::new_unchecked("one", "one"))),
            UtteranceContent::Word(Box::new(Word::new_unchecked("two", "two"))),
        ],
        Terminator::Period { span: Span::DUMMY },
    )
    .with_span(Span::from_usize(0, 20)); // *CHI: one two .

    let pho = PhoTier::new(PhoTierType::Pho, vec![PhoItem::Word(PhoWord::new("wan"))])
        .with_span(Span::from_usize(21, 35)); // %pho: wan

    let alignment = align_main_to_pho(&main, &pho);

    assert!(!alignment.is_error_free());
    assert_eq!(alignment.errors.len(), 1);

    let error = &alignment.errors[0];
    assert_eq!(error.code, ErrorCode::PhoCountMismatchTooFew);

    // The error should point to the main tier (which has more content than pho tier)
    assert_eq!(error.location.span.start, 0);
    assert_eq!(error.location.span.end, 20);
}

/// Anchors `%pho` overflow errors to the main-tier span as well.
///
/// Even when dependent tiers overproduce entries, the reported location remains
/// the enclosing main utterance span.
#[test]
fn test_pho_alignment_error_too_many_has_proper_location() {
    // Pho tier has 2 items, main tier has only 1 - should error
    let main = MainTier::new(
        "CHI",
        vec![UtteranceContent::Word(Box::new(Word::new_unchecked(
            "one", "one",
        )))],
        Terminator::Period { span: Span::DUMMY },
    )
    .with_span(Span::from_usize(0, 15)); // *CHI: one .

    let pho = PhoTier::new(
        PhoTierType::Pho,
        vec![
            PhoItem::Word(PhoWord::new("wan")),
            PhoItem::Word(PhoWord::new("tu")),
        ],
    )
    .with_span(Span::from_usize(16, 35)); // %pho: wan tu

    let alignment = align_main_to_pho(&main, &pho);

    assert!(!alignment.is_error_free());
    assert_eq!(alignment.errors.len(), 1);

    let error = &alignment.errors[0];
    assert_eq!(error.code, ErrorCode::PhoCountMismatchTooMany);

    // The error should point to the main tier (enclosing tier is always primary)
    assert_eq!(error.location.span.start, 0);
    assert_eq!(error.location.span.end, 15);
}

/// Anchors `%sin` underflow errors to the main-tier span.
///
/// This mirrors `%pho` behavior so UI tooling gets consistent error locations
/// across dependent tiers.
#[test]
fn test_sin_alignment_error_has_proper_location() {
    // Main tier has 2 words, sin tier has only 1 - should error
    let main = MainTier::new(
        "CHI",
        vec![
            UtteranceContent::Word(Box::new(Word::new_unchecked("one", "one"))),
            UtteranceContent::Word(Box::new(Word::new_unchecked("two", "two"))),
        ],
        Terminator::Period { span: Span::DUMMY },
    )
    .with_span(Span::from_usize(0, 20)); // *CHI: one two .

    let sin = SinTier::new(vec![SinItem::Token(SinToken::new_unchecked(
        "g:toy:dpoint",
    ))])
    .with_span(Span::from_usize(21, 45)); // %sin: g:toy:dpoint

    let alignment = align_main_to_sin(&main, &sin);

    assert!(!alignment.is_error_free());
    assert_eq!(alignment.errors.len(), 1);

    let error = &alignment.errors[0];
    assert_eq!(error.code, ErrorCode::SinCountMismatchTooFew);

    // The error should point to the main tier
    assert_eq!(error.location.span.start, 0);
    assert_eq!(error.location.span.end, 20);
}

/// Anchors `%sin` overflow errors to the main-tier span.
///
/// The invariant is that count mismatches are always reported on the primary
/// utterance tier, regardless of mismatch direction.
#[test]
fn test_sin_alignment_error_too_many_has_proper_location() {
    // Sin tier has 2 items, main tier has only 1 - should error
    let main = MainTier::new(
        "CHI",
        vec![UtteranceContent::Word(Box::new(Word::new_unchecked(
            "one", "one",
        )))],
        Terminator::Period { span: Span::DUMMY },
    )
    .with_span(Span::from_usize(0, 15)); // *CHI: one .

    let sin = SinTier::new(vec![
        SinItem::Token(SinToken::new_unchecked("g:toy:dpoint")),
        SinItem::Token(SinToken::new_unchecked("0")),
    ])
    .with_span(Span::from_usize(16, 45)); // %sin: g:toy:dpoint 0

    let alignment = align_main_to_sin(&main, &sin);

    assert!(!alignment.is_error_free());
    assert_eq!(alignment.errors.len(), 1);

    let error = &alignment.errors[0];
    assert_eq!(error.code, ErrorCode::SinCountMismatchTooMany);

    // The error should point to the main tier (enclosing tier is always primary)
    assert_eq!(error.location.span.start, 0);
    assert_eq!(error.location.span.end, 15);
}

/// `%wor` count mismatches must never emit validation errors.
///
/// `%wor` is a timing-annotation tier, not a structural mirror of the main tier.
/// Stale word counts are common when a transcript is edited without re-aligning.
/// The validator must never flag `%wor` count differences — the alignment pairs
/// are still computed for the batchalign injection layer, but the errors list is
/// always empty.
#[test]
fn test_wor_alignment_never_emits_errors_on_count_mismatch() {
    // Main tier has 2 words, wor tier has only 1 — mismatched, but errors must be empty.
    let main = MainTier::new(
        "CHI",
        vec![
            UtteranceContent::Word(Box::new(Word::new_unchecked("one", "one"))),
            UtteranceContent::Word(Box::new(Word::new_unchecked("two", "two"))),
        ],
        Terminator::Period { span: Span::DUMMY },
    )
    .with_span(Span::from_usize(0, 20));

    let wor = WorTier::from_words(vec![Word::new_unchecked("one", "one")])
        .with_span(Span::from_usize(21, 35));

    let alignment = align_main_to_wor(&main, &wor);

    assert!(
        alignment.errors.is_empty(),
        "%wor alignment must never emit validation errors; \
         stale word counts are expected after transcript edits. \
         Got: {:?}",
        alignment.errors
    );
}

/// `%wor` overflow mismatches also produce no errors.
#[test]
fn test_wor_alignment_never_emits_errors_on_too_many() {
    // Wor tier has 3 words, main tier has only 1 — no errors expected.
    let main = MainTier::new(
        "CHI",
        vec![UtteranceContent::Word(Box::new(Word::new_unchecked(
            "one", "one",
        )))],
        Terminator::Period { span: Span::DUMMY },
    )
    .with_span(Span::from_usize(0, 15));

    let wor = WorTier::from_words(vec![
        Word::new_unchecked("one", "one"),
        Word::new_unchecked("two", "two"),
        Word::new_unchecked("three", "three"),
    ])
    .with_span(Span::from_usize(16, 45));

    let alignment = align_main_to_wor(&main, &wor);

    assert!(
        alignment.errors.is_empty(),
        "%wor alignment must never emit validation errors (too-many case). \
         Got: {:?}",
        alignment.errors
    );
}

/// Confirms `%wor` alignment excludes terminators from token-count matching.
///
/// Terminators are utterance structure, not word-level timing units, so a
/// matching word count should align cleanly even when terminators differ in
/// bullet behavior.
#[test]
fn test_wor_alignment_does_not_count_terminator() {
    // This is the real-world case: main tier has 13 words + terminator
    // wor tier has 13 words with timing bullets + terminator (no bullet for terminator)
    // This should NOT produce an error because terminators don't get timing bullets

    let main = MainTier::new(
        "MOT",
        vec![
            UtteranceContent::Word(Box::new(Word::simple("in"))),
            UtteranceContent::Word(Box::new(Word::simple("the"))),
            UtteranceContent::Word(Box::new(Word::simple("light"))),
            UtteranceContent::Word(Box::new(Word::simple("of"))),
            UtteranceContent::Word(Box::new(Word::simple("the"))),
            UtteranceContent::Word(Box::new(Word::simple("moon"))),
            UtteranceContent::Word(Box::new(Word::simple("a"))),
            UtteranceContent::Word(Box::new(Word::simple("little"))),
            UtteranceContent::Word(Box::new(Word::simple("egg"))),
            UtteranceContent::Word(Box::new(Word::simple("lay"))),
            UtteranceContent::Word(Box::new(Word::simple("on"))),
            UtteranceContent::Word(Box::new(Word::simple("a"))),
            UtteranceContent::Word(Box::new(Word::simple("leaf"))),
        ],
        Terminator::Period { span: Span::DUMMY }, // <-- This terminator should NOT be counted for %wor alignment
    )
    .with_span(Span::from_usize(0, 100));

    // The %wor tier also has 13 words + terminator
    // Each word has a timing bullet, but the terminator does NOT
    let wor = WorTier::from_words(vec![
        Word::simple("in"),
        Word::simple("the"),
        Word::simple("light"),
        Word::simple("of"),
        Word::simple("the"),
        Word::simple("moon"),
        Word::simple("a"),
        Word::simple("little"),
        Word::simple("egg"),
        Word::simple("lay"),
        Word::simple("on"),
        Word::simple("a"),
        Word::simple("leaf"),
    ])
    .with_terminator(Some(Terminator::Period { span: Span::DUMMY })) // <-- Terminator present but no timing bullet
    .with_span(Span::from_usize(101, 200));

    let alignment = align_main_to_wor(&main, &wor);

    // Should be error-free: 13 words on main tier match 13 words on %wor tier
    // The terminator is structural and doesn't participate in word-level timing alignment
    assert!(
        alignment.is_error_free(),
        "Expected no errors, but got: {:?}",
        alignment.errors
    );
    assert_eq!(alignment.pairs.len(), 13); // 13 word-to-word pairs
}

/// Confirms `%wor` alignment accepts timed filler words copied from the main tier.
///
/// Brian's OCSC report boils down to this shape: the main tier has a filler
/// like `&-dt`, and `%wor` carries one timed token for that spoken material.
/// That should align cleanly.
#[test]
fn test_wor_alignment_allows_timed_fillers() {
    let main = MainTier::new(
        "PAR",
        vec![
            UtteranceContent::Word(Box::new(
                Word::new_unchecked("&-dt", "dt").with_category(WordCategory::Filler),
            )),
            UtteranceContent::Word(Box::new(Word::simple("there"))),
        ],
        Terminator::Period { span: Span::DUMMY },
    )
    .with_span(Span::from_usize(0, 24)); // *PAR: &-dt there .

    let wor = WorTier::from_words(vec![
        Word::simple("dt").with_inline_bullet(Bullet::new(0, 120)),
        Word::simple("there").with_inline_bullet(Bullet::new(120, 260)),
    ])
    .with_span(Span::from_usize(25, 55)); // %wor: dt 0_120 there 120_260

    let alignment = align_main_to_wor(&main, &wor);

    assert!(
        alignment.is_error_free(),
        "Expected timed filler to align in %wor, got: {:?}",
        alignment.errors
    );
    assert_eq!(alignment.pairs.len(), 2);
}

/// Helper: build a simple Mor item from POS and lemma strings.
fn simple_mor(pos: &str, lemma: &str) -> Mor {
    Mor::new(MorWord::new(PosCategory::new(pos), lemma))
}

/// Anchors `%mor` underflow errors (E705) to the main-tier span.
///
/// When the main tier has more alignable items than `%mor`, the primary
/// span is the main utterance so editors highlight the authoritative source.
#[test]
fn test_mor_alignment_error_too_few_has_proper_location() {
    // Main tier has 2 words, mor tier has only 1 → E705
    let main = MainTier::new(
        "CHI",
        vec![
            UtteranceContent::Word(Box::new(Word::new_unchecked("one", "one"))),
            UtteranceContent::Word(Box::new(Word::new_unchecked("two", "two"))),
        ],
        Terminator::Period { span: Span::DUMMY },
    )
    .with_span(Span::from_usize(0, 20)); // *CHI: one two .

    let mor = MorTier::new(MorTierType::Mor, vec![simple_mor("num", "one")])
        .with_span(Span::from_usize(21, 35)) // %mor: num|one
        .with_terminator(Some(".".into()));

    let alignment = align_main_to_mor(&main, &mor);

    assert!(!alignment.is_error_free());
    assert_eq!(alignment.errors.len(), 1);

    let error = &alignment.errors[0];
    assert_eq!(error.code, ErrorCode::new("E705"));

    // Primary span should be the main tier
    assert_eq!(error.location.span.start, 0);
    assert_eq!(error.location.span.end, 20);

    // Should have labels for both tiers
    assert!(
        error.labels.len() >= 2,
        "Expected labels for both main and mor tiers, got {}",
        error.labels.len()
    );
}

/// Anchors `%mor` overflow errors (E706) to the main-tier span.
///
/// Even when the dependent tier has more items, the primary location
/// remains the main utterance for consistency with other tier aligners.
#[test]
fn test_mor_alignment_error_too_many_has_proper_location() {
    // Mor tier has 2 items, main tier has only 1 → E706
    let main = MainTier::new(
        "CHI",
        vec![UtteranceContent::Word(Box::new(Word::new_unchecked(
            "one", "one",
        )))],
        Terminator::Period { span: Span::DUMMY },
    )
    .with_span(Span::from_usize(0, 15)); // *CHI: one .

    let mor = MorTier::new(
        MorTierType::Mor,
        vec![simple_mor("num", "one"), simple_mor("num", "two")],
    )
    .with_span(Span::from_usize(16, 40)) // %mor: num|one num|two
    .with_terminator(Some(".".into()));

    let alignment = align_main_to_mor(&main, &mor);

    assert!(!alignment.is_error_free());
    assert_eq!(alignment.errors.len(), 1);

    let error = &alignment.errors[0];
    assert_eq!(error.code, ErrorCode::new("E706"));

    // Primary span should be the main tier (consistent with pho/sin/wor)
    assert_eq!(error.location.span.start, 0);
    assert_eq!(error.location.span.end, 15);

    // Should have labels for both tiers
    assert!(
        error.labels.len() >= 2,
        "Expected labels for both main and mor tiers, got {}",
        error.labels.len()
    );
}

/// Verifies that E706 errors have no bogus ErrorContext with empty source text.
///
/// The alignment module does not have access to the source text, so it must
/// create errors with `context: None` (via `at_span`), not with a dummy
/// `ErrorContext { source_text: "", span: <absolute bytes> }`.
#[test]
fn test_mor_alignment_errors_have_no_bogus_context() {
    let main = MainTier::new(
        "CHI",
        vec![UtteranceContent::Word(Box::new(Word::new_unchecked(
            "one", "one",
        )))],
        Terminator::Period { span: Span::DUMMY },
    )
    .with_span(Span::from_usize(0, 15));

    let mor = MorTier::new(
        MorTierType::Mor,
        vec![simple_mor("num", "one"), simple_mor("num", "two")],
    )
    .with_span(Span::from_usize(16, 40))
    .with_terminator(Some(".".into()));

    let alignment = align_main_to_mor(&main, &mor);

    for error in &alignment.errors {
        // context should be None (no source text available at alignment time),
        // NOT Some(ErrorContext { source_text: "", ... })
        assert!(
            error.context.is_none(),
            "Alignment error should not have a dummy ErrorContext; \
             source context is populated later by enhance_errors_with_source"
        );
    }
}
