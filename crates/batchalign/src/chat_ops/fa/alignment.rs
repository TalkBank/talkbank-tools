//! Response parsing and deterministic alignment for FA results.

use crate::chat_ops::nlp::{FaIndexedTiming, FaRawResponse, FaRawToken};

use super::coordinates::{Clamped, FaWindow, Ms, OutsideWindow, RecordedInstant, WindowMs};
use super::origin::EngineId;
use super::timing::{SpanFault, SpanRejections, WordSpan};
use super::{FaWord, LAST_WORD_FALLBACK_MS, ModelAlignmentScore, WordTiming};

/// Typed error returned by [`parse_fa_response`].
///
/// Wave 5 of the morphotag reconciliation architecture replaced the
/// previous `Result<_, String>` return with this enum so failure modes
/// can be discriminated at the call site without string parsing. The
/// two variants correspond to structurally distinct problems:
///
/// - `JsonParse`: worker returned text that isn't a valid FA response
///   payload. This is a worker-protocol bug.
/// - `IndexedCountMismatch`: worker returned the wrong number of
///   per-word timings (the FA equivalent of morphotag's
///   `MisalignmentBug`). Always a worker-contract bug, the Python FA
///   worker is supposed to emit one `Option<FaIndexedTiming>` per input
///   `FaWord` for the indexed-word-level response shape.
#[derive(Debug, Clone, thiserror::Error)]
pub enum FaAlignmentError {
    /// The worker's JSON response could not be deserialized into
    /// [`FaRawResponse`](crate::chat_ops::nlp::FaRawResponse).
    #[error("failed to parse raw FA response: {message}")]
    JsonParse {
        /// Underlying serde error rendered as a string (preserved
        /// through the `Clone` boundary; `serde_json::Error` itself is
        /// not `Clone`).
        message: String,
    },
    /// The worker returned an indexed-word-level response whose length
    /// disagrees with the number of input words.
    #[error(
        "FA indexed-response length mismatch: expected {expected} timings for \
         {expected} words, got {actual}"
    )]
    IndexedCountMismatch {
        /// Number of words sent to the worker (expected count).
        expected: usize,
        /// Number of timings actually returned.
        actual: usize,
    },
}

/// Parse the JSON response from the FA callback and align it with original words.
///
/// # Why this takes a window rather than a start offset
///
/// An engine reports positions relative to the AUDIO IT WAS GIVEN. Until
/// 2026-08-15 this function took a bare `audio_start_ms: u64`, added it, and
/// checked only that each resulting end exceeded its start. That check is a
/// relation among the engine's own numbers and says nothing about the audio.
///
/// The failure it allowed, measured on a real session: a 2.263 second group
/// (`1257705..1259968`) was rejected by Wave2Vec for exceeding a CTC target
/// limit and retried on Whisper, which pads its input to a fixed 30 second
/// window and duly reported tokens out to 29.98 seconds. Offsetting those gave
/// word timings at 1287685 in a recording 1259968 milliseconds long. Six of 226
/// screened sessions carried up to 28.2 seconds of such phantom speech.
///
/// Taking the window makes containment askable, and [`FaWindow::to_file`] makes
/// it unskippable: it is the only route from window coordinates into file
/// coordinates, so neither response shape can bypass it.
///
/// # Errors
///
/// Returns [`FaAlignmentError::JsonParse`] if the response isn't valid
/// FA JSON, or [`FaAlignmentError::IndexedCountMismatch`] if the
/// indexed-word-level variant returned the wrong count.
pub fn parse_fa_response(
    json_str: &str,
    original_words: &[FaWord],
    window: &FaWindow,
    engine: &EngineId,
) -> Result<Vec<Option<WordTiming>>, FaAlignmentError> {
    let raw_resp: FaRawResponse =
        serde_json::from_str(json_str).map_err(|e| FaAlignmentError::JsonParse {
            message: e.to_string(),
        })?;

    match raw_resp {
        FaRawResponse::IndexedWordLevel { indexed_timings } => {
            if indexed_timings.len() != original_words.len() {
                return Err(FaAlignmentError::IndexedCountMismatch {
                    expected: original_words.len(),
                    actual: indexed_timings.len(),
                });
            }
            Ok(apply_indexed_timings(
                original_words,
                &indexed_timings,
                window,
                engine,
            ))
        }
        FaRawResponse::TokenLevel { tokens } => {
            Ok(align_token_timings(original_words, &tokens, window, engine))
        }
    }
}

