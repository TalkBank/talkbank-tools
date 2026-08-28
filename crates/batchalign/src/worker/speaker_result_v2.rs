//! Rust-side adapters for live worker-protocol V2 speaker results.
//!
//! Speaker diarization is still a raw-model concern at the worker boundary.
//! This adapter keeps Rust in charge of interpreting the typed response shape
//! instead of letting downstream callers pattern-match on generic JSON.

use serde::Deserialize;

use crate::api::DurationMs;
use crate::types::worker_v2::{
    ExecuteResponseV2, SpeakerInferenceEvidenceV2, SpeakerResultV2, SpeakerSegmentV2, TaskResultV2,
};
use crate::worker::execute_result_v2::require_success_result;

/// Parse one V2 speaker execute response into the typed segment list.
pub fn parse_speaker_result_v2(response: &ExecuteResponseV2) -> Result<&SpeakerResultV2, String> {
    let result = require_success_result(response, "speaker")?;

    match result {
        TaskResultV2::SpeakerResult(result) => Ok(result),
        TaskResultV2::MorphosyntaxResult(_) => {
            Err("worker protocol V2 speaker response returned morphosyntax data".into())
        }
        TaskResultV2::UtsegResult(_) => {
            Err("worker protocol V2 speaker response returned utterance-segmentation data".into())
        }
        TaskResultV2::WhisperChunkResult(_) => {
            Err("worker protocol V2 speaker response returned ASR chunk data".into())
        }
        TaskResultV2::MonologueAsrResult(_) => {
            Err("worker protocol V2 speaker response returned monologue ASR data".into())
        }
        TaskResultV2::WhisperTokenTimingResult(_) => {
            Err("worker protocol V2 speaker response returned forced-alignment token data".into())
        }
        TaskResultV2::IndexedWordTimingResult(_) => {
            Err("worker protocol V2 speaker response returned indexed timing data".into())
        }
        TaskResultV2::TranslationResult(_) => {
            Err("worker protocol V2 speaker response returned translation data".into())
        }
        TaskResultV2::CorefResult(_) => {
            Err("worker protocol V2 speaker response returned coreference data".into())
        }
        TaskResultV2::OpensmileResult(_) => {
            Err("worker protocol V2 speaker response returned openSMILE feature data".into())
        }
        TaskResultV2::AvqiResult(_) => {
            Err("worker protocol V2 speaker response returned AVQI feature data".into())
        }
    }
}

#[derive(Debug, Deserialize)]
struct PyannoteAiSegment {
    start: f64,
    end: f64,
    speaker: String,
}

