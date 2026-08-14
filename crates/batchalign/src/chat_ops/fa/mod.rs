//! Forced alignment orchestration for CHAT files.
//!
//! Extracts pure-Rust FA logic from the PyO3 bridge (`batchalign-core`) so that
//! both the PyO3 layer and the root Rust workspace can share it.
//!
//! Pipeline: parse -> group utterances -> dispatch inference -> parse responses
//! -> inject timings -> postprocess -> generate %wor -> enforce monotonicity/E704.

pub mod alignment;
mod expand_for_fillers;
mod extraction;
mod grouping;
mod injection;
mod orchestrate;
pub mod outcome;
mod postprocess;
pub mod repair;
mod rescue_narrow_bullets;
pub mod review_tiers;
pub mod utr;

#[cfg(test)]
mod tests;

use crate::types::engines::FaTimingResolution;
use serde::{Deserialize, Serialize};
use talkbank_model::alignment::helpers::{TierDomain, WordItem, counts_for_tier, walk_words};
use talkbank_model::model::{
    Bullet, ChatFile, DependentTier, Line, Utterance, UtteranceContent, Word, WordCategory,
};
use talkbank_model::{UtteranceIdx, WordIdx};

// Re-export public API so that `crate::chat_ops::fa::Foo` paths remain unchanged.
pub use self::alignment::parse_fa_response;
pub use self::expand_for_fillers::expand_bullets_for_edge_fillers;
pub use self::extraction::collect_fa_words;
pub use self::grouping::{
    WHISPER_FA_MAX_LABEL_TOKENS, count_utterance_timing, estimate_untimed_boundaries,
    group_utterances,
};
pub use self::injection::inject_timings_for_utterance;
pub use self::orchestrate::{
    apply_fa_results, enforce_monotonicity, has_reusable_wor_timing_for_utterance,
    refresh_existing_alignment, refresh_existing_alignment_for_utterance,
    refresh_reusable_utterances, strip_e704_same_speaker_overlaps, strip_timing_from_content,
    strip_wor_from_monotonicity_stripped_utterances,
};
pub use self::postprocess::postprocess_utterance_timings;
pub use self::repair::{RepairDecision, RepairResult, RepairStats, repair_bullets};
pub use self::rescue_narrow_bullets::rescue_narrow_bullets;
pub use self::review_tiers::{ReviewLevel, inject_review_tiers};
pub use self::utr::{
    CaMarkerPolicy, GlobalUtr, GroupingContext, TwoPassConfig, TwoPassOverlapUtr, UtrMatchMode,
    UtrStrategy, find_untimed_windows, select_strategy, utr_asr_cache_key,
    utr_asr_segment_cache_key,
};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A time interval in milliseconds.
///
/// NOT guaranteed `start <= end`: [`Self::new`] accepts any pair. Callers that
/// require a real extent must check.
///
/// The constructor cannot enforce it because post-processing holds spans whose
/// extent is not resolved yet, and those are legitimate. [`WordTiming`] is the
/// other half of that split: it is what a settled extent looks like, and it is
/// the only thing a word's bullet can be written from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeSpan {
    /// Start time in milliseconds.
    pub start_ms: u64,
    /// End time in milliseconds.
    pub end_ms: u64,
}

impl TimeSpan {
    /// Create a new time span. Caller is responsible for ensuring start <= end.
    pub fn new(start_ms: u64, end_ms: u64) -> Self {
        Self { start_ms, end_ms }
    }

    /// Duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.end_ms.saturating_sub(self.start_ms)
    }
}

