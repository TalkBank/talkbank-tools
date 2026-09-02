//! `enforce_monotonicity_with_policy`'s same-speaker end-overlap classification:
//! `EndOverlapResolution` and the three `MonotonicityEffect::EndClamped*`
//! variants it produces. The defect these tests pin: a same-speaker overlap
//! used to be resolved on the bullet alone, after `%wor` had been written,
//! which left `%wor` and main-tier word timings stale relative to it.

#![allow(unused_imports, dead_code)]

use super::*;

use crate::chat_ops::fa::orchestrate::{EndOverlapResolution, classify_end_overlap};
use talkbank_model::model::dependent_tier::WorItem;

/// The (start_ms, end_ms) of the `n`th `%wor` word in `utterance`'s `%wor`
/// tier, or `None` when that slot has no timing (or the tier/index doesn't
/// exist).
fn wor_word_timing(
    chat: &talkbank_model::model::ChatFile,
    utt_idx: usize,
    word_idx: usize,
) -> Option<(u64, u64)> {
    let utt = get_utterance(chat, utt_idx);
    let wor = utt.wor_tier()?;
    let mut seen = 0usize;
    for item in &wor.items {
        let WorItem::Word(word) = item else {
            continue;
        };
        if seen == word_idx {
            return word
                .inline_bullet
                .as_ref()
                .map(|b| (b.timing.start_ms, b.timing.end_ms));
        }
        seen += 1;
    }
    None
}

/// Case 1: the previous utterance's word hull already ends before the next
/// utterance's start. Only the bullet's coverage overshot; the bullet end
/// moves to the hull, and the word is untouched.
#[test]
fn coverage_only_clamps_bullet_to_hull_leaves_words_untouched() {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|x|CHI|||||Child|||\n*CHI:\thello . \u{15}1000_5000\u{15}\n%wor:\thello \u{15}1000_1500\u{15} .\n*CHI:\tnext . \u{15}2000_6000\u{15}\n@End\n";
    let mut chat = parse_chat(input);

    let result = enforce_monotonicity(&mut chat);

    assert_eq!(
        get_utterance_bullet(&chat, 0),
        Some((1000, 1500)),
        "bullet end should be pulled back to the word hull, not the next start"
    );
    assert_eq!(
        wor_word_timing(&chat, 0, 0),
        Some((1000, 1500)),
        "the word itself is untouched by a coverage-only clamp"
    );
    assert!(matches!(
        result.effects(),
        [MonotonicityEffect::EndClampedCoverageOnly {
            clamped_to_ms: 1500,
            ..
        }]
    ));
    assert!(!result.records()[0].needs_review, "bookkeeping only");
}

/// Case 1 without any words at all: falls back to the old bookkeeping clamp
/// (bullet end to next start) exactly as before this fix.
#[test]
fn coverage_only_with_no_words_clamps_to_next_start() {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|x|CHI|||||Child|||\n*CHI:\thello . \u{15}1000_5000\u{15}\n*CHI:\tnext . \u{15}4000_6000\u{15}\n@End\n";
    let mut chat = parse_chat(input);

    let result = enforce_monotonicity(&mut chat);

    assert_eq!(get_utterance_bullet(&chat, 0), Some((1000, 4000)));
    assert!(matches!(
        result.effects(),
        [MonotonicityEffect::EndClampedCoverageOnly {
            clamped_to_ms: 4000,
            ..
        }]
    ));
}

/// Case 2: the previous utterance's hull ends after the next start, but the
/// two utterances' words do not interleave. Both bullets take their
/// measured hull edges; neither word is touched.
#[test]
fn boundary_from_words_replaces_both_bullet_edges_leaves_words_untouched() {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|x|CHI|||||Child|||\n*CHI:\thello . \u{15}1000_5000\u{15}\n%wor:\thello \u{15}1000_3000\u{15} .\n*CHI:\tworld . \u{15}2000_6000\u{15}\n%wor:\tworld \u{15}3500_6000\u{15} .\n@End\n";
    let mut chat = parse_chat(input);

    let result = enforce_monotonicity(&mut chat);

    assert_eq!(
        get_utterance_bullet(&chat, 0),
        Some((1000, 3000)),
        "previous bullet end replaced by its measured hull end"
    );
    assert_eq!(
        get_utterance_bullet(&chat, 1),
        Some((3500, 6000)),
        "next bullet start replaced by its measured hull start"
    );
    assert_eq!(
        wor_word_timing(&chat, 0, 0),
        Some((1000, 3000)),
        "the previous word is untouched"
    );
    assert_eq!(
        wor_word_timing(&chat, 1, 0),
        Some((3500, 6000)),
        "the next word is untouched"
    );
    assert!(matches!(
        result.effects(),
        [MonotonicityEffect::EndClampedBoundaryFromWords {
            prev_hull_end_ms: 3000,
            next_hull_start_ms: 3500,
            ..
        }]
    ));
    assert!(
        !result.records()[0].needs_review,
        "a measurable boundary needs no human review"
    );
}

