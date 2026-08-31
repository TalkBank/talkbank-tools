//! Utterance grouping for forced alignment time windows.

use talkbank_model::model::{ChatFile, Line};
use talkbank_model::{UtteranceIdx, WordIdx};

use batchalign_transform::decisions::{DecisionRecord, DecisionStrategy, FaStrategy};

use super::coordinates::{Clamped, FileMs, Ms, Recording};
use super::extraction::collect_fa_words;
use super::speech_rate::SpeechRate;
use super::{FaGroup, FaWord, TimeSpan};

/// Where an utterance's audio is, or why it has none.
///
/// Returned per utterance by [`estimate_untimed_boundaries`]. The second
/// variant is the one that matters: an untimed run whose remaining audio could
/// not physically contain its words has no window, and saying so is better than
/// computing one. A `TimeSpan` alone could not express it, so the question went
/// unasked and every run got a window regardless.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Placement {
    /// Audio this utterance can be aligned against.
    Placed(TimeSpan),
    /// No audio can hold these words; the rate says how badly.
    Unplaceable(SpeechRate),
}

/// What [`estimate_untimed_boundaries`] worked out.
///
/// # Why this is not a bare `Vec<Placement>`
///
/// The pass LEARNS one thing besides where each utterance goes: how many of the
/// windows it computed ran past the end of the recording and had to be cut down
/// to fit. That is a fact about our own gap arithmetic, and the `.min()` it
/// used to be made it unobservable, so nobody could tell a session where every
/// window fitted from one where a dozen were trimmed.
#[derive(Debug, PartialEq)]
pub struct Estimates {
    /// Where each utterance's audio is, in file order.
    pub placements: Vec<Placement>,
    /// How many computed windows overshot the recording and were cut to it.
    pub windows_clamped: usize,
}

/// Whisper CTC forced-alignment hard limit on the total number of label tokens
/// (characters) that can appear in a single FA group.
///
/// Exceeding this limit causes a Python-side `ValueError: Labels' sequence
/// length N cannot exceed the maximum allowed length of 448 tokens`. Groups
/// must be split by char count as well as by time window to stay under this.
pub const WHISPER_FA_MAX_LABEL_TOKENS: usize = 448;

/// Maximum extension (ms) into the gap after the last utterance in a group.
///
/// When an utterance bullet ends before the next utterance starts, we extend
/// the FA group's audio window into that gap so the FA engine can hear
/// trailing fillers (`&-you_know`, `&-sort_of`) that live between utterances.
/// The extension is capped at this value to avoid bleeding into the next
/// utterance's content.
const TRAILING_GAP_EXTENSION_MS: u64 = 1500;

/// What grouping produced.
///
/// # Why the refusals come back rather than going to a log
///
/// An utterance this pass declines to place will have NO timing in the output,
/// permanently, and a reader of the CHAT file cannot distinguish that from an
/// aligner that simply failed. The program knows which; a `tracing::warn!` is
/// where that knowledge used to stop. `rescue_narrow_bullets` already returns
/// its decisions for the same reason, and this is the same kind of fact told
/// the other way round: the rescue says "we fixed it", this says "these words
/// cannot be aligned by anyone".
pub struct Grouping {
    /// Windows to send to the aligner.
    pub groups: Vec<FaGroup>,
    /// Utterances left unaligned, one record each, for durable evidence.
    pub refusals: Vec<DecisionRecord>,
    /// How many estimated windows overshot the recording and were cut to it.
    ///
    /// Carried here rather than logged and dropped. It survived one function
    /// boundary in [`Estimates`] and then died in a `tracing::warn!`, which is
    /// the shape this struct's own docstring above spends a paragraph refusing:
    /// a caller could not branch on it, report it, or put it in the artifact.
    /// A session where every window fitted and one where a dozen were trimmed
    /// are different, and that difference is about OUR gap arithmetic.
    pub windows_clamped: usize,
}

