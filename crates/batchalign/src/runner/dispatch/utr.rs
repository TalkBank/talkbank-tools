//! UTR (untimed utterance timing recovery) orchestration.
//!
//! This module keeps the full CHAT-level timing-recovery algorithm in Rust:
//! parse CHAT, decide between partial-window and full-file recovery, fetch raw
//! timed tokens from the selected backend, and inject timing bullets back into
//! the AST. Python is only used for the worker-hosted ASR path.

use std::path::Path;

use crate::api::{DurationMs, DurationSeconds, LanguageCode3, NumSpeakers};
use crate::cache::CacheBackend;
use crate::chat_ops::fa::coordinates::{FaWindow, FileMs, Recording, WindowMs};
use crate::chat_ops::fa::origin::EngineId;
use crate::chat_ops::fa::timing::{SpanRejections, WordSpan};
use crate::options::{UtrEngine, UtrOverlapStrategy};
use crate::params::CachePolicy;
use crate::pipeline::PipelineServices;
use crate::runner::debug_dumper::DebugDumper;
use tracing::{info, warn};

/// Immutable runtime inputs for one UTR execution.
#[derive(Clone, Copy)]
pub(in crate::runner) struct UtrPassContext<'a> {
    /// Audio file used to recover utterance timing.
    pub audio_path: &'a Path,
    /// CHAT language for ASR/UTR normalization.
    pub lang: &'a LanguageCode3,
    /// Shared worker pool/cache handles for the current pipeline stage.
    pub services: PipelineServices<'a>,
    /// Audio identity used to key UTR cache entries.
    pub audio_identity: &'a crate::chat_ops::fa::AudioIdentity,
    /// Cache policy selected for the current job.
    pub cache_policy: CachePolicy,
    /// Total audio duration in milliseconds when known.
    pub total_audio_ms: Option<DurationMs>,
    /// Maximum FA group duration in milliseconds. Used by the two-pass UTR
    /// strategy to compare FA grouping outcomes and detect the wider-window
    /// regression on non-English files.
    pub max_group_ms: Option<DurationMs>,
    /// Display filename for logging.
    pub filename: &'a str,
    /// Selected UTR backend.
    pub engine: &'a UtrEngine,
    /// Overlap strategy for `+<` utterances.
    pub overlap_strategy: UtrOverlapStrategy,
    /// Optional Rev.AI job ID produced by preflight submission.
    pub rev_job_id: Option<&'a str>,
    /// Debug artifact writer for offline replay.
    pub dumper: &'a DebugDumper,
}

impl<'a> UtrPassContext<'a> {
    /// The recording this pass is working against.
    ///
    /// Delegates to [`AudioContext::recording`] rather than repeating its
    /// probe-or-use logic. This context carries the same two facts an
    /// `AudioContext` does (`audio_path`, `total_audio_ms`), so a second
    /// derivation here would be the duplication that method exists to end, with
    /// its own divergent error message. One question, one owner.
    async fn recording(&self) -> Result<Recording, crate::error::ServerError> {
        crate::params::AudioContext {
            audio_path: self.audio_path,
            audio_identity: self.audio_identity,
            total_audio_ms: self.total_audio_ms,
        }
        .recording()
        .await
    }

    /// Return the same context without a preflight job ID.
    ///
    /// Segment-level partial UTR extracts fresh temporary WAV files, so those
    /// requests can never reuse a full-file Rev.AI preflight submission.
    fn without_rev_job_id(self) -> Self {
        Self {
            rev_job_id: None,
            ..self
        }
    }
}