/// Case 3: the two utterances' words genuinely overlap in time. The bullet
/// is clamped to the next start and every previous-utterance word past that
/// bound is clamped with it (here, on `%wor`, since these words carry no
/// main-tier inline bullet), flagged for review.
#[test]
fn interleaved_words_clamps_bullet_and_every_word_past_the_bound() {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|x|CHI|||||Child|||\n*CHI:\thello world . \u{15}1000_5000\u{15}\n%wor:\thello \u{15}1000_3000\u{15} world \u{15}3000_5000\u{15} .\n*CHI:\tnext . \u{15}4000_6000\u{15}\n%wor:\tnext \u{15}4000_6000\u{15} .\n@End\n";
    let mut chat = parse_chat(input);

    let result = enforce_monotonicity(&mut chat);

    assert_eq!(
        get_utterance_bullet(&chat, 0),
        Some((1000, 4000)),
        "bullet clamped to the next utterance's start, as before this fix"
    );
    assert_eq!(
        wor_word_timing(&chat, 0, 0),
        Some((1000, 3000)),
        "a word entirely before the bound is untouched"
    );
    assert_eq!(
        wor_word_timing(&chat, 0, 1),
        Some((3000, 4000)),
        "a word straddling the bound is clamped to it, not dropped"
    );
    let clamp = result
        .records()
        .iter()
        .find(|d| d.strategy.strategy_name() == "end_clamped_interleaved_words")
        .expect("overlap should produce an end clamp");
    assert!(clamp.needs_review, "a real word conflict needs review");
    assert!(matches!(
        result.effects(),
        [MonotonicityEffect::EndClampedInterleavedWords {
            words_clamped: 1,
            ..
        }]
    ));
}

/// Case 3, the second trigger: the next utterance has no measured word at
/// all to fix a boundary against, so the overlap cannot be resolved from
/// measurement alone even though the previous utterance's own words don't
/// extend past a plausible boundary on their own.
#[test]
fn interleaved_words_when_next_utterance_has_no_measured_words() {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|x|CHI|||||Child|||\n*CHI:\thello world . \u{15}1000_5000\u{15}\n%wor:\thello \u{15}1000_3000\u{15} world \u{15}3000_5000\u{15} .\n*CHI:\tnext . \u{15}4000_6000\u{15}\n@End\n";
    let mut chat = parse_chat(input);

    let result = enforce_monotonicity(&mut chat);

    assert_eq!(get_utterance_bullet(&chat, 0), Some((1000, 4000)));
    assert_eq!(wor_word_timing(&chat, 0, 1), Some((3000, 4000)));
    assert!(matches!(
        result.effects(),
        [MonotonicityEffect::EndClampedInterleavedWords { .. }]
    ));
}

/// A cross-speaker pair under `PreserveCrossSpeaker` is left entirely
/// alone: no classification runs, no bullet or word moves, no decision is
/// recorded.
#[test]
fn cross_speaker_pair_is_untouched_under_preserve_policy() {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child, MOT Mother\n@ID:\teng|x|CHI|||||Child|||\n@ID:\teng|x|MOT|||||Mother|||\n*CHI:\thello world . \u{15}1000_5000\u{15}\n%wor:\thello \u{15}1000_3000\u{15} world \u{15}3000_5000\u{15} .\n*MOT:\tnext . \u{15}4000_6000\u{15}\n@End\n";
    let mut chat = parse_chat(input);

    let result =
        enforce_monotonicity_with_policy(&mut chat, EndOverlapPolicy::PreserveCrossSpeaker);

    assert_eq!(get_utterance_bullet(&chat, 0), Some((1000, 5000)));
    assert_eq!(wor_word_timing(&chat, 0, 1), Some((3000, 5000)));
    assert!(
        result.records().is_empty(),
        "no decision should be recorded for a preserved cross-speaker pair"
    );
    assert!(result.effects().is_empty());
}

/// The Preserve policy's inherited coverage, wider than the word hull on
/// both sides, survives untouched when the bullet does not actually overlap
/// its neighbor: the pass never enters the classification branch at all.
#[test]
fn non_overlapping_bullet_keeps_its_full_preserve_coverage() {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|x|CHI|||||Child|||\n*CHI:\thello . \u{15}500_2000\u{15}\n%wor:\thello \u{15}1000_1500\u{15} .\n*CHI:\tnext . \u{15}2000_3000\u{15}\n@End\n";
    let mut chat = parse_chat(input);

    let result = enforce_monotonicity(&mut chat);

    assert_eq!(
        get_utterance_bullet(&chat, 0),
        Some((500, 2000)),
        "coverage on both sides of the hull survives when nothing overlaps"
    );
    assert_eq!(wor_word_timing(&chat, 0, 0), Some((1000, 1500)));
    assert!(result.records().is_empty());
    assert!(result.effects().is_empty());
}