/// A word's timing, guaranteed to cover a positive extent.
///
/// [`TimeSpan`] deliberately carries no invariant, because post-processing
/// works with spans whose extent is not settled yet. This is the type that
/// leaves that computation: a word occupies time, so `end > start` holds for
/// every value that reaches a `%wor` tier or the cache.
///
/// The invariant is enforced at CONSTRUCTION, which is the only way in;
/// `TimeSpan::new` accepts any pair and always did. Reads are unchanged: this
/// derefs to the span it wraps.
///
/// `postprocess_continuous::a_word_is_not_healed_backwards_into_an_earlier_neighbour`
/// is the executable account of why this type exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "TimeSpan", into = "TimeSpan")]
pub struct WordTiming(TimeSpan);

impl WordTiming {
    /// A timing for a word that occupies `start_ms..end_ms`.
    ///
    /// `None` when the pair does not describe a positive extent. A caller that
    /// cannot build one has learned something real: the engine gave no usable
    /// timing for that word, and the honest result is no timing rather than a
    /// zero-length or backwards one.
    pub fn new(start_ms: u64, end_ms: u64) -> Option<Self> {
        (end_ms > start_ms).then_some(Self(TimeSpan::new(start_ms, end_ms)))
    }
}

impl std::ops::Deref for WordTiming {
    type Target = TimeSpan;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<WordTiming> for TimeSpan {
    fn from(timing: WordTiming) -> Self {
        timing.0
    }
}

/// Rejects a stored span that does not describe a positive extent.
///
/// This is what makes the FA cache self-cleaning: a pre-2026-08-14 entry
/// holding zero-duration timings fails to deserialize, the read path treats
/// that as a miss, and the group is recomputed with the fixed parser.
#[derive(Debug, thiserror::Error)]
#[error("a word timing must cover a positive extent, got {start_ms}..{end_ms}")]
pub struct DegenerateWordTiming {
    /// Start of the rejected span.
    pub start_ms: u64,
    /// End of the rejected span.
    pub end_ms: u64,
}

impl TryFrom<TimeSpan> for WordTiming {
    type Error = DegenerateWordTiming;

    fn try_from(span: TimeSpan) -> Result<Self, Self::Error> {
        Self::new(span.start_ms, span.end_ms).ok_or(DegenerateWordTiming {
            start_ms: span.start_ms,
            end_ms: span.end_ms,
        })
    }
}

/// Split a compound filler's cleaned text at underscores, or return the
/// text as a single element. Only applies to `WordCategory::Filler` words;
/// regular compounds are unchanged.
///
/// Both extraction and injection must agree on the split count, extraction
/// sends N parts to FA, injection consumes N timings from the cursor. This
/// shared helper is the single source of truth for the splitting rule.
pub fn split_compound_filler(word: &talkbank_model::model::Word) -> Vec<String> {
    use talkbank_model::model::WordCategory;
    let text = word.cleaned_text();
    if word.category == Some(WordCategory::Filler) && text.contains('_') {
        text.split('_')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    } else {
        vec![text.to_string()]
    }
}

/// A word extracted for forced alignment, with its position in the AST.
#[derive(Debug, Clone)]
pub struct FaWord {
    /// Index of the utterance in the file (among utterances only).
    pub utterance_index: UtteranceIdx,
    /// Index among alignable words within the utterance.
    pub utterance_word_index: WordIdx,
    /// Cleaned text for the FA model.
    pub text: String,
}

impl FaWord {
    /// Stable word identifier for callback protocols.
    pub fn stable_id(&self) -> String {
        format!("u{}:w{}", self.utterance_index, self.utterance_word_index)
    }
}

/// A group of utterances clustered for a single FA call.
#[derive(Debug)]
pub struct FaGroup {
    /// Audio window for this group.
    #[allow(dead_code)]
    pub audio_span: TimeSpan,
    /// Words in this group with positional indices.
    pub words: Vec<FaWord>,
    /// Utterance indices included in this group.
    pub utterance_indices: Vec<UtteranceIdx>,
}

impl FaGroup {
    /// Start of the audio window (ms).
    pub fn audio_start_ms(&self) -> u64 {
        self.audio_span.start_ms
    }