/// Resolve the UTR overlap strategy for a specific CHAT file.
///
/// `Auto` currently always returns `GlobalUtr` regardless of file content
/// or language: the previous content/language-aware selection (which
/// auto-picked `TwoPassOverlapUtr` for English files containing `+<` or
/// `⌊` markers) was disabled 2026-03-30 after operator-reported alignment
/// regressions and the discovery that `enforce_monotonicity()` only checks
/// start times. See the inline comment in the `Auto` arm below for the
/// full rationale. `_chat_file` is retained on the signature so re-enabling
/// content inspection later does not require a function-shape change.
///
/// `Global` and `TwoPass` are explicit user overrides and are used as-is.
///
/// When `total_audio_ms` and `max_group_ms` are both available, a
/// [`GroupingContext`](crate::chat_ops::fa::GroupingContext) is passed to
/// the two-pass strategy so it can compare FA group counts and avoid the
/// wider-window regression on non-English files. This is only consulted on
/// the `TwoPass` override path; `Auto` no longer reaches it.
fn resolve_strategy(
    strategy: UtrOverlapStrategy,
    _chat_file: &crate::chat_ops::ChatFile,
    context: &UtrPassContext<'_>,
) -> Box<dyn crate::chat_ops::fa::UtrStrategy> {
    let grouping_context = match (context.total_audio_ms, context.max_group_ms) {
        (Some(total_audio_ms), Some(max_group_ms)) => Some(crate::chat_ops::fa::GroupingContext {
            total_audio_ms: total_audio_ms.0,
            max_group_ms: max_group_ms.0,
        }),
        _ => None,
    };

    match strategy {
        UtrOverlapStrategy::Auto => {
            // Two-pass overlap strategy is experimental and gated behind
            // --utr-strategy two-pass.  Auto always uses GlobalUtr until
            // the two-pass algorithm is validated on an operator's problem files
            // and the end-time overlap bug is resolved.
            //
            // Previous behavior: auto-selected TwoPassOverlapUtr for English
            // files with +< or ⌊ markers.  Disabled 2026-03-30 because:
            // 1. an operator reported alignment regressions on real files.
            // 2. enforce_monotonicity() only checks start times, not end
            //    times, so overlapping utterance bullets go uncorrected.
            // 3. Two-pass was tuned on 4 corpora but not broadly validated.
            Box::new(crate::chat_ops::fa::GlobalUtr)
        }
        UtrOverlapStrategy::Global => Box::new(crate::chat_ops::fa::GlobalUtr),
        UtrOverlapStrategy::TwoPass => Box::new(crate::chat_ops::fa::TwoPassOverlapUtr {
            grouping_context,
            config: crate::chat_ops::fa::TwoPassConfig::default(),
        }),
    }
}