/// Apply index-aligned word timings (no DP remapping required).
fn apply_indexed_timings(
    original: &[FaWord],
    indexed_timings: &[Option<FaIndexedTiming>],
    window: &FaWindow,
    engine: &EngineId,
) -> Vec<Option<WordTiming>> {
    let mut results = vec![None; original.len()];
    let mut discarded = DiscardedTimings::default();
    for (idx, maybe_timing) in indexed_timings.iter().enumerate() {
        let Some(timing) = maybe_timing else {
            continue;
        };
        let model_score = timing.confidence.and_then(|score| {
            ModelAlignmentScore::try_from_f64(score)
                .map_err(|_| discarded.record_invalid_model_score())
                .ok()
        });
        // A word-interval engine reports both ends, so this is the model's own
        // answer, in the coordinates of the audio it was handed. Both ends must
        // land inside that audio: half a span is not a span, so one end outside
        // condemns the pair rather than being repaired from the other.
        match (
            window.to_file(WindowMs::reported(timing.start_ms), engine),
            window.to_file(WindowMs::reported(timing.end_ms), engine),
        ) {
            // Both ends measured, so this is the one constructor that yields a
            // fully observed span. Routed through `WordSpan` rather than
            // straight to `WordTiming` so that both response shapes classify
            // their failures identically: an unusable timing is worse than
            // none, because it reads as a real measurement downstream.
            (Ok(start), Ok(end)) => match WordSpan::measured(start, end) {
                Ok(span) => {
                    // The span's ends already carry their provenance; the
                    // timing takes the END's, because that is the one a later
                    // pass may replace and the one a consumer asks about.
                    results[idx] = WordTiming::new(
                        span.start().at().get(),
                        span.end().at().get(),
                        span.start().origin().clone(),
                        span.end().origin().clone(),
                    )
                    .map(|timing| match model_score {
                        Some(score) => timing.with_model_score(score),
                        None => timing,
                    })
                }
                Err(fault) => discarded.record_span_fault(fault),
            },
            (Err(fault), _) | (Ok(_), Err(fault)) => discarded.record_outside(fault),
        }
    }

    discarded.warn_if_any(indexed_timings.len(), engine, window);
    results
}

/// What a conversion refused, and why.
///
/// The two reasons are different facts about the engine, and a shorter output
/// vector cannot say which occurred. `no_extent` means the model answered but
/// its answer had no width; `outside_window` means it answered about audio it
/// was never given, which is the shape that put 28 seconds of phantom speech
/// into six sessions.
#[derive(Default)]
struct DiscardedTimings {
    /// The classes this path shares with UTR, so both report them alike.
    rejected: SpanRejections,
    /// Assumed word ends that had to be cut back to the recording.
    ///
    /// Not a discard: the word keeps a timing. It is counted here because it is
    /// the same KIND of fact as the others, a place where the output is not
    /// what the engine said, and an operator reading one line wants all of them
    /// together.
    assumed_then_clamped: usize,
    /// Model scores that were non-finite or outside the promised 0..=1 range.
    ///
    /// The interval remains usable: corrupt optional metadata is removed
    /// rather than laundering it or discarding the independent measurement.
    invalid_model_score: usize,
}

impl DiscardedTimings {
    fn record_outside(&mut self, fault: OutsideWindow) {
        self.rejected.record_outside(fault);
    }

    fn record_span_fault(&mut self, fault: SpanFault) {
        self.rejected.record_span_fault(fault);
    }

    /// Record whether a span's end had to be cut back to the recording.
    ///
    /// Takes the [`Clamped`] outcome itself rather than an `Option<Ms>` teased
    /// out of it. Both are two-case values, but only one of them says what the
    /// cases MEAN: `AsGiven` and `ClampedTo` name the fact, while `Some`/`None`
    /// makes the caller remember which way round the question was asked.
    fn note_clamp(&mut self, outcome: &Clamped<WordSpan>) {
        match outcome {
            Clamped::AsGiven(_) => {}
            Clamped::ClampedTo { .. } => self.assumed_then_clamped += 1,
        }
    }

    fn record_invalid_model_score(&mut self) {
        self.invalid_model_score += 1;
    }

    fn warn_if_any(&self, total: usize, engine: &EngineId, window: &FaWindow) {
        // One total decides whether anything is worth saying; the fields then
        // say what. Enumerating the ways "nothing happened" can be true is how
        // a new counter gets forgotten.
        let notable = self.rejected.total() + self.assumed_then_clamped + self.invalid_model_score;
        if notable == 0 {
            return;
        }
        // The out-of-window case is the one that used to corrupt output, so it
        // gets the sentence; the rest ride along as fields.
        let headline = match self.rejected.any_outside_window() {
            false => "some FA word timings were unusable and those words are left unaligned",
            true => {
                "engine reported word timings past the end of the audio it was given; \
                  those words are left unaligned rather than placed outside the recording"
            }
        };
        // Field names come from `SpanRejections` so this line and UTR's can be
        // aggregated together; only `assumed_then_clamped` is this path's own.
        tracing::warn!(
            no_extent = self.rejected.no_extent,
            inverted = self.rejected.inverted,
            outside_window = self.rejected.outside_window,
            worst_overshoot_ms = self.rejected.worst_overshoot.0,
            assumed_then_clamped = self.assumed_then_clamped,
            invalid_model_score = self.invalid_model_score,
            window_len_ms = window.len().0,
            total,
            %engine,
            "{headline}"
        );
    }
}

fn normalize_fa_alignment_unit(text: &str) -> String {
    text.chars()
        .flat_map(|ch| ch.to_lowercase())
        .filter(|ch| ch.is_alphanumeric())
        .collect()
}

