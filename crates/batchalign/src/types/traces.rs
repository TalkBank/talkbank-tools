//! Trace data structures for algorithm visualization.
//!
//! Orchestrators return structured result types (e.g. [`super::results::FaResult`])
//! that always carry intermediate data.  When `debug_traces` is enabled for a
//! job, the dispatch layer converts these results into trace structs and stores
//! them in the ephemeral [`crate::trace_store::TraceStore`].  The dashboard
//! fetches them via `GET /jobs/{id}/traces`.
//!
//! When `debug_traces` is off (the default), structured results are still
//! returned but traces are not stored, no extra memory is used.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::api::{DurationMs, DurationSeconds};
use crate::chat_ops::fa::origin::{ClampBound, Origin};

// ---------------------------------------------------------------------------
// Top-level containers
// ---------------------------------------------------------------------------

/// All algorithm traces collected for a completed job.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct JobTraces {
    /// Per-file traces, keyed by file index (0-based).
    pub files: BTreeMap<usize, FileTraces>,
}

/// Algorithm traces for a single file within a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct FileTraces {
    /// Original filename (e.g. "01DM_18.cha").
    pub filename: crate::api::DisplayPath,
    /// DP alignment traces (one per alignment call: FA, retokenize, WER).
    pub dp_alignments: Vec<DpAlignmentTrace>,
    /// ASR post-processing pipeline trace (transcribe jobs only).
    pub asr_pipeline: Option<AsrPipelineTrace>,
    /// Forced alignment timeline trace (align jobs only).
    pub fa_timeline: Option<FaTimelineTrace>,
    /// Retokenization traces (one per utterance that was retokenized).
    pub retokenizations: Vec<RetokenizationTrace>,
}

// ---------------------------------------------------------------------------
// DP Alignment
// ---------------------------------------------------------------------------

/// Full matrix + traceback for a single `align_small` invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct DpAlignmentTrace {
    /// What triggered this alignment (e.g. "fa_whisper", "retokenize", "wer").
    pub context: String,
    /// Payload sequence (left side).
    pub payload: Vec<String>,
    /// Reference sequence (top side).
    pub reference: Vec<String>,
    /// Match mode used ("exact" or "case_insensitive").
    pub match_mode: String,
    /// Number of prefix elements stripped before DP.
    pub prefix_stripped: usize,
    /// Number of suffix elements stripped before DP.
    pub suffix_stripped: usize,
    /// Flat cost matrix (row-major, `(ref_len+1) * (pay_len+1)` entries).
    pub cost_matrix: Vec<usize>,
    /// Traceback path through the cost matrix.
    pub traceback: Vec<AlignStepTrace>,
    /// Final alignment result.
    pub result: Vec<AlignResultTrace>,
}

/// A single step in the DP traceback path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct AlignStepTrace {
    /// Action taken: "match", "substitution", "extra_payload", "extra_reference".
    pub action: String,
    /// Row index in the cost matrix.
    pub i: usize,
    /// Column index in the cost matrix.
    pub j: usize,
}

/// A single item in the alignment result (matches `AlignResult` enum).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct AlignResultTrace {
    /// "match", "extra_payload", or "extra_reference".
    pub kind: String,
    /// The string key.
    pub key: String,
    /// Index into payload (present for match and extra_payload).
    pub payload_idx: Option<usize>,
    /// Index into reference (present for match and extra_reference).
    pub reference_idx: Option<usize>,
}

// ---------------------------------------------------------------------------
// ASR Pipeline
// ---------------------------------------------------------------------------

/// Intermediate word lists at each stage of ASR post-processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct AsrPipelineTrace {
    /// Stage 0: raw tokens from the ASR worker.
    pub raw_tokens: Vec<AsrTokenTrace>,
    /// Stage 1: after compound merging.
    pub after_compound_merge: Vec<WordTrace>,
    /// Stage 2: after timed word extraction (seconds → ms).
    pub after_timing_extract: Vec<TimedWordTrace>,
    /// Stage 3: after multi-word splitting.
    pub after_multiword_split: Vec<TimedWordTrace>,
    /// Stage 4: after number expansion.
    pub after_number_expand: Vec<TimedWordTrace>,
    /// Stage 4b: after Cantonese normalization (only if lang=yue).
    pub after_cantonese_norm: Option<Vec<TimedWordTrace>>,
    /// Stage 5: after long-turn splitting (nested by turn).
    pub after_long_turn_split: Vec<Vec<TimedWordTrace>>,
    /// Stage 6: final utterances.
    pub final_utterances: Vec<UtteranceTrace>,
}