/// Group utterances from a ChatFile into FA segments.
///
/// Groups are split when the cumulative duration exceeds `max_group_ms`.
///
/// Utterances with no timing bullet are placed by distributing the surrounding
/// gap across them in proportion to word count, EXCEPT where the audio could not
/// physically contain them: such a run is refused and reported in
/// [`Grouping::refusals`] rather than handed to an aligner.
///
/// * `chat_file` - The parsed CHAT file whose utterances will be grouped.
/// * `max_group_ms` - Maximum audio window duration (in milliseconds) per
///   group. When adding an utterance would push the group past this limit,
///   a new group is started.
/// * `recording` - The audio being aligned against. Required, not optional:
///   when it was an `Option<u64>` a `None` made this function silently SKIP
///   every untimed utterance, so whether a word was ever aligned depended on
///   whether anyone had probed the media.
pub fn group_utterances(
    chat_file: &ChatFile,
    max_group_ms: u64,
    recording: &Recording,
) -> Grouping {
    let total_audio_ms = recording.duration().get();
    let Estimates {
        placements: estimates,
        windows_clamped,
    } = estimate_untimed_boundaries(chat_file, recording);
    if windows_clamped > 0 {
        // Logged AND returned: the log is the convenience, `Grouping` is the
        // contract.
        tracing::warn!(
            windows_clamped,
            "estimated FA windows ran past the end of the recording and were cut to it"
        );
    }

    let mut groups: Vec<FaGroup> = Vec::new();
    let mut refusals: Vec<DecisionRecord> = Vec::new();
    let mut current_words: Vec<FaWord> = Vec::new();
    let mut current_utt_indices: Vec<UtteranceIdx> = Vec::new();
    let mut current_chars: usize = 0;
    let mut seg_start: u64 = 0;
    let mut seg_end: u64 = 0;

    let mut utt_idx: usize = 0;

    let mut extracted = Vec::new();
    for line in &chat_file.lines {
        let utt = match line {
            Line::Utterance(u) => u,
            _ => continue,
        };

        // Taking a `Recording` rather than an `Option<u64>` deleted the third
        // arm here. It used to warn and SKIP an untimed utterance whenever the
        // audio length was unknown, which meant the words of that utterance
        // were silently never aligned; and "unknown audio length" was a state
        // only reachable because the duration was optional. It is not optional
        // now, so an estimate always exists.
        let utt_span = match &utt.main.content.bullet {
            Some(b) => TimeSpan::new(b.timing.start_ms, b.timing.end_ms),
            // An unplaceable run is skipped, and the estimator has already said
            // why in one line for the whole run. Its words stay unaligned,
            // which is what they would have been anyway: no aligner can place
            // words in audio too short to contain them, and the one that cannot
            // detect that invents timings instead.
            None => match estimates[utt_idx] {
                Placement::Placed(span) => span,
                // Recorded, not merely skipped: these words reach the output
                // with no timing, and the reason is a physical fact worth
                // telling a reviewer.
                Placement::Unplaceable(rate) => {
                    if let Some(line_idx) =
                        super::utterance_line_idx(chat_file, UtteranceIdx::new(utt_idx))
                    {
                        refusals.push(DecisionRecord::new_and_trace(
                            line_idx.raw(),
                            utt.main.speaker.as_str().to_string(),
                            DecisionStrategy::Fa(FaStrategy::UnplaceableRun),
                            format!("{rate}"),
                            true,
                        ));
                    }
                    utt_idx += 1;
                    continue;
                }
            },
        };

        // Extract words first so we can count chars before deciding to flush.
        // (drain(..) in the loop below empties `extracted` each iteration)
        collect_fa_words(&utt.main.content.content, &mut extracted);
        let utt_chars: usize = extracted.iter().map(|w| w.len()).sum();

        // Start a new group when this utterance would push the current group past
        // either the time window or Whisper's character-token limit.
        //
        // The char-limit guard is necessary because Whisper CTC FA fails with
        // "Labels' sequence length N cannot exceed the maximum allowed length of
        // 448 tokens" when the total character count in a group exceeds 448.
        // Dense transcripts (fast speech, Spanish/long-word languages) can hit
        // this within a normal time window.
        //
        // Exception: if the current group is empty, we include the utterance
        // regardless: an utterance that alone exceeds the limit still needs to
        // be sent (and will produce a graceful Python-side error rather than
        // silently dropping the utterance).
        let over_time =
            utt_span.end_ms <= seg_start || (utt_span.end_ms - seg_start) > max_group_ms;
        let over_chars = current_chars + utt_chars > WHISPER_FA_MAX_LABEL_TOKENS;
        if !current_words.is_empty() && (over_time || over_chars) {
            // Extend the audio window into the gap before this next utterance
            // so the FA engine can hear trailing fillers at utterance boundaries.
            let extended_end = extend_into_trailing_gap(seg_end, utt_span.start_ms);
            groups.push(FaGroup {
                audio_span: TimeSpan::new(seg_start, extended_end),
                words: std::mem::take(&mut current_words),
                utterance_indices: std::mem::take(&mut current_utt_indices),
            });
            seg_start = utt_span.start_ms;
            current_chars = 0;
        }

        if current_words.is_empty() {
            seg_start = utt_span.start_ms;
        }
        seg_end = utt_span.end_ms;
        current_chars += utt_chars;

        for (word_idx, w) in extracted.drain(..).enumerate() {
            current_words.push(FaWord {
                utterance_index: UtteranceIdx::new(utt_idx),
                utterance_word_index: WordIdx::new(word_idx),
                text: w,
            });
        }

        current_utt_indices.push(UtteranceIdx::new(utt_idx));
        utt_idx += 1;
    }

    // Push the last group, extending into the trailing audio. The "don't extend
    // blindly" arm went with the `Option`: there is no longer an unknown length
    // to be blind about.
    if !current_words.is_empty() {
        let extended_end = extend_into_trailing_gap(seg_end, total_audio_ms);
        groups.push(FaGroup {
            audio_span: TimeSpan::new(seg_start, extended_end),
            words: current_words,
            utterance_indices: current_utt_indices,
        });
    }

    Grouping {
        groups,
        refusals,
        windows_clamped,
    }
}