/// Finding 1 (2026-09-01 review). Direct unit test of `classify_end_overlap`'s
/// guard against `BoundaryFromWords` pushing `next`'s start past its OWN
/// successor's start. Without the guard this returns `BoundaryFromWords`
/// unconditionally whenever the hulls agree, regardless of what follows.
#[test]
fn boundary_from_words_never_pushes_next_start_past_its_successor() {
    // The successor starts before where the hull would place `next`: the
    // move is unsafe, so this must fall back to `InterleavedWords`.
    assert!(matches!(
        classify_end_overlap(Some(2900), 2000, Some(2950), Some(2400)),
        EndOverlapResolution::InterleavedWords
    ));

    // Same hulls, but the successor starts safely after: the move is fine.
    assert!(matches!(
        classify_end_overlap(Some(2900), 2000, Some(2950), Some(3000)),
        EndOverlapResolution::BoundaryFromWords {
            prev_hull_end_ms: 2900,
            next_hull_start_ms: 2950,
        }
    ));

    // No successor at all (last timed utterance in the file): nothing to
    // violate, so the move is fine.
    assert!(matches!(
        classify_end_overlap(Some(2900), 2000, Some(2950), None),
        EndOverlapResolution::BoundaryFromWords {
            prev_hull_end_ms: 2900,
            next_hull_start_ms: 2950,
        }
    ));
}

/// Finding 1 (2026-09-01 review), end to end: the exact failing construction.
/// Three same-speaker utterances. U0's hull (ends 2900) and U1's hull (starts
/// 2950) agree on a `BoundaryFromWords` boundary in isolation, but U1's own
/// successor U2 starts at 2400, before 2950. Applying the move anyway (the
/// pre-fix behavior) leaves U1 starting after U2, which the very next pair's
/// zero-duration guard then "resolved" by STRIPPING U1 entirely, destroying
/// an utterance that had just been correctly resolved with `needs_review =
/// false`. The fix must produce no strip anywhere and lose no utterance.
#[test]
fn three_utterance_boundary_conflict_is_not_stripped() {
    let input = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n@ID:\teng|x|CHI|||||Child|||\n*CHI:\thello world . \u{15}1000_3000\u{15}\n%wor:\thello \u{15}1000_1500\u{15} world \u{15}1500_2900\u{15} .\n*CHI:\tnext . \u{15}2000_3500\u{15}\n%wor:\tnext \u{15}2950_3200\u{15} .\n*CHI:\tthird . \u{15}2400_5000\u{15}\n@End\n";
    let mut chat = parse_chat(input);

    let result = enforce_monotonicity(&mut chat);

    // No strip anywhere: every utterance keeps a bullet.
    assert!(get_utterance_bullet(&chat, 0).is_some(), "U0 must survive");
    assert!(get_utterance_bullet(&chat, 1).is_some(), "U1 must survive");
    assert!(get_utterance_bullet(&chat, 2).is_some(), "U2 must survive");

    // U1's start was never moved to 2950 (the guard refused it), so pair
    // (U1, U2) never sees a degenerate start >= next_start and never strips.
    assert_eq!(
        get_utterance_bullet(&chat, 1),
        Some((2000, 2400)),
        "U1 keeps its own original start; its end is clamped by pair (U1,U2)"
    );
    // U0 was clamped to U1's ORIGINAL start (2000), not the hull boundary
    // (2900/2950), since the guard forced the InterleavedWords fallback.
    assert_eq!(get_utterance_bullet(&chat, 0), Some((1000, 2000)));
    assert_eq!(
        wor_word_timing(&chat, 0, 1),
        Some((1500, 2000)),
        "the word straddling the fallback bound is clamped, not dropped"
    );
    // U1's own word (2950-3200) is entirely past its bullet's new end (2400)
    // once pair (U1,U2) clamps U1: no valid extent remains, so it is
    // dropped, same as any other word entirely past a clamp bound.
    assert_eq!(wor_word_timing(&chat, 1, 0), None);

    assert!(
        result
            .records()
            .iter()
            .all(|d| d.strategy.strategy_name() != "timing_stripped"),
        "no timing_stripped record anywhere: {:?}",
        result.records()
    );
    assert_eq!(result.effects().len(), 2);
    assert!(
        result
            .effects()
            .iter()
            .all(|e| matches!(e, MonotonicityEffect::EndClampedInterleavedWords { .. }))
    );
}