/// Run ASR and inject UTR timing into a parsed `ChatFile`.
///
/// Returns `Ok((updated_chat_text, utr_result))` on success, or
/// `Err(original_chat_text)` on inference failure.
///
/// When `progress` is provided, per-window updates are sent during partial UTR
/// so frontends can show "Recovering utterance timing 2/5" etc.
///
/// Mutates `chat_file` in place, no serialize/re-parse cycle. The caller owns
/// the AST and can pass it directly to FA without a round-trip through text.
pub(in crate::runner) async fn run_utr_pass(
    chat_file: &mut crate::chat_ops::ChatFile,
    context: UtrPassContext<'_>,
    progress: Option<&super::super::util::ProgressSender>,
) -> Result<crate::chat_ops::fa::utr::UtrResult, crate::error::ServerError> {
    use crate::chat_ops::CacheTaskName;

    let (timed, untimed) = crate::chat_ops::fa::count_utterance_timing(chat_file);
    let total_utts = timed + untimed;

    if untimed == 0 {
        info!(context.filename, "UTR pass: no untimed utterances");
        return Ok(crate::chat_ops::fa::utr::UtrResult {
            injected: 0,
            skipped: timed,
            unmatched: 0,
            decisions: Vec::new(),
        });
    }

    info!(
        context.filename,
        timed,
        untimed,
        engine = context.engine.as_wire_name(),
        "UTR pass: running timing recovery"
    );

    // Partial-window UTR is useful for worker-hosted ASR because it can avoid
    // sending already-timed regions through local model inference. For the
    // Rust-owned Rev.AI path, full-file polling is the better boundary: one
    // provider job, one transcript projection, no segment upload fan-out.
    let untimed_ratio = if total_utts > 0 {
        untimed as f64 / total_utts as f64
    } else {
        1.0
    };
    let use_partial = context.engine.supports_partial_windows()
        && untimed_ratio < 0.5
        && context.total_audio_ms.is_some_and(|ms| ms.0 > 60_000);

    if use_partial {
        // The recording every window and every recovered token is checked
        // against. Asked of the context so this path shares the crate's single
        // derivation; a zero-length or unprobeable recording degrades to
        // full-file recovery rather than failing, because partial UTR is an
        // optimisation and the full path does not need the bound.
        let recording = match context.recording().await {
            Ok(recording) => recording,
            Err(why) => {
                warn!(
                    context.filename,
                    error = %why,
                    "Partial UTR needs a non-empty recording, falling back to full-file recovery"
                );
                return run_utr_pass_full(chat_file, context).await;
            }
        };
        // Named once, so every token recovered in this pass records the same
        // engine as its provenance.
        let utr_engine_id = EngineId::new(context.engine.as_wire_name());
        // The recording built above is the single derivation of the audio's
        // length on this path. It used to be derived a SECOND time three lines
        // later, from `context.total_audio_ms` through an `expect` whose safety
        // rested on a control-flow invariant argued in a four-line comment.
        // Passing the recording deletes the second derivation, the panic and
        // the argument together.
        let windows = crate::chat_ops::fa::find_untimed_windows(chat_file, &recording, 500);

        if windows.is_empty() {
            info!(
                context.filename,
                "Partial UTR: no windows found, falling back to full-file recovery"
            );
        } else {
            info!(
                context.filename,
                windows = windows.len(),
                "Partial UTR: running ASR on untimed windows only"
            );

            let mut all_tokens: Vec<crate::chat_ops::fa::utr::AsrTimingToken> = Vec::new();
            let total_windows = windows.len() as i64;

            for (window_idx, window) in windows.iter().enumerate() {
                let (start_ms, end_ms) = (window.start().get(), window.end().get());
                let seg_cache_key = crate::chat_ops::fa::utr_asr_segment_cache_key(
                    context.audio_identity,
                    start_ms,
                    end_ms,
                    context.lang,
                );
                let cached_seg = if context.cache_policy != CachePolicy::SkipCache {
                    match context
                        .services
                        .cache
                        .get(
                            seg_cache_key.as_str(),
                            CacheTaskName::UtrAsr.as_str(),
                            context.services.engine_version,
                        )
                        .await
                    {
                        Ok(Some(value)) => {
                            info!(context.filename, start_ms, end_ms, "UTR segment cache hit");
                            serde_json::from_value::<crate::transcribe::AsrResponse>(value).ok()
                        }
                        _ => None,
                    }
                } else {
                    None
                };

                let seg_response = if let Some(cached) = cached_seg {
                    cached
                } else {
                    let segment_path =
                        match crate::ensure_wav::extract_audio_segment(context.audio_path, *window)
                            .await
                        {
                            Ok(path) => path,
                            Err(error) => {
                                warn!(
                                    context.filename,
                                    error = %error,
                                    start_ms,
                                    end_ms,
                                    "Failed to extract audio segment, falling back to full UTR"
                                );
                                return run_utr_pass_full(chat_file, context).await;
                            }
                        };

                    match infer_utr_asr_response(&segment_path, context.without_rev_job_id()).await
                    {
                        Ok(response) => {
                            let ba_version = env!("CARGO_PKG_VERSION");
                            if let Ok(value) = serde_json::to_value(&response)
                                && let Err(error) = context
                                    .services
                                    .cache
                                    .put(
                                        seg_cache_key.as_str(),
                                        CacheTaskName::UtrAsr.as_str(),
                                        context.services.engine_version,
                                        ba_version,
                                        &value,
                                    )
                                    .await
                            {
                                warn!(
                                    context.filename,
                                    error = %error,
                                    "Failed to cache UTR segment (non-fatal)"
                                );
                            }
                            response
                        }
                        Err(error) => {
                            warn!(
                                context.filename,
                                error = %error,
                                "UTR segment ASR failed, falling back to full-file recovery"
                            );
                            return run_utr_pass_full(chat_file, context).await;
                        }
                    }
                };

                // The segment handed to the engine, as a window inside the
                // recording. `find_untimed_windows` already clamps both ends,
                // so this cannot fail; the failure arm is written out rather
                // than unwrapped so that a change there breaks here instead of
                // silently reinstating an unbounded offset.
                //
                // Converted from the `MediaWindow` itself rather than from the
                // two integers destructured out of it, so the ordering proof it
                // already carries is not thrown away and rebuilt here.
                let fa_window = match FaWindow::over(&recording, *window) {
                    Ok(window) => window,
                    Err(why) => {
                        warn!(
                            context.filename,
                            error = %why,
                            start_ms,
                            end_ms,
                            "UTR window is not inside the recording, skipping it"
                        );
                        continue;
                    }
                };
                let converted =
                    asr_response_to_utr_tokens(&seg_response, &fa_window, &utr_engine_id);
                converted.warn_if_lossy(&context);
                all_tokens.extend(converted.tokens);

                // Report per-window progress so the frontend shows "Recovering
                // utterance timing 2/5" as each window's ASR completes.
                if let Some(tx) = progress {
                    use super::super::util::{FileStage, ProgressUpdate};
                    let _ = tx.send(ProgressUpdate::new(
                        FileStage::RecoveringUtteranceTiming,
                        Some(window_idx as i64 + 1),
                        Some(total_windows),
                    ));
                }
            }

            all_tokens.sort_by_key(|token| token.start_ms);

            if context.dumper.is_enabled() {
                let text = batchalign_transform::serialize::to_chat_string(chat_file);
                context.dumper.dump_utr_input(context.filename, &text);
            }
            context
                .dumper
                .dump_utr_tokens(context.filename, &all_tokens);

            let strategy = resolve_strategy(context.overlap_strategy, chat_file, &context);
            let utr_result = strategy.inject(chat_file, &all_tokens);

            info!(
                context.filename,
                injected = utr_result.injected,
                skipped = utr_result.skipped,
                unmatched = utr_result.unmatched,
                "UTR partial pass complete"
            );

            if context.dumper.is_enabled() {
                let text = batchalign_transform::serialize::to_chat_string(chat_file);
                context
                    .dumper
                    .dump_utr_output(context.filename, &text, &utr_result);
            }
            return Ok(utr_result);
        }
    }

    // Full-file path: signal 0/1 so the frontend knows it's a single-window pass.
    if let Some(tx) = progress {
        use super::super::util::{FileStage, ProgressUpdate};
        let _ = tx.send(ProgressUpdate::new(
            FileStage::RecoveringUtteranceTiming,
            Some(0),
            Some(1),
        ));
    }

    run_utr_pass_full(chat_file, context).await
}

