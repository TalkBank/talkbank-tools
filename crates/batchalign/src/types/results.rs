//! Structured result types for server-side orchestrators.
//!
//! Each orchestrator returns a rich result type that includes both the
//! serialized CHAT output and any intermediate data produced during
//! processing.  The dispatch layer decides what to write to disk vs.
//! what to store in the trace cache.

use crate::chat_ops::fa::WordGapHealing;
use crate::chat_ops::morphosyntax_ops::RetokenizationInfo;
use batchalign_transform::asr_postprocess::AsrPipelineSnapshot;

use super::traces::{
    AsrPipelineTrace, AsrTokenTrace, FaDecisionTrace, FaEvidenceSourceTrace, FaFallbackEventTrace,
    FaGroupTrace, FaTimelineTrace, FaTimingDecisionTrace, RetokenizationTrace, TimedWordTrace,
    TimingTrace, UtteranceTrace, ViolationTrace, WordTrace,
};
use crate::api::DurationSeconds;

// ---------------------------------------------------------------------------
// Forced alignment
// ---------------------------------------------------------------------------

/// Structured result from the internal `crate::fa::process_fa` pipeline.
pub struct FaResult {
    /// Serialized CHAT text with timings injected.
    pub chat_text: String,
    /// Per-group evidence whose fields cannot become cardinality-misaligned.
    pub(crate) group_evidence: Vec<FaGroupEvidence>,
    /// Selected forced-alignment engine.
    pub engine: String,
    /// Build/model revision used for cache identity.
    pub engine_version: String,
    /// Decisions made while injecting and enforcing final timing invariants.
    pub decisions: Vec<FaDecisionTrace>,
    /// Numeric monotonicity effects corresponding to the generic decisions.
    pub timing_decisions: Vec<FaTimingDecisionTrace>,
    /// Gap-healing policy used for this run.
    pub gap_healing: WordGapHealing,
    /// Post-validation violations.
    pub violations: Vec<ViolationTrace>,
    /// Engine fallback events captured during worker inference.
    pub fallback_events: Vec<FaFallbackEventTrace>,
}

/// One FA group inseparably paired with the evidence that produced its timing.
#[derive(Debug)]
pub(crate) struct FaGroupEvidence {
    pub(crate) group: FaGroupTrace,
    pub(crate) source: FaEvidenceSourceTrace,
    pub(crate) cache_key: String,
    pub(crate) pre_injection_timings: Vec<Option<TimingTrace>>,
}

impl FaResult {
    /// Construct a result for a legitimate no-group path (dummy, NoAlign,
    /// complete `%wor` reuse, or no alignable words).
    pub fn without_groups(
        chat_text: String,
        gap_healing: WordGapHealing,
        engine: &str,
        engine_version: &str,
    ) -> Self {
        Self {
            chat_text,
            group_evidence: Vec::new(),
            engine: engine.to_owned(),
            engine_version: engine_version.to_owned(),
            decisions: Vec::new(),
            timing_decisions: Vec::new(),
            gap_healing,
            violations: Vec::new(),
            fallback_events: Vec::new(),
        }
    }

    /// Attach the exact decision set after it has passed through CHAT
    /// projection, for a legitimate path with no fresh FA groups.
    pub fn with_written_decisions(
        mut self,
        written: crate::chat_ops::fa::WrittenFaDecisions,
    ) -> Self {
        let (records, effects) = written.into_evidence();
        self.decisions = records.into_iter().map(Into::into).collect();
        self.timing_decisions = effects.into_iter().map(Into::into).collect();
        self
    }