    /// End of the audio window (ms).
    pub fn audio_end_ms(&self) -> u64 {
        self.audio_span.end_ms
    }
}

/// Duration given to a word whose end nothing could measure or derive.
///
/// The last word of a group has no successor onset, and an utterance may have
/// no bullet to borrow an end from. It is a stand-in for an unmeasured
/// quantity, which is why it is named once here rather than written as a bare
/// number at each of the places that need it.
pub const LAST_WORD_FALLBACK_MS: u64 = 500;

/// Whether FA post-processing heals small gaps between consecutive words.
///
/// This used to be `FaTimingMode`, a name that covered two unrelated
/// decisions: how the token-level parser derived a word's END, and whether
/// the gap-healing pass ran. One flag drove both, so asking for one asked for
/// the other. That is how `--fa-engine whisper` without `--pauses` came to
/// request word ends set to their own starts, which is the shape of a `%wor`
/// tier full of 0-duration words.
///
/// The parser no longer has a choice to make: an onset-only engine reports no
/// end, so the next word's onset is the only end available, and it always uses
/// it. What remains is genuinely a policy, and this type names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WordGapHealing {
    /// Extend a word to the next word's start when the gap between them is
    /// small and plausible, so words read as contiguous speech.
    ///
    /// See `postprocess::postprocess_utterance_timings` for the plausibility
    /// caps; the healing is bounded, not unconditional.
    Heal,
    /// Leave each word ending where the engine said it ended, so silence
    /// between words survives into `%wor`.
    PreserveMeasured,
}

/// What post-processing may do to a word's end time.
///
/// Two independent facts that must describe the SAME run: whether the user
/// asked for gaps to be healed, and whether this engine measured word ends or
/// had them derived from the next onset. They were separate arguments until
/// 2026-08-14, which type-checked happily with one engine's healing policy and
/// another engine's resolution.
///
/// Constructed where both facts are already in hand, so a consumer cannot pair
/// them wrongly: [`crate::types::params::FaParams::word_end_policy`] derives
/// the resolution from the engine it already holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WordEndPolicy {
    gap_healing: WordGapHealing,
    resolution: crate::types::engines::FaTimingResolution,
}

impl WordEndPolicy {
    /// Derive the policy for an engine, given what the user asked for.
    pub fn for_engine(
        engine: crate::types::engines::FaEngineName,
        gap_healing: WordGapHealing,
    ) -> Self {
        Self {
            gap_healing,
            resolution: engine.timing_resolution(),
        }
    }

    /// A word-interval engine: ends came from the model.
    #[cfg(test)]
    pub fn measured(gap_healing: WordGapHealing) -> Self {
        Self {
            gap_healing,
            resolution: crate::types::engines::FaTimingResolution::WordIntervals,
        }
    }

    /// An onset-only engine: ends were derived from the next onset.
    #[cfg(test)]
    pub fn onset_only(gap_healing: WordGapHealing) -> Self {
        Self {
            gap_healing,
            resolution: crate::types::engines::FaTimingResolution::TokenOnsets,
        }
    }

    /// Whether small gaps between words may be closed.
    pub fn heals(self) -> bool {
        self.gap_healing == WordGapHealing::Heal
    }

    /// Whether this engine's word ends were derived rather than measured.
    ///
    /// A derived end may be replaced by a better one, such as the utterance
    /// bullet; a measured end is the model's own answer and is left alone.
    pub fn ends_are_derived(self) -> bool {
        self.resolution == crate::types::engines::FaTimingResolution::TokenOnsets
    }
}

/// Words that lost their timing during post-processing, by cause.
///
/// Two counts rather than one total, because a reviewer reads the reason. The
/// caller used to receive a bare `usize` and stamp
/// `reason=clamped_to_utterance_boundary` on every one, which became false as
/// soon as a second cause existed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DroppedWordTimings {
    /// Clamping to an authoritative utterance bullet left no extent.
    pub clamped_to_utterance_boundary: usize,
    /// The span reaching the write-back did not describe an extent.
    pub without_extent: usize,
}

