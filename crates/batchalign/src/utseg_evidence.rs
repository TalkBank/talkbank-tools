//! Durable utterance-segmentation evidence for controlled experiments.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::types::worker_v2::UtsegBoundaryModelEvidenceV2;
use crate::utseg::{AdmittedUtsegPrediction, LocalUtsegDecisionReceipt};
use batchalign_transform::utseg::UtsegBatchItem;

/// Location in transcribe at which utterance segmentation ran.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UtsegEvidencePhase {
    /// Segmentation over timed ASR chunks before CHAT construction.
    PreChat,
    /// Segmentation over main-tier words after CHAT construction.
    PostChat,
}

impl UtsegEvidencePhase {
    fn filename_component(self) -> &'static str {
        match self {
            Self::PreChat => "pre_chat",
            Self::PostChat => "post_chat",
        }
    }
}

/// Complete, versioned evidence from one utterance-segmentation batch.
#[derive(Debug, Serialize)]
pub(crate) struct UtsegEvidenceTrace {
    schema_version: u8,
    phase: UtsegEvidencePhase,
    language: String,
    engine_version: String,
    items: Vec<UtsegEvidenceItem>,
}

/// One request and the admitted prediction that is safe to apply to it.
#[derive(Debug, Serialize)]
struct UtsegEvidenceItem {
    item_ordinal: usize,
    words: Vec<String>,
    text: String,
    prediction: UtsegEvidencePrediction,
}

/// Closed set of inference sources that can produce utseg assignments.
#[derive(Debug, Serialize)]
#[serde(tag = "source", rename_all = "snake_case")]
enum UtsegEvidencePrediction {
    /// TalkBank boundary model with raw and applied per-word evidence.
    BoundaryModel {
        assignments: Vec<usize>,
        evidence: UtsegBoundaryModelEvidenceV2,
        #[serde(skip_serializing_if = "Option::is_none")]
        local_decision: Option<LocalUtsegDecisionReceipt>,
    },
    /// Compatibility path whose worker did not expose evidence.
    UnobservedAssignments { assignments: Vec<usize> },
    /// Stanza constituency-tree projection.
    Constituency { assignments: Vec<usize> },
}

/// A partial batch cannot be represented as a complete experiment trace.
#[derive(Debug, thiserror::Error)]
#[error(
    "cannot construct utseg evidence trace from {request_count} requests and {prediction_count} predictions"
)]
pub(crate) struct UtsegEvidenceShapeError {
    request_count: usize,
    prediction_count: usize,
}

impl UtsegEvidenceTrace {
    /// Construct a complete trace only when every request has one admitted
    /// prediction. Admission has already established all per-word invariants.
    pub(crate) fn from_predictions(
        phase: UtsegEvidencePhase,
        language: &str,
        engine_version: &str,
        requests: &[(usize, UtsegBatchItem)],
        predictions: &[AdmittedUtsegPrediction],
    ) -> Result<Self, UtsegEvidenceShapeError> {
        if requests.len() != predictions.len() {
            return Err(UtsegEvidenceShapeError {
                request_count: requests.len(),
                prediction_count: predictions.len(),
            });
        }

        let items = requests
            .iter()
            .zip(predictions.iter())
            .map(|((item_ordinal, request), prediction)| UtsegEvidenceItem {
                item_ordinal: *item_ordinal,
                words: request.words.clone(),
                text: request.text.clone(),
                prediction: match prediction {
                    AdmittedUtsegPrediction::BoundaryModelWorkerDeclared { response, evidence } => {
                        UtsegEvidencePrediction::BoundaryModel {
                            assignments: response.assignments.clone(),
                            evidence: evidence.clone(),
                            local_decision: None,
                        }
                    }
                    AdmittedUtsegPrediction::BoundaryModelLocallyReapplied {
                        response,
                        evidence,
                        receipt,
                    } => UtsegEvidencePrediction::BoundaryModel {
                        assignments: response.assignments.clone(),
                        evidence: evidence.clone(),
                        local_decision: Some(receipt.clone()),
                    },
                    AdmittedUtsegPrediction::UnobservedAssignments { response } => {
                        UtsegEvidencePrediction::UnobservedAssignments {
                            assignments: response.assignments.clone(),
                        }
                    }
                    AdmittedUtsegPrediction::Constituency { response } => {
                        UtsegEvidencePrediction::Constituency {
                            assignments: response.assignments.clone(),
                        }
                    }
                },
            })
            .collect();

        Ok(Self {
            schema_version: 2,
            phase,
            language: language.to_owned(),
            engine_version: engine_version.to_owned(),
            items,
        })
    }
}