/// Align token-level onset times (typical for Whisper) with original CHAT words.
///
/// This path is deterministic only: it stitches normalized Whisper tokens onto
/// normalized transcript words in order. If stitching fails, unmatched words are
/// left as `None` (no DP remapping).
fn align_token_timings(
    original: &[FaWord],
    tokens: &[FaRawToken],
    window: &FaWindow,
    engine: &EngineId,
) -> Vec<Option<WordTiming>> {
    if original.is_empty() || tokens.is_empty() {
        return vec![None; original.len()];
    }

    let mut word_norms = Vec::with_capacity(original.len());
    for word in original {
        let norm = normalize_fa_alignment_unit(word.text.as_str());
        if norm.is_empty() {
            return vec![None; original.len()];
        }
        word_norms.push(norm);
    }

    let mut token_norms = Vec::new();
    // Onsets are kept as `RecordedInstant`s rather than integers, so each one
    // still knows that an engine measured it and which recording it belongs to.
    // That is what lets the span constructors below RELABEL an onset when it is
    // reused as the previous word's end: the same number, no longer a
    // measurement of this word, and the type says so.
    let mut token_onsets: Vec<RecordedInstant> = Vec::with_capacity(tokens.len());
    let mut discarded = DiscardedTimings::default();
    for token in tokens {
        let token_text = token.text.trim();
        if token_text.starts_with("<|") && token_text.ends_with("|>") {
            continue;
        }
        let norm = normalize_fa_alignment_unit(token_text);
        if norm.is_empty() {
            continue;
        }
        // THE fix for this path, and the one the real failure needed. Whisper
        // pads its input to a fixed 30 second window and reports token onsets
        // across the padding, so a 2.3 second group yields onsets out to 29.98
        // seconds. Offsetting those unchecked is what wrote word timings 28
        // seconds past the end of a recording. A token the engine could not
        // have heard is dropped, not placed.
        match window.to_file(WindowMs::reported((token.time_s * 1000.0) as u64), engine) {
            Ok(onset) => {
                token_norms.push(norm);
                token_onsets.push(onset);
            }
            Err(fault) => discarded.record_outside(fault),
        }
    }
    if token_norms.is_empty() {
        discarded.warn_if_any(tokens.len(), engine, window);
        return vec![None; original.len()];
    }

    let mut results = vec![None; original.len()];
    let mut token_idx = 0usize;
    let mut matched_words = 0usize;

    for (word_idx, word_norm) in word_norms.iter().enumerate() {
        if token_idx >= token_norms.len() {
            break;
        }

        let start = &token_onsets[token_idx];
        let mut acc = String::new();
        let mut matched = false;

        while token_idx < token_norms.len() {
            let mut next_acc = acc.clone();
            next_acc.push_str(token_norms[token_idx].as_str());
            if !word_norm.starts_with(next_acc.as_str()) {
                break;
            }
            acc = next_acc;
            token_idx += 1;
            if acc == *word_norm {
                // An onset-only engine says when a word STARTS and never when
                // it ends, so this word's end is always somebody else's number.
                // Which somebody is a fact about the value, and each case has
                // its own constructor rather than an arm in an expression:
                //
                //   a successor exists -> the end is DERIVED from its onset
                //   none does          -> the end is ASSUMED, and may be CLAMPED
                //
                // Both are recorded in the resulting instant's `Origin`, so a
                // later consumer can ask whether this word's extent was ever
                // observed instead of assuming it was.
                let proposed = match token_onsets.get(token_idx) {
                    Some(next_onset) => WordSpan::end_from_next_onset(start.clone(), next_onset)
                        .map(Clamped::AsGiven),
                    None => WordSpan::end_assumed(
                        start.clone(),
                        Ms(LAST_WORD_FALLBACK_MS),
                        &window.recording(),
                    ),
                };

                match proposed {
                    Ok(outcome) => {
                        discarded.note_clamp(&outcome);
                        let span = outcome.value();
                        // Both ends carry their own provenance across, which is
                        // the whole point on this path: an onset-only engine
                        // MEASURED the start and only the end is inferred, so
                        // collapsing the two would report a half-observed word
                        // as wholly derived.
                        results[word_idx] = WordTiming::new(
                            span.start().at().get(),
                            span.end().at().get(),
                            span.start().origin().clone(),
                            span.end().origin().clone(),
                        );
                        matched_words += 1;
                        matched = true;
                    }
                    // A zero-width or inverted span is refused, never nudged: a
                    // word cannot start and end at the same instant, so an
                    // invented millisecond would turn the engine's admission
                    // that it found nothing into a measurement.
                    Err(fault) => discarded.record_span_fault(fault),
                }
                break;
            }
        }

        if !matched {
            break;
        }
    }

    // After the loop, so it reports what the loop actually discarded. It used
    // to run before it, which meant every count the word loop accumulated
    // (clamped assumptions, zero-width spans, inverted spans) was written and
    // never read: bookkeeping that cost work and told nobody anything.
    discarded.warn_if_any(tokens.len(), engine, window);

    if matched_words < original.len() {
        tracing::warn!(
            matched_words,
            total_words = original.len(),
            token_count = token_norms.len(),
            "deterministic token stitching did not cover all words; leaving unmatched words untimed"
        );
    }

    results
}