/// Normalize backend-specific evidence into ordered millisecond speaker spans.
///
/// Keeping this projection in Rust means completed paid provider output can be
/// stored and replayed under a newer normalization revision without another
/// service call.
pub(crate) fn normalize_speaker_evidence_v2(
    evidence: &SpeakerInferenceEvidenceV2,
) -> Result<Vec<SpeakerSegmentV2>, String> {
    let mut segments = match evidence {
        SpeakerInferenceEvidenceV2::PyannoteAi {
            job_id,
            output,
            warning: _,
        } => {
            if job_id.as_str().trim().is_empty() {
                return Err("pyannoteAI evidence has an empty provider job id".to_owned());
            }
            let raw_segments = output
                .get("exclusiveDiarization")
                .or_else(|| output.get("diarization"))
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| {
                    "pyannoteAI completed job has no diarization segment array".to_owned()
                })?;
            raw_segments
                .iter()
                .enumerate()
                .map(|(index, raw)| {
                    let parsed: PyannoteAiSegment =
                        serde_json::from_value(raw.clone()).map_err(|error| {
                            format!("pyannoteAI segment {index} has invalid data: {error}")
                        })?;
                    if parsed.speaker.trim().is_empty() {
                        return Err(format!(
                            "pyannoteAI segment {index} has an empty speaker label"
                        ));
                    }
                    let start_ms = seconds_to_ms(parsed.start, index, "start")?;
                    let end_ms = seconds_to_ms(parsed.end, index, "end")?;
                    if end_ms < start_ms {
                        return Err(format!(
                            "pyannoteAI segment {index} has an inverted interval"
                        ));
                    }
                    Ok(SpeakerSegmentV2 {
                        start_ms,
                        end_ms,
                        speaker: parsed.speaker,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?
        }
        SpeakerInferenceEvidenceV2::Pyannote { segments }
        | SpeakerInferenceEvidenceV2::Nemo { segments } => segments.clone(),
    };
    segments.sort_by(|left, right| {
        (left.start_ms.0, left.end_ms.0, left.speaker.as_str()).cmp(&(
            right.start_ms.0,
            right.end_ms.0,
            right.speaker.as_str(),
        ))
    });
    Ok(segments)
}

fn seconds_to_ms(value: f64, index: usize, field: &str) -> Result<DurationMs, String> {
    if !value.is_finite() || value < 0.0 || value > (u64::MAX as f64 / 1000.0) {
        return Err(format!(
            "pyannoteAI segment {index} has invalid {field} seconds"
        ));
    }
    Ok(DurationMs((value * 1000.0).round() as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DurationSeconds;
    use crate::types::worker_v2::{
        ExecuteResponseV2, SpeakerInferenceEvidenceV2, SpeakerProviderJobIdV2, SpeakerResultV2,
        SpeakerSegmentV2, WorkerRequestIdV2,
    };

    #[test]
    fn parses_speaker_segments_from_typed_v2_result() {
        let response = ExecuteResponseV2::success(
            WorkerRequestIdV2::from("req-speaker-v2-1"),
            TaskResultV2::SpeakerResult(SpeakerResultV2 {
                evidence: SpeakerInferenceEvidenceV2::Pyannote {
                    segments: vec![SpeakerSegmentV2 {
                        start_ms: DurationMs(0),
                        end_ms: DurationMs(900),
                        speaker: "SPEAKER_1".into(),
                    }],
                },
            }),
            DurationSeconds(0.01),
        );

        let parsed = parse_speaker_result_v2(&response).expect("speaker result should parse");
        let segments = normalize_speaker_evidence_v2(&parsed.evidence)
            .expect("speaker evidence should normalize");
        assert_eq!(segments[0].speaker, "SPEAKER_1");
        assert_eq!(segments[0].end_ms, DurationMs(900));
    }

    #[test]
    fn normalizes_completed_pyannote_ai_evidence_without_python() {
        let evidence = SpeakerInferenceEvidenceV2::PyannoteAi {
            job_id: SpeakerProviderJobIdV2::from("job-1"),
            output: serde_json::from_value(serde_json::json!({
                "exclusiveDiarization": [
                    {"start": 1.25, "end": 1.5, "speaker": "SPEAKER_01"},
                    {"start": 0.0, "end": 0.75, "speaker": "SPEAKER_00"}
                ]
            }))
            .expect("provider output object"),
            warning: None,
        };

        let segments = normalize_speaker_evidence_v2(&evidence).expect("normalize");
        assert_eq!(segments[0].start_ms, DurationMs(0));
        assert_eq!(segments[1].end_ms, DurationMs(1500));
    }

    #[test]
    fn rejects_non_speaker_v2_payloads() {
        let response = ExecuteResponseV2::success(
            WorkerRequestIdV2::from("req-speaker-v2-2"),
            TaskResultV2::TranslationResult(crate::types::worker_v2::TranslationResultV2 {
                items: vec![crate::types::worker_v2::TranslationItemResultV2 {
                    raw_translation: Some("hola".into()),
                    error: None,
                }],
            }),
            DurationSeconds(0.01),
        );

        let error =
            parse_speaker_result_v2(&response).expect_err("translation result should be rejected");
        assert!(error.contains("translation data"));
    }
}