/// Typestate-like evidence destination: absence and enabled persistence are
/// explicit variants instead of an optional path threaded through writes.
pub(crate) enum UtsegEvidenceSink {
    /// The run did not request durable debug evidence.
    Disabled,
    /// Every requested trace must be durably written or fail the run.
    Enabled(EnabledUtsegEvidenceSink),
}

/// Validated destination for an enabled evidence run.
pub(crate) struct EnabledUtsegEvidenceSink {
    dir: PathBuf,
}

/// Observable outcome of a write request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UtsegEvidenceWriteOutcome {
    /// Evidence collection was not requested.
    Disabled,
    /// Complete evidence was durably written to this path.
    Written(PathBuf),
}

/// Failures that prevent an enabled evidence request from being durable.
#[derive(Debug, thiserror::Error)]
pub(crate) enum UtsegEvidenceWriteError {
    /// Destination directory could not be created.
    #[error("failed to create utseg evidence directory {}: {source}", path.display())]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// Complete trace could not be serialized before publication.
    #[error("failed to serialize utseg evidence: {0}")]
    Serialize(#[from] serde_json::Error),
    /// Atomic publication failed.
    #[error("failed to write utseg evidence {}: {source}", path.display())]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl UtsegEvidenceSink {
    /// Resolve optional CLI configuration into one explicit sink state.
    pub(crate) fn new(dir: Option<&Path>) -> Self {
        match dir {
            Some(dir) => Self::Enabled(EnabledUtsegEvidenceSink {
                dir: dir.to_owned(),
            }),
            None => Self::Disabled,
        }
    }

    /// Durably publish a trace when evidence collection is enabled.
    pub(crate) fn write(
        &self,
        filename: &str,
        trace: &UtsegEvidenceTrace,
    ) -> Result<UtsegEvidenceWriteOutcome, UtsegEvidenceWriteError> {
        match self {
            Self::Disabled => Ok(UtsegEvidenceWriteOutcome::Disabled),
            Self::Enabled(enabled) => enabled.write(filename, trace),
        }
    }
}

impl EnabledUtsegEvidenceSink {
    fn write(
        &self,
        filename: &str,
        trace: &UtsegEvidenceTrace,
    ) -> Result<UtsegEvidenceWriteOutcome, UtsegEvidenceWriteError> {
        std::fs::create_dir_all(&self.dir).map_err(|source| {
            UtsegEvidenceWriteError::CreateDirectory {
                path: self.dir.clone(),
                source,
            }
        })?;
        let path = self.dir.join(format!(
            "{}_{}_utseg_evidence.json",
            evidence_stem(filename),
            trace.phase.filename_component()
        ));
        let bytes = serde_json::to_vec_pretty(trace)?;
        let mut temp = tempfile::NamedTempFile::new_in(&self.dir).map_err(|source| {
            UtsegEvidenceWriteError::Write {
                path: path.clone(),
                source,
            }
        })?;
        temp.write_all(&bytes)
            .and_then(|()| temp.as_file().sync_all())
            .map_err(|source| UtsegEvidenceWriteError::Write {
                path: path.clone(),
                source,
            })?;
        let persisted = temp
            .persist(&path)
            .map_err(|error| UtsegEvidenceWriteError::Write {
                path: path.clone(),
                source: error.error,
            })?;
        persisted
            .sync_all()
            .map_err(|source| UtsegEvidenceWriteError::Write {
                path: path.clone(),
                source,
            })?;
        #[cfg(unix)]
        std::fs::File::open(&self.dir)
            .and_then(|dir| dir.sync_all())
            .map_err(|source| UtsegEvidenceWriteError::Write {
                path: path.clone(),
                source,
            })?;
        Ok(UtsegEvidenceWriteOutcome::Written(path))
    }
}

fn evidence_stem(filename: &str) -> String {
    let path = Path::new(filename);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown");
    if path.components().count() <= 1 {
        return stem.to_owned();
    }
    let digest = blake3::hash(filename.as_bytes()).to_hex();
    format!("{stem}-{}", &digest[..12])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::worker_v2::{
        UtsegAdjacencyPolicyRevisionV2, UtsegBoundaryModelEvidenceV2, UtsegNormalizationRevisionV2,
        UtsegWordBoundaryEvidenceV2,
    };
    use crate::utseg::AdmittedUtsegPrediction;
    use batchalign_transform::utseg::{UtsegBatchItem, UtsegResponse};

    fn request() -> UtsegBatchItem {
        UtsegBatchItem {
            words: vec!["hello".to_owned(), "there".to_owned()],
            text: "hello there".to_owned(),
        }
    }

    fn prediction() -> AdmittedUtsegPrediction {
        AdmittedUtsegPrediction::BoundaryModelWorkerDeclared {
            response: UtsegResponse {
                assignments: vec![0, 1],
            },
            evidence: UtsegBoundaryModelEvidenceV2 {
                model_id: "talkbank/utterance-boundary".to_owned(),
                model_revision: Some("revision-1".to_owned()),
                normalization_revision: UtsegNormalizationRevisionV2::LowerStripAsciiPunctuationV1,
                adjacency_policy_revision:
                    UtsegAdjacencyPolicyRevisionV2::SuppressEarlierAdjacentNonordinaryV1,
                word_evidence: vec![
                    UtsegWordBoundaryEvidenceV2::NormalizationOmission,
                    UtsegWordBoundaryEvidenceV2::ModelShortCircuit,
                ],
            },
        }
    }

    #[test]
    fn trace_keeps_request_assignments_source_and_model_provenance_together() {
        let trace = UtsegEvidenceTrace::from_predictions(
            UtsegEvidencePhase::PreChat,
            "eng",
            "engine-test-1",
            &[(0, request())],
            &[prediction()],
        )
        .expect("parallel admitted predictions should form a trace");

        let value = serde_json::to_value(trace).expect("serialize evidence trace");
        assert_eq!(value["schema_version"], 2);
        assert_eq!(value["phase"], "pre_chat");
        assert_eq!(value["language"], "eng");
        assert_eq!(value["engine_version"], "engine-test-1");
        assert_eq!(value["items"][0]["words"][1], "there");
        assert_eq!(value["items"][0]["text"], "hello there");
        assert_eq!(value["items"][0]["prediction"]["source"], "boundary_model");
        assert_eq!(value["items"][0]["prediction"]["assignments"][1], 1);
        assert_eq!(
            value["items"][0]["prediction"]["evidence"]["model_revision"],
            "revision-1"
        );
    }

    #[test]
    fn trace_refuses_to_zip_different_request_and_prediction_counts() {
        let error = UtsegEvidenceTrace::from_predictions(
            UtsegEvidencePhase::PostChat,
            "eng",
            "engine-test-1",
            &[(0, request())],
            &[],
        )
        .expect_err("a partial trace must not be constructible");

        assert!(error.to_string().contains("1 requests"));
        assert!(error.to_string().contains("0 predictions"));
    }

    #[test]
    fn enabled_sink_writes_a_versioned_artifact_atomically() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sink = UtsegEvidenceSink::new(Some(dir.path()));
        let trace = UtsegEvidenceTrace::from_predictions(
            UtsegEvidencePhase::PreChat,
            "eng",
            "engine-test-1",
            &[(0, request())],
            &[prediction()],
        )
        .expect("trace");
        let expected = serde_json::to_value(&trace).expect("serialize expected trace");

        let outcome = sink
            .write("sample.wav", &trace)
            .expect("enabled evidence request should be durable");
        let UtsegEvidenceWriteOutcome::Written(path) = outcome else {
            panic!("enabled sink should write");
        };
        assert_eq!(path, dir.path().join("sample_pre_chat_utseg_evidence.json"));
        let value: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("read persisted trace"))
                .expect("parse persisted trace");
        assert_eq!(value, expected);
    }
}