/// A raw ASR token (stage 0).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct AsrTokenTrace {
    /// Token text.
    pub value: String,
    /// Start time in seconds.
    pub ts: DurationSeconds,
    /// End time in seconds.
    pub end_ts: DurationSeconds,
    /// Token type ("text", "punctuation", etc.).
    pub token_type: String,
}

/// A word without timing (e.g. after compound merge).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct WordTrace {
    /// Word text.
    pub text: String,
}

/// A word with optional timing in milliseconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct TimedWordTrace {
    /// Word text.
    pub text: String,
    /// Start time in ms (if known).
    pub start_ms: Option<i64>,
    /// End time in ms (if known).
    pub end_ms: Option<i64>,
}

/// A final utterance (stage 6).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct UtteranceTrace {
    /// Speaker index (0-based).
    pub speaker: usize,
    /// Words in the utterance.
    pub words: Vec<TimedWordTrace>,
}

// ---------------------------------------------------------------------------
// FA Timeline
// ---------------------------------------------------------------------------

/// Forced alignment trace: grouping, timing injection, and post-processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct FaTimelineTrace {
    /// Evidence schema revision. Version 1 records pre-injection timing,
    /// confidence, and full boundary provenance. Version 2 additionally
    /// records the typed post-injection decisions that altered or removed
    /// timing; `post_injection_timings` remains reserved for a later complete
    /// per-word outcome projection.
    #[serde(default)]
    pub evidence_schema_version: u32,
    /// Forced-alignment engine selected for this run.
    #[serde(default)]
    pub engine: String,
    /// Build/model revision used in cache identity for this run.
    #[serde(default)]
    pub engine_version: String,
    /// Utterance groups for batched FA.
    pub groups: Vec<FaGroupTrace>,
    /// How each group obtained its timing evidence.
    #[serde(default)]
    pub evidence_sources: Vec<FaEvidenceSourceTrace>,
    /// Content-addressed FA cache key for each group.
    #[serde(default)]
    pub cache_keys: Vec<String>,
    /// Pre-injection timings per group, per word (None = untimed).
    pub pre_injection_timings: Vec<Vec<Option<TimingTrace>>>,
    /// Post-injection timings after post-processing fixes.
    pub post_injection_timings: Vec<Vec<Option<TimingTrace>>>,
    /// Typed decisions made after inference, including timing removal and
    /// clamping that cannot be reconstructed from final CHAT alone.
    #[serde(default)]
    pub decisions: Vec<FaDecisionTrace>,
    /// Structured numeric facts for every monotonicity decision. This is
    /// deliberately separate from the human-readable decision reason.
    #[serde(default)]
    pub timing_decisions: Vec<FaTimingDecisionTrace>,
    /// Gap-healing policy, as the `Debug` spelling of `WordGapHealing`
    /// (`"Heal"` / `"PreserveMeasured"`). A string because this trace is a
    /// serialization boundary shared with the dashboard.
    pub gap_healing: String,
    /// Validation violations detected (e.g. E362, E704).
    pub violations: Vec<ViolationTrace>,
    /// Engine fallback events that occurred while aligning this file.
    pub fallback_events: Vec<FaFallbackEventTrace>,
}

/// One typed pipeline decision retained in a forced-alignment evidence file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct FaDecisionTrace {
    /// Index into `ChatFile.lines`, including headers.
    pub line_idx: usize,
    /// Speaker code on the affected utterance.
    pub speaker: String,
    /// Typed producer module rendered using its stable wire name.
    pub module: String,
    /// Typed strategy rendered using its stable wire name.
    pub strategy: String,
    /// Structured key/value explanation emitted by the decision point.
    pub reason: String,
    /// Whether the decision requires human review.
    pub needs_review: bool,
}