/// Run the full-file UTR path with cache reuse.
async fn run_utr_pass_full(
    chat_file: &mut crate::chat_ops::ChatFile,
    context: UtrPassContext<'_>,
) -> Result<crate::chat_ops::fa::utr::UtrResult, crate::error::ServerError> {
    use crate::chat_ops::CacheTaskName;

    let cache_key = crate::chat_ops::fa::utr_asr_cache_key(context.audio_identity, context.lang);
    let cached_response = if context.cache_policy != CachePolicy::SkipCache {
        match context
            .services
            .cache
            .get(
                cache_key.as_str(),
                CacheTaskName::UtrAsr.as_str(),
                context.services.engine_version,
            )
            .await
        {
            Ok(Some(value)) => {
                info!(context.filename, "UTR ASR cache hit");
                serde_json::from_value::<crate::transcribe::AsrResponse>(value).ok()
            }
            Ok(None) => None,
            Err(error) => {
                warn!(
                    context.filename,
                    error = %error,
                    "UTR ASR cache lookup failed (non-fatal)"
                );
                None
            }
        }
    } else {
        None
    };

    let asr_response = if let Some(cached) = cached_response {
        cached
    } else {
        info!(
            context.filename,
            engine = context.engine.as_wire_name(),
            "UTR ASR cache miss, running inference"
        );
        match infer_utr_asr_response(context.audio_path, context).await {
            Ok(response) => {
                let ba_version = env!("CARGO_PKG_VERSION");
                if let Ok(value) = serde_json::to_value(&response)
                    && let Err(error) = context
                        .services
                        .cache
                        .put(
                            cache_key.as_str(),
                            CacheTaskName::UtrAsr.as_str(),
                            context.services.engine_version,
                            ba_version,
                            &value,
                        )
                        .await
                {
                    warn!(
                        context.filename,
                        error = %error,
                        "Failed to cache UTR ASR result (non-fatal)"
                    );
                }
                response
            }
            Err(error) => {
                warn!(context.filename, error = %error, "UTR ASR inference failed");
                return Err(error);
            }
        }
    };

    // The whole file is the window here, so the offset is zero, but the BOUND
    // still matters: an engine that loses alignment reports positions past the
    // end of the audio whether or not a window offset is involved. This path
    // passed a bare `0` and checked nothing.
    let recording = context.recording().await?;
    let whole_file =
        FaWindow::within(&recording, FileMs::new(0), recording.duration()).map_err(|why| {
            crate::error::ServerError::RecordingDuration(format!(
                "the whole recording is not a valid window over itself: {why}"
            ))
        })?;
    let converted = asr_response_to_utr_tokens(
        &asr_response,
        &whole_file,
        &EngineId::new(context.engine.as_wire_name()),
    );
    converted.warn_if_lossy(&context);
    let asr_tokens = converted.tokens;

    if context.dumper.is_enabled() {
        let text = batchalign_transform::serialize::to_chat_string(chat_file);
        context.dumper.dump_utr_input(context.filename, &text);
    }
    context
        .dumper
        .dump_utr_tokens(context.filename, &asr_tokens);

    let strategy = resolve_strategy(context.overlap_strategy, chat_file, &context);
    let utr_result = strategy.inject(chat_file, &asr_tokens);

    info!(
        context.filename,
        injected = utr_result.injected,
        skipped = utr_result.skipped,
        unmatched = utr_result.unmatched,
        "UTR pass complete"
    );

    if context.dumper.is_enabled() {
        let text = batchalign_transform::serialize::to_chat_string(chat_file);
        context
            .dumper
            .dump_utr_output(context.filename, &text, &utr_result);
    }
    Ok(utr_result)
}

