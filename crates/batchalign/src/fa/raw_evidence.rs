//! Immutable model-returned forced-alignment evidence.

use serde::{Deserialize, Serialize};

use crate::chat_ops::CacheKey;
use crate::types::engines::FaEngineName;
use crate::types::worker_v2::{ExecuteOutcomeRef, ExecuteResponseV2, TaskResultV2};

const FA_RAW_EVIDENCE_SCHEMA_VERSION: u8 = 1;

/// Expected main-tier word cardinality attached to one FA request.
///
/// This is a separate type because a raw response's payload cardinality is
/// meaningful only relative to the exact request that produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct ExpectedFaWords(usize);

impl ExpectedFaWords {
    pub(super) fn new(value: usize) -> Self {
        Self(value)
    }

    pub(super) fn get(self) -> usize {
        self.0
    }
}

/// How the effective response relates to the engine selected by the caller.
pub(super) enum FaEvidenceRoute<'a> {
    /// The selected engine produced the response directly.
    Direct,
    /// A retry engine produced the response after a named primary failure.
    Fallback { reason: &'a str },
}

/// Raw worker response admitted against the request facts needed for replay.
///
/// Fields are private and this type does not implement `Deserialize`:
/// persisted JSON must pass through [`FaRawEvidence::decode`], which rechecks
/// engine identity and cardinality before returning replayable evidence.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(super) struct FaRawEvidence {
    schema_version: u8,
    requested_engine: FaEngineName,
    effective_engine: FaEngineName,
    expected_words: ExpectedFaWords,
    cache_key: CacheKey,
    fallback_reason: Option<String>,
    response: ExecuteResponseV2,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFaRawEvidence {
    schema_version: u8,
    requested_engine: FaEngineName,
    effective_engine: FaEngineName,
    expected_words: ExpectedFaWords,
    cache_key: CacheKey,
    fallback_reason: Option<String>,
    response: ExecuteResponseV2,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum FaRawEvidenceError {
    #[error("forced-alignment raw evidence JSON is invalid: {0}")]
    InvalidJson(#[from] serde_json::Error),
    #[error("unsupported forced-alignment raw evidence schema version {0}")]
    SchemaVersion(u8),
    #[error(
        "cached forced-alignment evidence requested {cached:?}, not current engine {current:?}"
    )]
    RequestedEngineDrift {
        cached: FaEngineName,
        current: FaEngineName,
    },
    #[error(
        "cached forced-alignment evidence expected {cached} words, not current cardinality {current}"
    )]
    ExpectedWordsDrift { cached: usize, current: usize },
    #[error("cached forced-alignment evidence belongs to a different semantic cache key")]
    CacheKeyDrift,
    #[error(
        "cached forced-alignment evidence claims effective engine {cached:?}, but its payload proves {proven:?}"
    )]
    EffectiveEngineDrift {
        cached: FaEngineName,
        proven: FaEngineName,
    },
    #[error("forced-alignment worker returned an error: {code:?}: {message}")]
    WorkerFailure {
        code: crate::types::worker_v2::ProtocolErrorCodeV2,
        message: String,
    },
    #[error("forced-alignment worker returned the wrong task-result kind")]
    UnexpectedResult,
    #[error("indexed forced-alignment evidence cannot satisfy requested engine {0:?}")]
    IndexedEngineMismatch(FaEngineName),
    #[error(
        "indexed forced-alignment evidence contains {actual} timings for {expected} request words"
    )]
    WordCardinality { expected: usize, actual: usize },
    #[error("direct forced-alignment evidence was produced by a different engine")]
    UnexpectedDirectEngine,
    #[error("forced-alignment fallback did not change the effective engine")]
    IneffectiveFallback,
    #[error("forced-alignment fallback reason must not be empty")]
    EmptyFallbackReason,
}

impl FaRawEvidence {
    /// Admit a newly returned worker response against its request facts.
    pub(super) fn admit_requested(
        response: &ExecuteResponseV2,
        requested_engine: FaEngineName,
        expected_words: ExpectedFaWords,
        cache_key: &CacheKey,
        route: FaEvidenceRoute<'_>,
    ) -> Result<Self, FaRawEvidenceError> {
        let effective_engine = match response.read() {
            ExecuteOutcomeRef::Failed { code, message } => {
                return Err(FaRawEvidenceError::WorkerFailure {
                    code,
                    message: message.to_owned(),
                });
            }
            ExecuteOutcomeRef::Success(TaskResultV2::IndexedWordTimingResult(result)) => {
                if requested_engine == FaEngineName::Whisper {
                    return Err(FaRawEvidenceError::IndexedEngineMismatch(requested_engine));
                }
                if result.indexed_timings.len() != expected_words.get() {
                    return Err(FaRawEvidenceError::WordCardinality {
                        expected: expected_words.get(),
                        actual: result.indexed_timings.len(),
                    });
                }
                requested_engine
            }
            ExecuteOutcomeRef::Success(TaskResultV2::WhisperTokenTimingResult(_)) => {
                FaEngineName::Whisper
            }
            ExecuteOutcomeRef::Success(_) => return Err(FaRawEvidenceError::UnexpectedResult),
        };

        let fallback_reason = match route {
            FaEvidenceRoute::Direct if effective_engine == requested_engine => None,
            FaEvidenceRoute::Direct => return Err(FaRawEvidenceError::UnexpectedDirectEngine),
            FaEvidenceRoute::Fallback { reason: _ } if effective_engine == requested_engine => {
                return Err(FaRawEvidenceError::IneffectiveFallback);
            }
            FaEvidenceRoute::Fallback { reason } if reason.trim().is_empty() => {
                return Err(FaRawEvidenceError::EmptyFallbackReason);
            }
            FaEvidenceRoute::Fallback { reason } => Some(reason.to_owned()),
        };

        Ok(Self {
            schema_version: FA_RAW_EVIDENCE_SCHEMA_VERSION,
            requested_engine,
            effective_engine,
            expected_words,
            cache_key: cache_key.clone(),
            fallback_reason,
            response: response.clone(),
        })
    }