impl From<batchalign_transform::decisions::DecisionRecord> for FaDecisionTrace {
    fn from(record: batchalign_transform::decisions::DecisionRecord) -> Self {
        Self {
            line_idx: record.line_idx.raw(),
            speaker: record.speaker,
            module: record.strategy.module().as_str().to_owned(),
            strategy: record.strategy.strategy_name().to_owned(),
            reason: record.reason,
            needs_review: record.needs_review,
        }
    }
}

/// Machine-readable numeric effect of one monotonicity decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FaTimingDecisionTrace {
    /// A later utterance's start went backward in document order.
    StartRegressionStripped {
        /// Affected line index, including headers.
        line_idx: usize,
        /// Speaker on the affected utterance.
        speaker: String,
        /// Measured start that regressed.
        start_ms: u64,
        /// Greatest preceding start in document order.
        previous_start_ms: u64,
        /// Line that supplied `previous_start_ms`.
        previous_line_idx: usize,
        /// Speaker on the preceding line.
        previous_speaker: String,
    },
    /// An earlier utterance could not be end-clamped without becoming empty.
    ZeroDurationClampStripped {
        /// Affected line index, including headers.
        line_idx: usize,
        /// Speaker on the affected utterance.
        speaker: String,
        /// Original start of the earlier utterance.
        start_ms: u64,
        /// Original end of the earlier utterance.
        original_end_ms: u64,
        /// Following start that made a positive clamp impossible.
        next_start_ms: u64,
        /// Following line that supplied `next_start_ms`.
        next_line_idx: usize,
        /// Speaker on the following line.
        next_speaker: String,
    },
    /// An earlier utterance's end was clamped to the next start.
    EndClamped {
        /// Affected line index, including headers.
        line_idx: usize,
        /// Speaker on the affected utterance.
        speaker: String,
        /// End before clamping.
        original_end_ms: u64,
        /// Following start used as the new end.
        clamped_to_ms: u64,
        /// Following line that supplied the clamp boundary.
        next_line_idx: usize,
        /// Speaker on the following line.
        next_speaker: String,
    },
}

impl From<crate::chat_ops::fa::MonotonicityEffect> for FaTimingDecisionTrace {
    fn from(effect: crate::chat_ops::fa::MonotonicityEffect) -> Self {
        use crate::chat_ops::fa::MonotonicityEffect;
        match effect {
            MonotonicityEffect::StartRegressionStripped {
                line_idx,
                speaker,
                start_ms,
                previous_start_ms,
                previous_line_idx,
                previous_speaker,
            } => Self::StartRegressionStripped {
                line_idx,
                speaker,
                start_ms,
                previous_start_ms,
                previous_line_idx,
                previous_speaker,
            },
            MonotonicityEffect::ZeroDurationClampStripped {
                line_idx,
                speaker,
                start_ms,
                original_end_ms,
                next_start_ms,
                next_line_idx,
                next_speaker,
            } => Self::ZeroDurationClampStripped {
                line_idx,
                speaker,
                start_ms,
                original_end_ms,
                next_start_ms,
                next_line_idx,
                next_speaker,
            },
            MonotonicityEffect::EndClamped {
                line_idx,
                speaker,
                original_end_ms,
                clamped_to_ms,
                next_line_idx,
                next_speaker,
            } => Self::EndClamped {
                line_idx,
                speaker,
                original_end_ms,
                clamped_to_ms,
                next_line_idx,
                next_speaker,
            },
        }
    }
}

/// A single FA group (time-windowed batch of utterances).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct FaGroupTrace {
    /// Audio start time in ms.
    pub audio_start_ms: DurationMs,
    /// Audio end time in ms.
    pub audio_end_ms: DurationMs,
    /// Utterance indices covered by this group.
    pub utterance_indices: Vec<usize>,
    /// Words in this group.
    pub words: Vec<String>,
    /// Stable AST-derived identity corresponding one-to-one with `words`.
    #[serde(default)]
    pub word_ids: Vec<String>,
}