/// Fetch one UTR ASR response from the selected backend and map it into the
/// shared `AsrResponse` cache format.
async fn infer_utr_asr_response(
    audio_path: &Path,
    context: UtrPassContext<'_>,
) -> Result<crate::transcribe::AsrResponse, crate::error::ServerError> {
    match context.engine {
        UtrEngine::RevAi => {
            let tokens =
                crate::revai::infer_revai_utr(audio_path, context.lang, context.rev_job_id).await?;
            Ok(crate::transcribe::AsrResponse {
                tokens: tokens
                    .into_iter()
                    .map(|token| crate::transcribe::AsrToken {
                        text: token.text,
                        start_s: Some(DurationSeconds(token.start_ms as f64 / 1000.0)),
                        end_s: Some(DurationSeconds(token.end_ms as f64 / 1000.0)),
                        speaker: None,
                        confidence: None,
                    })
                    .collect(),
                lang: context.lang.clone(),
                source_monologues: None,
            })
        }
        UtrEngine::Whisper | UtrEngine::HkTencent => {
            // UTR uses the default per-engine configuration; there are
            // no UTR-specific knobs in `EngineOverrides.extras` today.
            // An empty extras map preserves the current behavior and
            // gives a stable place to wire UTR-time knobs later if any
            // engine grows them.
            let empty_extras = std::collections::BTreeMap::new();
            crate::transcribe::infer_asr(
                context.services.pool,
                &crate::transcribe::AsrInferParams {
                    backend: crate::transcribe::AsrBackend::Worker(
                        crate::transcribe::AsrWorkerMode::LocalWhisperV2,
                    ),
                    audio_path,
                    lang: &crate::api::LanguageSpec::Resolved(context.lang.clone()),
                    num_speakers: NumSpeakers(1),
                    rev_job_id: None,
                    extras: &empty_extras,
                },
            )
            .await
        }
    }
}