impl DroppedWordTimings {
    /// Whether any word lost its timing.
    pub fn any(self) -> bool {
        self.total() > 0
    }

    /// How many words lost their timing, for any reason.
    pub fn total(self) -> usize {
        self.clamped_to_utterance_boundary + self.without_extent
    }

    /// A decision-record reason naming only the causes that actually fired.
    pub fn reason(self) -> String {
        let mut causes = Vec::new();
        if self.clamped_to_utterance_boundary > 0 {
            causes.push(format!(
                "clamped_to_utterance_boundary={}",
                self.clamped_to_utterance_boundary
            ));
        }
        if self.without_extent > 0 {
            causes.push(format!("no_extent={}", self.without_extent));
        }
        format!("count={} {}", self.total(), causes.join(" "))
    }
}

/// Wire type for the FA infer protocol -- one group sent to a Python worker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaInferItem {
    /// Words to align (cleaned text).
    pub words: Vec<String>,
    /// Stable word IDs aligned 1:1 with `words`.
    pub word_ids: Vec<String>,
    /// Utterance indices aligned 1:1 with `words`.
    pub word_utterance_indices: Vec<usize>,
    /// Word indices inside each utterance aligned 1:1 with `words`.
    pub word_utterance_word_indices: Vec<usize>,
    /// Path to the audio file.
    pub audio_path: String,
    /// Start of the audio window (ms).
    pub audio_start_ms: u64,
    /// End of the audio window (ms).
    pub audio_end_ms: u64,
    /// How to handle word end times during post-processing.
    pub gap_healing: WordGapHealing,
}

impl FaInferItem {
    /// Audio window as a [`TimeSpan`].
    pub fn audio_span(&self) -> TimeSpan {
        TimeSpan::new(self.audio_start_ms, self.audio_end_ms)
    }
}

// ---------------------------------------------------------------------------
// %wor tier management
// ---------------------------------------------------------------------------

/// Remove existing %wor tier from an utterance (if any).
pub fn remove_wor_tier(utterance: &mut Utterance) {
    utterance
        .dependent_tiers
        .retain(|t| !matches!(t.tier, DependentTier::Wor(_)));
}

/// Add a `%wor` tier generated from the inline bullets on words.
///
/// If the utterance already has a `%wor` tier, it is replaced **in place**,
/// preserving its position among other dependent tiers. If it has none,
/// the new tier is appended. Previously this called
/// [`remove_wor_tier`] followed by `push`, which destroyed the original
/// position and produced noisy git diffs on every file whose `%wor`
/// originally sat before `%mor` / `%gra` or in some other non-default
/// slot. The position-preserving replace fixes that entire class of
/// diff noise.
pub fn add_wor_tier(utterance: &mut Utterance) {
    let wor_tier = utterance.main.generate_wor_tier();
    batchalign_transform::inject::replace_or_add_tier(
        &mut utterance.dependent_tiers,
        DependentTier::Wor(wor_tier),
    );
}

/// Return `true` when every alignable FA word in the file already has reusable
/// `%wor` timing.
///
/// This intentionally does **not** look at `main` tier `inline_bullet` alone.
/// After a parse roundtrip, main-tier word timing may be represented as
/// `InternalBullet` tokens, while `%wor` carries the durable first-class timing
/// bullets. For the cheap rerun path we therefore verify that `%wor` fully and
/// cleanly aligns back to the main tier.
pub fn has_reusable_wor_timing(chat_file: &ChatFile) -> bool {
    let mut saw_alignable_word = false;
    let reusable = find_reusable_utterance_indices(chat_file);
    let mut expected_reusable = 0usize;
    let mut utt_idx = 0usize;

    for line in &chat_file.lines {
        let Line::Utterance(utterance) = line else {
            continue;
        };

        let main_word_count = count_alignable_main_words(utterance);
        if main_word_count == 0 {
            utt_idx += 1;
            continue;
        }
        saw_alignable_word = true;
        expected_reusable += 1;
        if !reusable.contains(&utt_idx) {
            return false;
        }
        utt_idx += 1;
    }

    saw_alignable_word && reusable.len() == expected_reusable
}