/// Extend an audio window's end into the gap before the next utterance.
///
/// Returns `seg_end + min(gap / 2, TRAILING_GAP_EXTENSION_MS)`; we take
/// at most half the gap to avoid bleeding into the next utterance's audio,
/// capped at the configured maximum extension.
fn extend_into_trailing_gap(seg_end: u64, next_utt_start: u64) -> u64 {
    if next_utt_start <= seg_end {
        return seg_end; // no gap (overlap or adjacent)
    }
    let gap = next_utt_start - seg_end;
    let extension = (gap / 2).min(TRAILING_GAP_EXTENSION_MS);
    seg_end + extension
}

/// Count utterances with and without timing bullets.
///
/// Returns `(timed, untimed)`: the number of utterances that have a
/// timing bullet and the number that lack one. Non-utterance lines
/// (headers, comments) are not counted.
pub fn count_utterance_timing(chat_file: &ChatFile) -> (usize, usize) {
    let (mut timed, mut untimed) = (0, 0);
    for line in &chat_file.lines {
        if let Line::Utterance(utt) = line {
            if utt.main.content.bullet.is_some() {
                timed += 1;
            } else {
                untimed += 1;
            }
        }
    }
    (timed, untimed)
}

/// Pre-compute interpolated estimates for ALL utterances (indexed by utt_idx).
///
/// For timed utterances the estimate is unused (the real bullet is preferred).
/// For untimed utterances the estimate is interpolated from the nearest
/// neighboring timed utterances, with time distributed proportionally by
/// word count within each gap. Falls back to proportional distribution
/// across the full audio when no timed neighbors exist.
///
/// * `chat_file` - The parsed CHAT file to compute estimates for.
/// * `recording` - The audio being aligned against. Taken as a [`Recording`]
///   rather than as a raw `u64` bound, because a bound pulled out of the type
///   one line into the function is a bound nothing can clamp against safely;
///   `group_utterances` did exactly that before 2026-08-15.
pub fn estimate_untimed_boundaries(chat_file: &ChatFile, recording: &Recording) -> Estimates {
    const BUFFER_MS: u64 = 2000;

    let total_audio_ms = recording.duration().get();
    let mut windows_clamped = 0usize;

    // Collect word counts and existing timing for each utterance.
    let mut info: Vec<(usize, Option<TimeSpan>)> = Vec::new();
    for line in &chat_file.lines {
        if let Line::Utterance(utt) = line {
            let mut words = Vec::new();
            collect_fa_words(&utt.main.content.content, &mut words);
            let span = utt
                .main
                .content
                .bullet
                .as_ref()
                .map(|b| TimeSpan::new(b.timing.start_ms, b.timing.end_ms));
            info.push((words.len(), span));
        }
    }

    if info.is_empty() {
        return Estimates {
            placements: Vec::new(),
            windows_clamped: 0,
        };
    }

    let mut estimates = vec![Placement::Placed(TimeSpan::new(0, 0)); info.len()];

    // Process runs of consecutive untimed utterances between timed anchors.
    // A "run" is a maximal sequence of untimed utterances.
    let mut i = 0;
    while i < info.len() {
        // Skip timed utterances: their estimates are unused.
        if let Some(span) = info[i].1 {
            estimates[i] = Placement::Placed(span);
            i += 1;
            continue;
        }

        // Found start of an untimed run. Find its end.
        let run_start = i;
        while i < info.len() && info[i].1.is_none() {
            i += 1;
        }
        let run_end = i; // exclusive

        // Determine the gap boundaries from neighboring timed utterances.
        let gap_start = if run_start > 0 {
            // Previous timed utterance's end_ms
            info[..run_start]
                .iter()
                .rev()
                .find_map(|(_, span)| span.as_ref())
                .map_or(0, |s| s.end_ms)
        } else {
            0
        };
        let gap_end = if run_end < info.len() {
            // Next timed utterance's start_ms
            info[run_end..]
                .iter()
                .find_map(|(_, span)| span.as_ref())
                .map_or(total_audio_ms, |s| s.start_ms)
        } else {
            total_audio_ms
        };

        // Distribute the gap proportionally by word count.
        let run_words: usize = info[run_start..run_end].iter().map(|(w, _)| w).sum();
        if run_words == 0 {
            // No words: give each utterance a zero-width span at gap_start.
            for est in estimates.iter_mut().take(run_end).skip(run_start) {
                *est = Placement::Placed(TimeSpan::new(gap_start, gap_start));
            }
            continue;
        }

        let gap_duration = gap_end.saturating_sub(gap_start);

        // THE REFUSAL. Distributing words across a gap is arithmetic, and
        // arithmetic will happily produce a window at any density; whether a
        // human could have said those words in that audio is a separate
        // question, and it was never asked. On one real session this placed 175
        // words into 291 ms. The resulting window went to an aligner, which is
        // how invented timings got into a transcript.
        //
        // Refusing is not the same as the skip this function's caller used to
        // perform when the audio length was unknown: that fired on OUR ignorance
        // and silently dropped words. This fires on a stated physical fact,
        // names the rate, and the words stay unaligned because no aligner could
        // have placed them anyway.
        // Measured against the audio the ALIGNER will receive, not the raw gap:
        // each estimate is widened by `BUFFER_MS` at both ends below, so the raw
        // gap understates the window and would refuse runs that are perfectly
        // placeable. An untimed utterance immediately before a timed one has a
        // zero-width raw gap and a two-second real window.
        //
        // It does not rescue the case this check exists for: the measured
        // failure was 175 words in 291 ms, and buffering takes that to 4.3
        // seconds, still 40 words per second and still impossible.
        let available = Ms(gap_duration + 2 * BUFFER_MS);
        let rate = SpeechRate::of(run_words, available);
        if !rate.is_possible() {
            tracing::warn!(
                first_utterance = run_start,
                utterances = run_end - run_start,
                %rate,
                "untimed run cannot fit the audio left for it; leaving it unplaced \
                 rather than handing an impossible window to an aligner"
            );
            for est in estimates.iter_mut().take(run_end).skip(run_start) {
                *est = Placement::Unplaceable(rate);
            }
            continue;
        }

        let mut words_before: usize = 0;
        for idx in run_start..run_end {
            let count = info[idx].0;
            let raw_start =
                gap_start + (words_before as f64 / run_words as f64 * gap_duration as f64) as u64;
            let raw_end = gap_start
                + ((words_before + count) as f64 / run_words as f64 * gap_duration as f64) as u64;

            // The START is clamped too, and this is not symmetry for its own
            // sake. `gap_start` and `gap_end` come from transcript BULLETS,
            // which can lie past the end of the recording; that is the whole
            // phantom-timing story. So a buffered start could exceed the audio
            // while the end below was cut down to it, producing a window whose
            // end precedes its start. `TimeSpan::new`'s doc says "caller is
            // responsible for ensuring start <= end" and this is the caller
            // that could not.
            let start =
                match recording.clamp_bound(FileMs::new(raw_start.saturating_sub(BUFFER_MS))) {
                    Clamped::AsGiven(at) => at.get(),
                    Clamped::ClampedTo { bound } => {
                        windows_clamped += 1;
                        bound.get()
                    }
                };
            // `.min(total_audio_ms)` until 2026-08-15, which is correct
            // arithmetic and invisible behaviour: a window that FITTED and one
            // cut down to fit read identically afterwards. Both arms are named
            // here so the second is acknowledged, and it is counted, because a
            // computed window running past the end of the audio is a fact about
            // OUR gap arithmetic rather than about the recording.
            let end = match recording.clamp_bound(FileMs::new(raw_end + BUFFER_MS)) {
                Clamped::AsGiven(at) => at.get(),
                Clamped::ClampedTo { bound } => {
                    windows_clamped += 1;
                    bound.get()
                }
            };

            // A window with no extent is REFUSED rather than handed on as an
            // inverted or empty `TimeSpan`. `SpeechRate::NoAudio` already means
            // exactly this and already reports `is_possible() == false`, so the
            // case needs no new variant: it is the same answer as the density
            // refusal above, reached a different way.
            estimates[idx] = match end > start {
                true => Placement::Placed(TimeSpan::new(start, end)),
                false => Placement::Unplaceable(SpeechRate::of(count, Ms(0))),
            };
            words_before += count;
        }
    }

    Estimates {
        placements: estimates,
        windows_clamped,
    }
}