    /// Decode persisted evidence only after proving it still matches the request.
    pub(super) fn decode(
        value: serde_json::Value,
        requested_engine: FaEngineName,
        expected_words: ExpectedFaWords,
        cache_key: &CacheKey,
    ) -> Result<Self, FaRawEvidenceError> {
        let raw: RawFaRawEvidence = serde_json::from_value(value)?;
        if raw.schema_version != FA_RAW_EVIDENCE_SCHEMA_VERSION {
            return Err(FaRawEvidenceError::SchemaVersion(raw.schema_version));
        }
        if raw.requested_engine != requested_engine {
            return Err(FaRawEvidenceError::RequestedEngineDrift {
                cached: raw.requested_engine,
                current: requested_engine,
            });
        }
        if raw.expected_words != expected_words {
            return Err(FaRawEvidenceError::ExpectedWordsDrift {
                cached: raw.expected_words.get(),
                current: expected_words.get(),
            });
        }
        if &raw.cache_key != cache_key {
            return Err(FaRawEvidenceError::CacheKeyDrift);
        }

        let route = match raw.fallback_reason.as_deref() {
            Some(reason) => FaEvidenceRoute::Fallback { reason },
            None => FaEvidenceRoute::Direct,
        };
        let admitted = Self::admit_requested(
            &raw.response,
            requested_engine,
            expected_words,
            cache_key,
            route,
        )?;
        if raw.effective_engine != admitted.effective_engine {
            return Err(FaRawEvidenceError::EffectiveEngineDrift {
                cached: raw.effective_engine,
                proven: admitted.effective_engine,
            });
        }
        Ok(admitted)
    }

    pub(super) fn requested_engine(&self) -> FaEngineName {
        self.requested_engine
    }

    pub(super) fn effective_engine(&self) -> FaEngineName {
        self.effective_engine
    }

    #[cfg(test)]
    pub(super) fn expected_words(&self) -> ExpectedFaWords {
        self.expected_words
    }

    pub(super) fn response(&self) -> &ExecuteResponseV2 {
        &self.response
    }