/// What a conversion kept, and what it discarded.
///
/// The two discard reasons are different facts about the run: an engine that
/// reports no word timings and an engine whose timings all collapsed both
/// leave UTR with nothing to inject, and a bare token list cannot say which.
pub(crate) struct UtrTokenConversion {
    pub(crate) tokens: Vec<crate::chat_ops::fa::utr::AsrTimingToken>,
    /// Tokens the engine gave no start or no end for.
    ///
    /// This path's own field: an ASR token can carry no span at all, which the
    /// FA path never sees because its engines answer per word.
    pub(crate) dropped_untimed: usize,
    /// The classes this path shares with FA, reported under the same names so
    /// one engine failure can be aggregated across both.
    pub(crate) rejected: SpanRejections,
}

impl UtrTokenConversion {
    /// Log at most one line describing what was discarded.
    pub(crate) fn warn_if_lossy(&self, context: &UtrPassContext<'_>) {
        // Nothing discarded is the common case and deserves no log line.
        // Written as a total over the three counts rather than a three-way
        // pattern: the question is "was anything lost", and summing says that
        // once instead of enumerating the ways it could be false.
        let discarded = self.dropped_untimed + self.rejected.total();
        if discarded == 0 {
            return;
        }
        // Field names match the FA path's line, so "how many out-of-window
        // rejections did this file have" is one query rather than two.
        tracing::warn!(
            filename = context.filename,
            engine = context.engine.as_wire_name(),
            total = self.tokens.len() + discarded,
            dropped_untimed = self.dropped_untimed,
            no_extent = self.rejected.no_extent,
            inverted = self.rejected.inverted,
            outside_window = self.rejected.outside_window,
            worst_overshoot_ms = self.rejected.worst_overshoot.0,
            "ASR tokens discarded before UTR"
        );
    }
}