/// How one group's word timing evidence was obtained.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(rename_all = "snake_case")]
pub enum FaEvidenceSourceTrace {
    /// Reprojected from an existing healthy `%wor` tier; original provenance
    /// is unavailable because CHAT bullets cannot store it.
    WorReuse,
    /// Replayed from the content-addressed FA cache.
    Cache,
    /// Reparsed locally from an immutable cached worker response.
    RawEvidenceReplay,
    /// Produced by a worker call during this run.
    Inference,
}

/// One forced-alignment engine fallback that occurred for a single group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct FaFallbackEventTrace {
    /// Group index within the file-local FA grouping.
    pub group_index: usize,
    /// Engine originally requested by the Rust control plane.
    pub from_engine: String,
    /// Engine actually used for the retry.
    pub to_engine: String,
    /// Human-readable reason why the fallback was triggered.
    pub reason: String,
    /// Audio start time of the affected group in ms.
    pub audio_start_ms: DurationMs,
    /// Audio end time of the affected group in ms.
    pub audio_end_ms: DurationMs,
}

/// Start/end timing for a single word.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct TimingTrace {
    /// Start time in ms.
    pub start_ms: i64,
    /// End time in ms.
    pub end_ms: i64,
    /// Model-emitted alignment score, quantized to millionths. This is not a
    /// calibrated probability of boundary accuracy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_score_millionths: Option<u32>,
    /// Full provenance chain for the start boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_origin: Option<OriginTrace>,
    /// Full provenance chain for the end boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_origin: Option<OriginTrace>,
}

impl TimingTrace {
    /// Preserve all timing evidence that survives in `WordTiming` before CHAT
    /// serialization lowers it to two integers.
    pub fn from_word_timing(timing: &crate::chat_ops::fa::WordTiming) -> Self {
        Self {
            start_ms: timing.start_ms as i64,
            end_ms: timing.end_ms as i64,
            model_score_millionths: timing.model_score().map(|score| score.millionths()),
            start_origin: Some(OriginTrace::from(timing.start_origin())),
            end_origin: Some(OriginTrace::from(timing.end_origin())),
        }
    }
}

/// Serializable mirror of the complete forced-alignment origin chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OriginTrace {
    /// A model measured this boundary.
    EngineMeasured {
        /// Engine identity carried by the measurement.
        engine: String,
    },
    /// Timing was already present in the input CHAT.
    TranscriptBullet,
    /// A boundary was reduced to a typed bound.
    ClampedTo {
        /// Typed bound applied to the value.
        bound: String,
        /// Provenance before clamping.
        was: Box<OriginTrace>,
        /// Boundary before clamping.
        original_ms: u64,
        /// Distance beyond the bound.
        overshoot_ms: u64,
    },
    /// A gap was distributed by word count.
    EstimatedFromWordCount {
        /// Gap distributed across the run.
        gap_ms: u64,
        /// Words preceding this position in the run.
        words_before: usize,
        /// Total words sharing the gap.
        words_total: usize,
    },
    /// A boundary was moved to restore ordering.
    RepairedForOrder {
        /// Provenance before the ordering repair.
        was: Box<OriginTrace>,
        /// Boundary before the repair.
        original_ms: u64,
    },
    /// A boundary was copied from a neighbor.
    InheritedFromNeighbour {
        /// Neighbor boundary that was copied.
        from_ms: u64,
    },
    /// An envelope was built over multiple measured spans.
    MergedFromParts {
        /// Number of spans covered by the envelope.
        parts: usize,
    },
    /// An onset from the next token supplied this end.
    DerivedFromNextOnset,
    /// A constant duration supplied this end.
    FallbackDuration {
        /// Constant duration supplied by the fallback.
        assumed_ms: u64,
    },
}