/// Find utterance indices that have reusable `%wor` timing.
///
/// Returns a set of utterance ordinal indices where
/// [`has_reusable_wor_timing_for_utterance()`] succeeds. Used by the plain
/// rerun path to selectively skip FA for utterances whose `%wor` is still
/// clean after manual edits to other utterances.
pub fn find_reusable_utterance_indices(chat_file: &ChatFile) -> std::collections::HashSet<usize> {
    struct ReuseCandidate {
        utt_idx: usize,
        has_alignable_words: bool,
        wor_span: Option<WordTiming>,
        main_start_ms: Option<u64>,
    }

    let mut candidates = Vec::new();
    let mut reusable = std::collections::HashSet::new();
    let mut utt_idx = 0usize;
    for line in &chat_file.lines {
        let Line::Utterance(utterance) = line else {
            continue;
        };
        let has_alignable_words = count_alignable_main_words(utterance) > 0;
        candidates.push(ReuseCandidate {
            utt_idx,
            has_alignable_words,
            wor_span: if has_alignable_words {
                self::orchestrate::collect_wor_backed_span(utterance)
            } else {
                None
            },
            main_start_ms: utterance
                .main
                .content
                .bullet
                .as_ref()
                .map(|bullet| bullet.timing.start_ms),
        });
        utt_idx += 1;
    }

    let mut next_timed_start = None;
    let mut next_timed_start_after = vec![None; candidates.len()];
    for i in (0..candidates.len()).rev() {
        next_timed_start_after[i] = next_timed_start;
        if let Some(start_ms) = candidates[i].main_start_ms {
            next_timed_start = Some(start_ms);
        }
    }

    for (i, candidate) in candidates.iter().enumerate() {
        if !candidate.has_alignable_words {
            continue;
        }
        let Some(span) = candidate.wor_span else {
            continue;
        };
        if let Some(next_start_ms) = next_timed_start_after[i]
            && span.end_ms > next_start_ms
        {
            continue;
        }
        reusable.insert(candidate.utt_idx);
    }

    reusable
}

/// Count Wor-alignable words in the main tier.
pub(crate) fn count_alignable_main_words(utterance: &Utterance) -> usize {
    let mut count = 0usize;
    walk_words(
        &utterance.main.content.content,
        None,
        &mut |leaf| match leaf {
            WordItem::Word(word) => {
                if counts_for_tier(word, TierDomain::Wor) {
                    count += 1;
                }
            }
            WordItem::ReplacedWord(replaced) => {
                // Must mirror extraction policy: the original word is the FA
                // unit, not the replacement words.  Counting replacement words
                // here would overcount vs. extraction and desync the cursor.
                if counts_for_tier(&replaced.word, TierDomain::Wor) {
                    count += 1;
                }
            }
            WordItem::Separator(_) => {}
        },
    );
    count
}