/// Convert an ASR response over one extracted audio segment into the timing
/// tokens the UTR injector consumes, moving them from SEGMENT coordinates into
/// FILE coordinates.
///
/// # Why this takes a window rather than an offset
///
/// It took a bare `offset_ms: u64` until 2026-08-15, added it to the engine's
/// reported times, and validated only that the result's end exceeded its start.
/// That is a relation among the engine's own numbers and says nothing about the
/// audio: an engine that loses alignment on a short segment reports positions
/// past the end of the slice it was handed, and the offset then places them
/// past the end of the whole recording. Six of 226 screened sessions carried
/// timings up to 28.2 seconds beyond their own media because of it, and nothing
/// in the pipeline was in a position to notice, because the segment's length
/// was never passed in.
///
/// Taking the window makes the containment question askable, and
/// [`FaWindow::to_file`] makes it unskippable: it is the only route from
/// [`WindowMs`] to a file-coordinate instant.
///
/// Zero-duration tokens are still dropped. Whisper's DTW timestamp extraction
/// works at 20ms resolution and can assign `start == end` to very short words
/// (single-frame backchannels like "mhm", "yeah"). Such tokens carry no useful
/// interval information and, if allowed through, cause UTR to create `•T_T•`
/// utterance bullets that the FA postprocess then perpetuates indefinitely
/// (see OCSC bug, 2026-04-08).
fn asr_response_to_utr_tokens(
    asr_response: &crate::transcribe::AsrResponse,
    window: &FaWindow,
    engine: &EngineId,
) -> UtrTokenConversion {
    let mut conversion = UtrTokenConversion {
        tokens: Vec::with_capacity(asr_response.tokens.len()),
        dropped_untimed: 0,
        rejected: SpanRejections::default(),
    };
    for token in &asr_response.tokens {
        let (Some(start_s), Some(end_s)) = (token.start_s, token.end_s) else {
            // The engine reported no span for this token. Distinct from a span
            // it reported as empty: that is a measurement, this is a silence.
            conversion.dropped_untimed += 1;
            continue;
        };
        // Seconds relative to the extracted segment. Naming the space is the
        // point: these cannot be written to a transcript without conversion,
        // because a transcript's timings are not measured from a segment.
        let reported_start = WindowMs::reported((start_s.0 * 1000.0).round() as u64);
        let reported_end = WindowMs::reported((end_s.0 * 1000.0).round() as u64);

        match (
            window.to_file(reported_start, engine),
            window.to_file(reported_end, engine),
        ) {
            // Built through `WordSpan` rather than by comparing the two ends
            // here. A bare `end > start` is a bool over exactly the relation
            // `SpanFault` splits into `NoExtent` and `Inverted`, so testing it
            // by hand both re-implements the constructor and throws away which
            // of the two happened, leaving this path's counters incomparable
            // with the FA path's for the same engine failure.
            (Ok(start), Ok(end)) => match WordSpan::measured(start, end) {
                Ok(span) => conversion
                    .tokens
                    .push(crate::chat_ops::fa::utr::AsrTimingToken {
                        text: token.text.clone(),
                        start_ms: span.start().at().get(),
                        end_ms: span.end().at().get(),
                    }),
                Err(fault) => conversion.rejected.record_span_fault(fault),
            },
            // Either end past the segment condemns the token: half of a span is
            // not a span, and keeping the surviving end would invent the other.
            (Err(fault), _) | (Ok(_), Err(fault)) => conversion.rejected.record_outside(fault),
        }
    }
    conversion
}

#[cfg(test)]
mod utr_token_conversion_tests {
    use super::*;
    use crate::chat_ops::fa::coordinates::Ms;
    use crate::transcribe::{AsrResponse, AsrToken};

    fn response(tokens: Vec<AsrToken>) -> AsrResponse {
        AsrResponse {
            tokens,
            lang: LanguageCode3::eng(),
            source_monologues: None,
        }
    }

    fn token(text: &str, start_s: Option<f64>, end_s: Option<f64>) -> AsrToken {
        AsrToken {
            text: text.to_owned(),
            start_s: start_s.map(DurationSeconds),
            end_s: end_s.map(DurationSeconds),
            speaker: None,
            confidence: None,
        }
    }

    /// The distinction this type exists for.
    ///
    /// Both discards used to produce a shorter vector and nothing else, so an
    /// engine reporting no word timings and an engine whose timings collapsed
    /// were indistinguishable at the call site: each left UTR with nothing to
    /// inject and no way to say which had happened.
    /// A recording long enough that the windows below sit comfortably inside
    /// it, so these tests exercise conversion rather than containment.
    fn wide_window(start_ms: u64, end_ms: u64) -> (Recording, FaWindow) {
        let recording = Recording::of_duration(Ms(600_000)).expect("non-zero");
        let window = FaWindow::within(&recording, FileMs::new(start_ms), FileMs::new(end_ms))
            .expect("window inside the recording");
        (recording, window)
    }

    fn test_engine() -> EngineId {
        EngineId::new("test-asr")
    }