    /// Convert into a [`FaTimelineTrace`] for dashboard visualization.
    pub fn into_timeline_trace(self) -> FaTimelineTrace {
        let mut groups = Vec::with_capacity(self.group_evidence.len());
        let mut evidence_sources = Vec::with_capacity(self.group_evidence.len());
        let mut cache_keys = Vec::with_capacity(self.group_evidence.len());
        let mut pre_injection_timings = Vec::with_capacity(self.group_evidence.len());
        for evidence in self.group_evidence {
            groups.push(evidence.group);
            evidence_sources.push(evidence.source);
            cache_keys.push(evidence.cache_key);
            pre_injection_timings.push(evidence.pre_injection_timings);
        }
        FaTimelineTrace {
            evidence_schema_version: crate::types::traces::CURRENT_FA_EVIDENCE_SCHEMA_VERSION,
            engine: self.engine,
            engine_version: self.engine_version,
            groups,
            evidence_sources,
            cache_keys,
            pre_injection_timings,
            post_injection_timings: Vec::new(), // TODO Phase 4
            decisions: self.decisions,
            timing_decisions: self.timing_decisions,
            gap_healing: format!("{:?}", self.gap_healing),
            violations: self.violations,
            fallback_events: self.fallback_events,
        }
    }
}

// ---------------------------------------------------------------------------
// ASR pipeline trace conversion
// ---------------------------------------------------------------------------

/// Lossy conversion from the chat-ops-side per-stage snapshot to the
/// dashboard-facing `AsrPipelineTrace`.
///
/// Drops timing and structural detail not surfaced in the trace shape
/// (e.g. `AsrWord::kind`). The trace shape is the dashboard contract;
/// the snapshot is the wire-protocol-free internal capture.
pub fn snapshot_into_pipeline_trace(snapshot: AsrPipelineSnapshot) -> AsrPipelineTrace {
    AsrPipelineTrace {
        raw_tokens: snapshot
            .raw_elements
            .iter()
            .map(|e| AsrTokenTrace {
                value: e.value.as_str().to_owned(),
                ts: DurationSeconds(e.ts.as_f64()),
                end_ts: DurationSeconds(e.end_ts.as_f64()),
                token_type: format!("{:?}", e.kind).to_lowercase(),
            })
            .collect(),
        after_compound_merge: snapshot
            .after_compound_merge
            .iter()
            .map(|e| WordTrace {
                text: e.value.as_str().to_owned(),
            })
            .collect(),
        after_timing_extract: snapshot
            .after_timing_extract
            .iter()
            .map(asr_word_to_timed_trace)
            .collect(),
        after_multiword_split: snapshot
            .after_multiword_split
            .iter()
            .map(asr_word_to_timed_trace)
            .collect(),
        after_number_expand: snapshot
            .after_number_expand
            .iter()
            .map(asr_word_to_timed_trace)
            .collect(),
        after_cantonese_norm: snapshot
            .after_cantonese_norm
            .map(|words| words.iter().map(asr_word_to_timed_trace).collect()),
        after_long_turn_split: snapshot
            .after_long_turn_split
            .iter()
            .map(|chunk| chunk.iter().map(asr_word_to_timed_trace).collect())
            .collect(),
        final_utterances: snapshot
            .final_utterances
            .iter()
            .map(|u| UtteranceTrace {
                speaker: u.speaker.as_usize(),
                words: u.words.iter().map(asr_word_to_timed_trace).collect(),
            })
            .collect(),
    }
}

fn asr_word_to_timed_trace(w: &batchalign_transform::asr_postprocess::AsrWord) -> TimedWordTrace {
    TimedWordTrace {
        text: w.text.as_str().to_owned(),
        start_ms: w.start_ms,
        end_ms: w.end_ms,
    }
}

// ---------------------------------------------------------------------------
// Morphosyntax
// ---------------------------------------------------------------------------

/// Structured result from a single-file morphosyntax run.
pub struct MorphosyntaxResult {
    /// Serialized CHAT text with %mor/%gra injected.
    pub chat_text: String,
    /// Retokenization mappings (empty when retokenization is off).
    pub retokenizations: Vec<RetokenizationInfo>,
}

impl MorphosyntaxResult {
    /// Convert retokenization info into dashboard trace format.
    pub fn into_retokenization_traces(self) -> Vec<RetokenizationTrace> {
        self.retokenizations
            .into_iter()
            .map(|info| RetokenizationTrace {
                utterance_index: info.utterance_ordinal,
                original_words: info.original_words,
                stanza_tokens: info.stanza_tokens,
                normalized_original: String::new(), // not captured at this level
                normalized_tokens: String::new(),
                mapping: info.mapping,
                used_fallback: info.used_fallback,
            })
            .collect()
    }
}