impl From<&Origin> for OriginTrace {
    fn from(origin: &Origin) -> Self {
        match origin {
            Origin::EngineMeasured { engine } => Self::EngineMeasured {
                engine: engine.to_string(),
            },
            Origin::TranscriptBullet => Self::TranscriptBullet,
            Origin::ClampedTo {
                bound,
                was,
                original,
                overshoot,
            } => Self::ClampedTo {
                bound: match bound {
                    ClampBound::RecordingEnd => "recording_end",
                    ClampBound::UtteranceBullet => "utterance_bullet",
                    ClampBound::NextOnset => "next_onset",
                }
                .to_owned(),
                was: Box::new(Self::from(was.as_ref())),
                original_ms: original.get(),
                overshoot_ms: overshoot.0,
            },
            Origin::EstimatedFromWordCount {
                gap,
                words_before,
                words_total,
            } => Self::EstimatedFromWordCount {
                gap_ms: gap.0,
                words_before: *words_before,
                words_total: *words_total,
            },
            Origin::RepairedForOrder { was, original } => Self::RepairedForOrder {
                was: Box::new(Self::from(was.as_ref())),
                original_ms: original.get(),
            },
            Origin::InheritedFromNeighbour { from } => Self::InheritedFromNeighbour {
                from_ms: from.get(),
            },
            Origin::MergedFromParts { parts } => Self::MergedFromParts { parts: *parts },
            Origin::DerivedFromNextOnset => Self::DerivedFromNextOnset,
            Origin::FallbackDuration { assumed } => Self::FallbackDuration {
                assumed_ms: assumed.0,
            },
        }
    }
}

/// A validation violation detected during FA.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct ViolationTrace {
    /// Error code (e.g. "E362", "E704").
    pub code: String,
    /// Human-readable description.
    pub message: String,
    /// Utterance index where the violation was found.
    pub utterance_index: Option<usize>,
}

// ---------------------------------------------------------------------------
// Retokenization
// ---------------------------------------------------------------------------

/// Retokenization trace for a single utterance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct RetokenizationTrace {
    /// Utterance index in the file (0-based).
    pub utterance_index: usize,
    /// Original CHAT words.
    pub original_words: Vec<String>,
    /// Stanza tokens after retokenization.
    pub stanza_tokens: Vec<String>,
    /// Normalized concatenation of original words.
    pub normalized_original: String,
    /// Normalized concatenation of Stanza tokens.
    pub normalized_tokens: String,
    /// Word→token index mapping: `mapping[word_idx]` = list of token indices.
    pub mapping: Vec<Vec<usize>>,
    /// Whether the fallback (length-proportional) mapping was used.
    pub used_fallback: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_ops::fa::origin::EngineId;
    use crate::chat_ops::fa::{ModelAlignmentScore, WordTiming};
    use crate::time::{FileMs, Ms};

    #[test]
    fn timing_trace_preserves_score_and_complete_origin_chain() {
        let measured = Origin::EngineMeasured {
            engine: EngineId::new("wav2vec_fa"),
        };
        let adjusted_start = Origin::ClampedTo {
            bound: ClampBound::UtteranceBullet,
            was: Box::new(measured.clone()),
            original: FileMs::new(90),
            overshoot: Ms(10),
        };
        let adjusted_end = Origin::RepairedForOrder {
            was: Box::new(measured),
            original: FileMs::new(220),
        };
        let score = ModelAlignmentScore::try_from_f64(0.812_345).expect("fixture score is valid");
        let timing = WordTiming::new(100, 200, adjusted_start, adjusted_end)
            .expect("fixture timing has positive extent")
            .with_model_score(score);

        let trace = TimingTrace::from_word_timing(&timing);
        let json = serde_json::to_value(trace).expect("timing trace should serialize");

        assert_eq!(json["start_ms"], 100);
        assert_eq!(json["end_ms"], 200);
        assert_eq!(json["model_score_millionths"], 812_345);
        assert_eq!(json["start_origin"]["kind"], "clamped_to");
        assert_eq!(json["start_origin"]["bound"], "utterance_bullet");
        assert_eq!(json["start_origin"]["was"]["kind"], "engine_measured");
        assert_eq!(json["start_origin"]["was"]["engine"], "wav2vec_fa");
        assert_eq!(json["end_origin"]["kind"], "repaired_for_order");
        assert_eq!(json["end_origin"]["was"]["kind"], "engine_measured");
    }
}