    #[test]
    fn untimed_and_degenerate_discards_are_counted_apart() {
        let (_rec, window) = wide_window(0, 60_000);
        let converted = asr_response_to_utr_tokens(
            &response(vec![
                token("kept", Some(1.0), Some(1.5)),
                token("no start", None, Some(2.0)),
                token("no end", Some(2.0), None),
                token("empty span", Some(3.0), Some(3.0)),
                token("inverted", Some(5.0), Some(4.0)),
            ]),
            &window,
            &test_engine(),
        );

        assert_eq!(converted.tokens.len(), 1);
        assert_eq!(converted.tokens[0].text, "kept");
        assert_eq!(converted.dropped_untimed, 2);
        // These two were one counter (`dropped_degenerate == 2`) until
        // 2026-08-15, when this path started building spans through
        // `WordSpan::measured` instead of testing `end > start` by hand. The
        // expectation changed because the CONFLATION was the defect: an empty
        // span is a model that found nothing, an inverted one is a model whose
        // two answers contradict each other, and the FA path had always kept
        // them apart. One number for both made the two paths' logs
        // incomparable for the same engine failure.
        assert_eq!(converted.rejected.no_extent, 1);
        assert_eq!(converted.rejected.inverted, 1);
        assert_eq!(converted.rejected.outside_window, 0);
    }

    #[test]
    fn a_clean_response_reports_no_losses() {
        let (_rec, window) = wide_window(0, 60_000);
        let converted = asr_response_to_utr_tokens(
            &response(vec![
                token("one", Some(0.0), Some(0.5)),
                token("two", Some(0.5), Some(1.0)),
            ]),
            &window,
            &test_engine(),
        );
        assert_eq!(converted.tokens.len(), 2);
        assert_eq!(converted.dropped_untimed, 0);
        assert_eq!(converted.rejected.no_extent, 0);
        assert_eq!(converted.rejected.outside_window, 0);
    }

    #[test]
    fn the_offset_applies_to_both_ends() {
        let (_rec, window) = wide_window(10_000, 70_000);
        let converted = asr_response_to_utr_tokens(
            &response(vec![token("w", Some(1.0), Some(2.0))]),
            &window,
            &test_engine(),
        );
        assert_eq!(converted.tokens[0].start_ms, 11_000);
        assert_eq!(converted.tokens[0].end_ms, 12_000);
    }

    /// The defect this signature change exists to prevent.
    ///
    /// UTR extracts a physical audio slice and asks an engine about it. An
    /// engine that loses alignment on a short slice reports positions past the
    /// end of the audio it was handed; the old signature took a bare offset,
    /// added it, checked only that end exceeded start, and wrote the result
    /// out. That is how six of 226 screened sessions came to carry timings up
    /// to 28.2 seconds past the end of their own media.
    ///
    /// The numbers here are the real ones: a 4.803 second window at the very
    /// end of a 1259.968 second recording, and an engine reporting 33.02
    /// seconds into it.
    #[test]
    fn a_token_past_the_end_of_its_own_segment_is_refused_not_offset() {
        let recording = Recording::of_duration(Ms(1_259_968)).expect("non-zero");
        let window = FaWindow::within(&recording, FileMs::new(1_255_165), FileMs::new(1_259_968))
            .expect("window inside the recording");

        let converted = asr_response_to_utr_tokens(
            &response(vec![
                token("real", Some(1.0), Some(2.0)),
                token("hallucinated", Some(33.02), Some(33.5)),
            ]),
            &window,
            &test_engine(),
        );

        // The plausible token survives; the impossible one is discarded and
        // COUNTED, rather than being written out as a measurement.
        assert_eq!(converted.tokens.len(), 1);
        assert_eq!(converted.tokens[0].text, "real");
        assert_eq!(converted.rejected.outside_window, 1);
        assert_eq!(converted.rejected.worst_overshoot, Ms(28_217));

        // The property that actually matters, stated directly: nothing this
        // conversion emits can name a moment the recording does not contain.
        for kept in &converted.tokens {
            assert!(kept.end_ms <= recording.duration().get());
        }
    }
}