/// Update the utterance-level bullet from word timings.
///
/// The behavior depends on the bullet's provenance ([`BulletSource`]):
///
/// - **No pre-existing bullet**, sets bullet directly from the FA word span.
///
/// - **`BulletSource::Utr`**: overwrites with the FA word span. The UTR
///   bullet was a provisional grouping hint; once FA has produced word
///   timings the hint is discarded and the FA span is authoritative.
///
/// - **`BulletSource::Authoritative`**: usually unions the existing bullet
///   with the FA word span when that preserves plausible leading/trailing
///   coverage. On reruns with an existing `%wor` tier, a large lead before the
///   first aligned word is only preserved when untimed leading filler coverage
///   still remains. Otherwise it is treated as stale inherited timing and reset
///   to the FA word start. Trailing coverage still never shrinks so gestures
///   and similar non-alignable material are preserved.
///
/// When FA produces no word timings at all (all `None`), the existing bullet
/// is left unchanged: the UTR hint is the only timing we have and must be
/// preserved.
///
/// # Invariant
///
/// Every bullet written by this function has `BulletSource::Authoritative`,
/// marking it as FA-derived (no longer a provisional UTR hint).
pub fn update_utterance_bullet(utterance: &mut Utterance) {
    use talkbank_model::model::BulletSource;

    const MAX_AUTHORITATIVE_START_LEAD_MS: u64 = 2_000;

    let mut first_start: Option<u64> = None;
    let mut last_end: Option<u64> = None;

    let mut timings: Vec<Option<TimeSpan>> = Vec::new();
    postprocess::collect_word_timings(&utterance.main.content.content, &mut timings);
    let has_fa_wor = utterance.wor_tier().is_some();
    let has_untimed_leading_filler_coverage =
        has_untimed_leading_filler_coverage(&utterance.main.content.content);

    for span in timings.iter().flatten() {
        if first_start.is_none_or(|s| span.start_ms < s) {
            first_start = Some(span.start_ms);
        }
        if last_end.is_none_or(|e| span.end_ms > e) {
            last_end = Some(span.end_ms);
        }
    }

    if let (Some(word_start), Some(word_end)) = (first_start, last_end) {
        let (final_start, final_end) = match &utterance.main.content.bullet {
            // Provisional UTR hint: FA word span is authoritative, overwrite.
            Some(existing) if existing.source == BulletSource::Utr => (word_start, word_end),
            // Authoritative hand-linked/FA bullet: reruns with an existing %wor
            // can preserve stale starts from a previous pass. If that lead is
            // implausibly large and there is no untimed leading filler coverage
            // left to preserve, snap the start back to the FA word span.
            // Otherwise keep the old leading coverage.
            Some(existing) => {
                let start_lead_ms = word_start.saturating_sub(existing.timing.start_ms);
                let final_start = if !has_fa_wor
                    || has_untimed_leading_filler_coverage
                    || start_lead_ms <= MAX_AUTHORITATIVE_START_LEAD_MS
                {
                    word_start.min(existing.timing.start_ms)
                } else {
                    word_start
                };
                (final_start, word_end.max(existing.timing.end_ms))
            }
            // No pre-existing bullet: set from word span.
            None => (word_start, word_end),
        };
        // The resulting bullet is authoritative (FA-derived).
        utterance.main.content.bullet = Some(Bullet::new(final_start, final_end));
    }
    // If no word timings: leave existing bullet unchanged, UNLESS it is
    // zero-duration (start >= end), which is an invalid timing that would
    // produce E362. A zero-duration bullet from a previous buggy run must be
    // cleared; no bullet is valid CHAT, an invalid bullet is not.
    if let Some(ref existing) = utterance.main.content.bullet
        && existing.timing.start_ms >= existing.timing.end_ms
    {
        utterance.main.content.bullet = None;
    }
}

fn has_untimed_leading_filler_coverage(content: &[UtteranceContent]) -> bool {
    let mut before_first_timed_word = true;
    let mut has_untimed_leading_filler = false;

    walk_words(content, Some(TierDomain::Wor), &mut |leaf| {
        if !before_first_timed_word {
            return;
        }
        let word = match leaf {
            WordItem::Word(word) => word,
            WordItem::ReplacedWord(replaced) => &replaced.word,
            WordItem::Separator(_) => return,
        };
        if get_word_timing(word).is_some() {
            before_first_timed_word = false;
            return;
        }
        if word.category == Some(WordCategory::Filler) {
            has_untimed_leading_filler = true;
        }
    });

    has_untimed_leading_filler
}