    pub(super) fn fallback_reason(&self) -> Option<&str> {
        self.fallback_reason.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{DurationMs, DurationSeconds};
    use crate::types::engines::FaEngineName;
    use crate::types::worker_v2::{
        ExecuteResponseV2, IndexedWordTimingResultV2, IndexedWordTimingV2, TaskResultV2,
        WhisperTokenTimingResultV2, WhisperTokenTimingV2, WorkerRequestIdV2,
    };

    fn cache_key(label: &str) -> CacheKey {
        CacheKey::from_content(label)
    }

    #[test]
    fn admitted_raw_evidence_round_trips_without_losing_engine_or_cardinality() {
        let response = ExecuteResponseV2::success(
            WorkerRequestIdV2::from("fa-raw-1"),
            TaskResultV2::IndexedWordTimingResult(IndexedWordTimingResultV2 {
                indexed_timings: vec![Some(IndexedWordTimingV2 {
                    start_ms: DurationMs(10),
                    end_ms: DurationMs(30),
                    confidence: Some(0.75),
                })],
            }),
            DurationSeconds(0.01),
        );

        let evidence = FaRawEvidence::admit_requested(
            &response,
            FaEngineName::Wave2Vec,
            ExpectedFaWords::new(1),
            &cache_key("roundtrip"),
            FaEvidenceRoute::Direct,
        )
        .expect("valid indexed evidence");
        let encoded = serde_json::to_value(&evidence).expect("serialize evidence");
        let replayed = FaRawEvidence::decode(
            encoded,
            FaEngineName::Wave2Vec,
            ExpectedFaWords::new(1),
            &cache_key("roundtrip"),
        )
        .expect("decode evidence");

        assert_eq!(replayed.requested_engine(), FaEngineName::Wave2Vec);
        assert_eq!(replayed.effective_engine(), FaEngineName::Wave2Vec);
        assert_eq!(replayed.expected_words().get(), 1);
    }

    #[test]
    fn cached_evidence_for_a_different_requested_engine_is_refused() {
        let response = ExecuteResponseV2::success(
            WorkerRequestIdV2::from("fa-raw-2"),
            TaskResultV2::WhisperTokenTimingResult(WhisperTokenTimingResultV2 {
                tokens: vec![WhisperTokenTimingV2 {
                    text: "hello".to_owned(),
                    time_s: DurationSeconds(0.1),
                }],
            }),
            DurationSeconds(0.01),
        );
        let evidence = FaRawEvidence::admit_requested(
            &response,
            FaEngineName::Whisper,
            ExpectedFaWords::new(1),
            &cache_key("engine-drift"),
            FaEvidenceRoute::Direct,
        )
        .expect("valid token evidence");

        let error = FaRawEvidence::decode(
            serde_json::to_value(evidence).expect("serialize evidence"),
            FaEngineName::Wave2Vec,
            ExpectedFaWords::new(1),
            &cache_key("engine-drift"),
        )
        .expect_err("a key for another engine cannot admit this evidence");

        assert!(matches!(
            error,
            FaRawEvidenceError::RequestedEngineDrift { .. }
        ));
    }

    #[test]
    fn indexed_raw_evidence_with_wrong_word_cardinality_is_refused() {
        let response = ExecuteResponseV2::success(
            WorkerRequestIdV2::from("fa-raw-3"),
            TaskResultV2::IndexedWordTimingResult(IndexedWordTimingResultV2 {
                indexed_timings: vec![],
            }),
            DurationSeconds(0.01),
        );

        let error = FaRawEvidence::admit_requested(
            &response,
            FaEngineName::Wave2Vec,
            ExpectedFaWords::new(1),
            &cache_key("cardinality"),
            FaEvidenceRoute::Direct,
        )
        .expect_err("short indexed evidence must not become replayable");

        assert!(matches!(error, FaRawEvidenceError::WordCardinality { .. }));
    }

    #[test]
    fn cached_evidence_for_a_different_semantic_key_is_refused() {
        let response = ExecuteResponseV2::success(
            WorkerRequestIdV2::from("fa-raw-4"),
            TaskResultV2::IndexedWordTimingResult(IndexedWordTimingResultV2 {
                indexed_timings: vec![None],
            }),
            DurationSeconds(0.01),
        );
        let evidence = FaRawEvidence::admit_requested(
            &response,
            FaEngineName::Wave2Vec,
            ExpectedFaWords::new(1),
            &cache_key("first-group"),
            FaEvidenceRoute::Direct,
        )
        .expect("valid indexed evidence");

        let error = FaRawEvidence::decode(
            serde_json::to_value(evidence).expect("serialize evidence"),
            FaEngineName::Wave2Vec,
            ExpectedFaWords::new(1),
            &cache_key("second-group"),
        )
        .expect_err("same-cardinality evidence from another group must not replay");

        assert!(matches!(error, FaRawEvidenceError::CacheKeyDrift));
    }

    #[test]
    fn fallback_route_round_trips_with_its_reason_and_effective_engine() {
        let response = ExecuteResponseV2::success(
            WorkerRequestIdV2::from("fa-raw-fallback"),
            TaskResultV2::WhisperTokenTimingResult(WhisperTokenTimingResultV2 {
                tokens: vec![WhisperTokenTimingV2 {
                    text: "hello".to_owned(),
                    time_s: DurationSeconds(0.1),
                }],
            }),
            DurationSeconds(0.01),
        );
        let key = cache_key("fallback");
        let evidence = FaRawEvidence::admit_requested(
            &response,
            FaEngineName::Wave2Vec,
            ExpectedFaWords::new(1),
            &key,
            FaEvidenceRoute::Fallback {
                reason: "targets length is too long for CTC",
            },
        )
        .expect("valid fallback evidence");

        let replayed = FaRawEvidence::decode(
            serde_json::to_value(evidence).expect("serialize evidence"),
            FaEngineName::Wave2Vec,
            ExpectedFaWords::new(1),
            &key,
        )
        .expect("decode fallback evidence");

        assert_eq!(replayed.effective_engine(), FaEngineName::Whisper);
        assert_eq!(
            replayed.fallback_reason(),
            Some("targets length is too long for CTC")
        );
    }

    #[test]
    fn direct_route_cannot_hide_an_effective_engine_change() {
        let response = ExecuteResponseV2::success(
            WorkerRequestIdV2::from("fa-raw-unlabelled-fallback"),
            TaskResultV2::WhisperTokenTimingResult(WhisperTokenTimingResultV2 {
                tokens: Vec::new(),
            }),
            DurationSeconds(0.01),
        );

        let error = FaRawEvidence::admit_requested(
            &response,
            FaEngineName::Wave2Vec,
            ExpectedFaWords::new(1),
            &cache_key("unlabelled-fallback"),
            FaEvidenceRoute::Direct,
        )
        .expect_err("a fallback response needs an explicit fallback route");

        assert!(matches!(error, FaRawEvidenceError::UnexpectedDirectEngine));
    }
}