/// Collect current main-tier word timings in the exact order FA uses for
/// extraction and injection.
///
/// This is the stable timing surface for selective reuse: when an utterance has
/// already been refreshed from `%wor`, the returned vector can be stitched
/// directly into a preserved FA group without a worker roundtrip.
pub fn collect_existing_fa_word_timings(utterance: &Utterance) -> Vec<Option<WordTiming>> {
    let mut timings = Vec::new();
    walk_words(
        &utterance.main.content.content,
        None,
        &mut |leaf| match leaf {
            WordItem::Word(word) => {
                if counts_for_tier(word, TierDomain::Wor) {
                    timings.push(
                        get_word_timing(word).and_then(|span| WordTiming::try_from(span).ok()),
                    );
                }
            }
            WordItem::ReplacedWord(replaced) => {
                // Mirror extraction policy: the original word is the FA unit.
                // Replacement words never receive inline_bullet after injection,
                // so collecting them would always produce None for each and
                // miscount vs. collect_fa_words → collect_preserved_group_timings
                // would return None and needlessly bypass the %wor preservation path.
                if counts_for_tier(&replaced.word, TierDomain::Wor) {
                    timings.push(
                        get_word_timing(&replaced.word)
                            .and_then(|span| WordTiming::try_from(span).ok()),
                    );
                }
            }
            WordItem::Separator(_) => {}
        },
    );
    timings
}

// ---------------------------------------------------------------------------
// Helpers shared across submodules
// ---------------------------------------------------------------------------

/// The LINE index of the nth utterance, or `None` if there is no nth utterance.
///
/// `UtteranceIdx` counts only utterances; `LineIdx` counts every line, headers
/// included. The two never coincide, because a CHAT file always opens with
/// headers, so converting between them requires walking the file. That is why
/// this is a function taking the file rather than a cast: a producer holding
/// an ordinal cannot fabricate a line index without looking.
pub(super) fn utterance_line_idx(
    chat_file: &talkbank_model::model::ChatFile,
    utt_idx: UtteranceIdx,
) -> Option<batchalign_transform::decisions::LineIdx> {
    use talkbank_model::model::Line;
    let mut seen = 0;
    for (line_idx, line) in chat_file.lines.as_slice().iter().enumerate() {
        if matches!(line, Line::Utterance(_)) {
            if seen == utt_idx.raw() {
                return Some(batchalign_transform::decisions::LineIdx::new(line_idx));
            }
            seen += 1;
        }
    }
    None
}

/// Get a mutable reference to the nth utterance in the file.
pub(super) fn get_utterance_mut(
    chat_file: &mut talkbank_model::model::ChatFile,
    utt_idx: UtteranceIdx,
) -> Option<&mut Utterance> {
    use talkbank_model::model::Line;
    let mut current = 0;
    for line in &mut chat_file.lines {
        if let Line::Utterance(utt) = line {
            if current == utt_idx.raw() {
                return Some(utt);
            }
            current += 1;
        }
    }
    None
}

/// Get the inline timing from a word, if present.
pub(super) fn get_word_timing(word: &Word) -> Option<TimeSpan> {
    word.inline_bullet
        .as_ref()
        .map(|b| TimeSpan::new(b.timing.start_ms, b.timing.end_ms))
}

// ---------------------------------------------------------------------------
// AudioIdentity
// ---------------------------------------------------------------------------

/// Content identity for an audio file used in FA cache keys.
///
/// # Invariant
///
/// Format: `"{resolved_path}|{mtime_secs}|{file_size}"`. Fast identity
/// based on filesystem metadata (no file content hashing). Created by
/// [`AudioIdentity::from_metadata`] in the server runner.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AudioIdentity(String);

impl AudioIdentity {
    /// Build an identity from resolved path + filesystem metadata.
    pub fn from_metadata(path: &str, mtime_secs: u64, size: u64) -> Self {
        Self(format!("{path}|{mtime_secs}|{size}"))
    }

    /// Access the raw identity string (for display/logging).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AudioIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Cache key computation
// ---------------------------------------------------------------------------

/// Compute cache key for an FA result.
///
/// Key = `BLAKE3("{audio_identity}|{start}|{end}|{text}|{healing_flag}|{engine}")`.
pub fn cache_key(
    words: &[String],
    audio_identity: &AudioIdentity,
    start_ms: u64,
    end_ms: u64,
    gap_healing: WordGapHealing,
    engine: crate::types::engines::FaEngineName,
) -> crate::chat_ops::CacheKey {
    let text = words.join(" ");
    // What is CACHED is the parsed `Vec<Option<WordTiming>>`, not the raw
    // worker response, so this key must cover everything those timings depend
    // on and nothing they do not. Gap healing is applied later, in
    // post-processing, and never reaches the stored value.
    //
    // The two resolutions therefore differ, and collapsing them is what makes
    // the difference expensive:
    //
    // - WORD-INTERVAL engines take their start and end from the model. The
    //   stored timings do not depend on the healing policy at all, and
    //   the healing policy cannot change what the model returns, so one
    //   constant is correct. (The Cantonese request does carry `pauses` on the
    //   wire; its runner binds it unused, so it changes no output today.) It keeps the literal the old code always hashed for these
    //   engines, which is what lets the fleet's existing entries stay
    //   reachable: re-running them would spend GPU hours to recompute
    //   byte-identical values.
    //
    // - TOKEN-ONSET engines get their ends derived by the parser, and that
    //   derivation changed on 2026-08-14: entries written before it hold ends
    //   equal to their own starts, which is no duration at all. New strings
    //   retire exactly those, and the flag additionally stands in for the
    //   tokenization, since the same `--pauses` value selects it.
    let healing_flag = match engine.timing_resolution() {
        FaTimingResolution::WordIntervals => "no_pauses",
        FaTimingResolution::TokenOnsets => match gap_healing {
            WordGapHealing::Heal => "heal_gaps",
            WordGapHealing::PreserveMeasured => "preserve_measured",
        },
    };
    // The ENGINE, so two models that share a parse strategy cannot share
    // cache entries for the same audio and words.
    let engine_str = {
        use crate::types::engines::EngineBackend;
        engine.wire_name()
    };
    let input = format!(
        "{}|{start_ms}|{end_ms}|{text}|{healing_flag}|{engine_str}",
        audio_identity.0
    );
    crate::chat_ops::CacheKey::from_content(&input)
}

#[cfg(test)]
mod utterance_line_idx_tests {
    use super::*;

    /// The conversion must SKIP headers, which is the whole reason it exists.
    ///
    /// `orchestrate` previously stored an utterance ordinal in a field that
    /// indexes lines. It compiled because both were `usize`, and the effect was
    /// invisible: the decision either vanished or attached to the wrong
    /// utterance. `LineIdx` now rejects the assignment, and this pins the
    /// conversion that replaced it.
    #[test]
    fn converts_an_utterance_ordinal_to_a_line_index() {
        let chat_text = "\
@UTF8
@Begin
@Languages:\teng
@Participants:\tCHI Target_Child
@ID:\teng|test|CHI|2;0.0||||Target_Child|||
*CHI:\thello .
*CHI:\tworld .
@End
";
        let parser = talkbank_parser::TreeSitterParser::new().expect("parser init");
        let chat = parser.parse_chat_file(chat_text).expect_built();

        // Five headers precede the first utterance, so the spaces differ.
        assert_eq!(
            utterance_line_idx(&chat, UtteranceIdx::new(0)).map(|l| l.raw()),
            Some(5)
        );
        assert_eq!(
            utterance_line_idx(&chat, UtteranceIdx::new(1)).map(|l| l.raw()),
            Some(6)
        );
        // Past the end is None, not a fabricated index.
        assert_eq!(utterance_line_idx(&chat, UtteranceIdx::new(2)), None);
    }
}
