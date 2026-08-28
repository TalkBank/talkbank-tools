//! Durable raw Rev.AI transcript evidence and paid-call typestate.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::api::{EngineVersion, LanguageCode3, LanguageSpec, NumSpeakers};
use crate::cache::{CacheBackend, CacheError, InferenceLease, UtteranceCache};
use crate::chat_ops::{CacheKey, CacheTaskName};
use crate::error::ServerError;
use crate::params::CachePolicy;

use super::Transcript;

const REV_ASR_EVIDENCE_SCHEMA_VERSION: u32 = 1;
const REV_ASR_REQUEST_POLICY_REVISION: &str = "langid-skip-itn-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct RevProviderMediaDigest(String);

impl RevProviderMediaDigest {
    async fn from_path(path: &Path) -> Result<Self, RevAsrEvidenceCacheError> {
        let mut file = tokio::fs::File::open(path).await?;
        let mut hasher = blake3::Hasher::new();
        let mut buffer = vec![0_u8; 1024 * 1024];
        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        Ok(Self(hasher.finalize().to_hex().to_string()))
    }
}

#[derive(Debug, Serialize)]
struct RevAsrEvidenceKeyMaterial<'a> {
    domain: &'static str,
    schema_version: u32,
    provider_media_blake3: &'a RevProviderMediaDigest,
    requested_language: String,
    expected_speakers: u32,
    request_policy_revision: &'static str,
    model_revision: &'a str,
}

/// Provider revision identity distinct from worker ASR engine versions.
#[derive(Debug, Clone)]
pub(crate) struct RevAsrModelRevision(EngineVersion);

impl RevAsrModelRevision {
    pub(crate) fn current() -> Self {
        Self(EngineVersion::from("revai:asynchronous-transcript-v1"))
    }

    fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RevAsrEvidenceRequest {
    cache_key: CacheKey,
    model_revision: RevAsrModelRevision,
}

impl RevAsrEvidenceRequest {
    pub(crate) async fn from_audio(
        audio_path: &Path,
        requested_language: &LanguageSpec,
        expected_speakers: NumSpeakers,
        model_revision: &RevAsrModelRevision,
    ) -> Result<Self, RevAsrEvidenceCacheError> {
        if requested_language.is_per_file() {
            return Err(RevAsrEvidenceCacheError::InvalidRequest(
                "Rev.AI transcribe evidence cannot use per-file language routing".to_owned(),
            ));
        }
        let digest = RevProviderMediaDigest::from_path(audio_path).await?;
        let material = RevAsrEvidenceKeyMaterial {
            domain: "rev_asr_evidence",
            schema_version: REV_ASR_EVIDENCE_SCHEMA_VERSION,
            provider_media_blake3: &digest,
            requested_language: requested_language.to_string(),
            expected_speakers: expected_speakers.0,
            request_policy_revision: REV_ASR_REQUEST_POLICY_REVISION,
            model_revision: model_revision.as_str(),
        };
        let canonical = serde_json::to_string(&material)?;
        Ok(Self {
            cache_key: CacheKey::from_content(&canonical),
            model_revision: model_revision.clone(),
        })
    }

    pub(crate) fn cache_key(&self) -> &CacheKey {
        &self.cache_key
    }

    async fn lookup(
        &self,
        cache: &UtteranceCache,
        policy: CachePolicy,
    ) -> Result<RevAsrEvidenceLookup, RevAsrEvidenceCacheError> {
        let lease = InferenceLease::acquire(&self.cache_key, cache).await;
        if policy.should_skip() && !lease.observed_commit_while_waiting() {
            return Ok(RevAsrEvidenceLookup::Miss(RevAsrEvidenceMiss {
                request: self.clone(),
                reason: RevAsrEvidenceMissReason::ForcedRefresh,
                _lease: lease,
            }));
        }
        let stored = cache
            .get(
                self.cache_key.as_str(),
                CacheTaskName::RevAsrEvidence.as_str(),
                self.model_revision.as_str(),
            )
            .await?;
        let Some(stored) = stored else {
            return Ok(RevAsrEvidenceLookup::Miss(RevAsrEvidenceMiss {
                request: self.clone(),
                reason: RevAsrEvidenceMissReason::NotFound,
                _lease: lease,
            }));
        };
        let envelope: StoredRevAsrEvidence = serde_json::from_value(stored)
            .map_err(|error| RevAsrEvidenceCacheError::InvalidEvidence(error.to_string()))?;
        envelope.validate_for(self)?;
        Ok(RevAsrEvidenceLookup::Hit(envelope.evidence))
    }

    #[cfg(test)]
    async fn store_unchecked_for_test(
        &self,
        cache: &UtteranceCache,
        value: serde_json::Value,
    ) -> Result<(), RevAsrEvidenceCacheError> {
        cache
            .put(
                self.cache_key.as_str(),
                CacheTaskName::RevAsrEvidence.as_str(),
                self.model_revision.as_str(),
                env!("CARGO_PKG_VERSION"),
                &value,
            )
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CompletedRevAsrEvidence {
    pub(crate) transcript: Transcript,
    pub(crate) resolved_language: LanguageCode3,
}

fn validate_evidence(evidence: &CompletedRevAsrEvidence) -> Result<(), RevAsrEvidenceCacheError> {
    for (monologue_index, monologue) in evidence.transcript.monologues.iter().enumerate() {
        if monologue.speaker < 0 {
            return Err(RevAsrEvidenceCacheError::InvalidEvidence(format!(
                "monologue {monologue_index} has negative speaker index"
            )));
        }
        for (element_index, element) in monologue.elements.iter().enumerate() {
            for (label, value) in [("start", element.ts), ("end", element.end_ts)] {
                if let Some(value) = value
                    && (!value.is_finite() || value < 0.0)
                {
                    return Err(RevAsrEvidenceCacheError::InvalidEvidence(format!(
                        "monologue {monologue_index} element {element_index} has invalid {label}"
                    )));
                }
            }
            if let (Some(start), Some(end)) = (element.ts, element.end_ts)
                && end < start
            {
                return Err(RevAsrEvidenceCacheError::InvalidEvidence(format!(
                    "monologue {monologue_index} element {element_index} ends before it starts"
                )));
            }
            if let Some(confidence) = element.confidence
                && (!confidence.is_finite() || !(0.0..=1.0).contains(&confidence))
            {
                return Err(RevAsrEvidenceCacheError::InvalidEvidence(format!(
                    "monologue {monologue_index} element {element_index} has invalid confidence"
                )));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredRevAsrEvidence {
    schema_version: u32,
    request_fingerprint: String,
    evidence: CompletedRevAsrEvidence,
}

impl StoredRevAsrEvidence {
    fn validate_for(
        &self,
        request: &RevAsrEvidenceRequest,
    ) -> Result<(), RevAsrEvidenceCacheError> {
        if self.schema_version != REV_ASR_EVIDENCE_SCHEMA_VERSION {
            return Err(RevAsrEvidenceCacheError::InvalidEvidence(format!(
                "schema version {} does not match {}",
                self.schema_version, REV_ASR_EVIDENCE_SCHEMA_VERSION
            )));
        }
        if self.request_fingerprint != request.cache_key.as_str() {
            return Err(RevAsrEvidenceCacheError::InvalidEvidence(
                "request fingerprint does not match cache key".to_owned(),
            ));
        }
        validate_evidence(&self.evidence)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevAsrEvidenceMissReason {
    NotFound,
    ForcedRefresh,
}

#[derive(Debug)]
enum RevAsrEvidenceLookup {
    Hit(CompletedRevAsrEvidence),
    Miss(RevAsrEvidenceMiss),
}

#[derive(Debug)]
struct RevAsrEvidenceMiss {
    request: RevAsrEvidenceRequest,
    reason: RevAsrEvidenceMissReason,
    _lease: InferenceLease,
}

impl RevAsrEvidenceMiss {
    fn authorize(self) -> RevAsrInferenceAuthorization {
        RevAsrInferenceAuthorization {
            request: self.request,
            reason: self.reason,
            _lease: self._lease,
        }
    }
}

#[derive(Debug)]
pub(crate) struct RevAsrInferenceAuthorization {
    request: RevAsrEvidenceRequest,
    reason: RevAsrEvidenceMissReason,
    _lease: InferenceLease,
}

impl RevAsrInferenceAuthorization {
    async fn commit(
        self,
        cache: &UtteranceCache,
        evidence: CompletedRevAsrEvidence,
    ) -> Result<CompletedRevAsrEvidence, RevAsrEvidenceCacheError> {
        validate_evidence(&evidence)?;
        let envelope = StoredRevAsrEvidence {
            schema_version: REV_ASR_EVIDENCE_SCHEMA_VERSION,
            request_fingerprint: self.request.cache_key.to_string(),
            evidence,
        };
        let value = serde_json::to_value(&envelope)?;
        cache
            .put(
                self.request.cache_key.as_str(),
                CacheTaskName::RevAsrEvidence.as_str(),
                self.request.model_revision.as_str(),
                env!("CARGO_PKG_VERSION"),
                &value,
            )
            .await?;
        self._lease.mark_committed();
        Ok(envelope.evidence)
    }
}

#[async_trait::async_trait]
pub(crate) trait RevAsrEvidenceInference: Sync {
    async fn infer(
        &self,
        authorization: &RevAsrInferenceAuthorization,
    ) -> Result<CompletedRevAsrEvidence, ServerError>;
}

#[derive(Debug)]
pub(crate) enum RevAsrEvidenceResolution {
    Replayed(CompletedRevAsrEvidence),
    Inferred {
        evidence: CompletedRevAsrEvidence,
        reason: RevAsrEvidenceMissReason,
    },
}

pub(crate) async fn resolve_rev_asr_evidence<I: RevAsrEvidenceInference>(
    request: &RevAsrEvidenceRequest,
    cache: &UtteranceCache,
    policy: CachePolicy,
    inference: &I,
) -> Result<RevAsrEvidenceResolution, RevAsrEvidenceResolutionError> {
    match request.lookup(cache, policy).await? {
        RevAsrEvidenceLookup::Hit(evidence) => Ok(RevAsrEvidenceResolution::Replayed(evidence)),
        RevAsrEvidenceLookup::Miss(miss) => {
            let authorization = miss.authorize();
            let reason = authorization.reason;
            let evidence = inference.infer(&authorization).await?;
            let evidence = authorization.commit(cache, evidence).await?;
            Ok(RevAsrEvidenceResolution::Inferred { evidence, reason })
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RevAsrEvidenceResolutionError {
    #[error(transparent)]
    Evidence(#[from] RevAsrEvidenceCacheError),
    #[error(transparent)]
    Inference(#[from] ServerError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RevAsrEvidenceCacheError {
    #[error("could not read Rev.AI provider media: {0}")]
    Io(#[from] std::io::Error),
    #[error("Rev.AI evidence cache failed: {0}")]
    Cache(#[from] CacheError),
    #[error("Rev.AI evidence serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid Rev.AI evidence request: {0}")]
    InvalidRequest(String),
    #[error("invalid cached Rev.AI evidence: {0}")]
    InvalidEvidence(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::LanguageCode3;
    use crate::cache::CacheStats;
    use crate::revai::types::{Element, Monologue};
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn sample_evidence() -> CompletedRevAsrEvidence {
        CompletedRevAsrEvidence {
            transcript: Transcript {
                monologues: vec![Monologue {
                    speaker: 0,
                    elements: vec![Element {
                        element_type: "text".to_owned(),
                        value: "hello".to_owned(),
                        ts: Some(0.1),
                        end_ts: Some(0.5),
                        confidence: Some(0.9),
                    }],
                }],
            },
            resolved_language: LanguageCode3::eng(),
        }
    }

    struct CountingRevService {
        calls: AtomicUsize,
        delay_ms: u64,
    }

    struct FailingCommitBackend;

    #[async_trait::async_trait]
    impl CacheBackend for FailingCommitBackend {
        async fn get(
            &self,
            _key: &str,
            _task: &str,
            _engine_version: &str,
        ) -> Result<Option<serde_json::Value>, CacheError> {
            Ok(None)
        }

        async fn get_batch(
            &self,
            _keys: &[String],
            _task: &str,
            _engine_version: &str,
        ) -> Result<HashMap<String, serde_json::Value>, CacheError> {
            Ok(HashMap::new())
        }

        async fn put(
            &self,
            _key: &str,
            _task: &str,
            _engine_version: &str,
            _ba_version: &str,
            _data: &serde_json::Value,
        ) -> Result<(), CacheError> {
            Err(CacheError::Io(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "injected durable commit failure",
            )))
        }

        async fn put_batch(
            &self,
            _entries: &[(String, serde_json::Value)],
            _task: &str,
            _engine_version: &str,
            _ba_version: &str,
        ) -> Result<(), CacheError> {
            unreachable!("evidence commits are single-entry writes")
        }

        async fn delete_batch(&self, _keys: &[String], _task: &str) -> Result<usize, CacheError> {
            unreachable!("evidence resolution does not delete cache entries")
        }

        async fn stats(&self) -> Result<CacheStats, CacheError> {
            unreachable!("evidence resolution does not request cache statistics")
        }
    }

    #[async_trait::async_trait]
    impl RevAsrEvidenceInference for CountingRevService {
        async fn infer(
            &self,
            _authorization: &RevAsrInferenceAuthorization,
        ) -> Result<CompletedRevAsrEvidence, ServerError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.delay_ms > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
            }
            Ok(sample_evidence())
        }
    }

    #[tokio::test]
    async fn rev_key_reuses_identical_media_but_invalidates_semantic_changes() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let first = tempdir.path().join("first.wav");
        let renamed = tempdir.path().join("renamed.wav");
        tokio::fs::write(&first, b"provider media")
            .await
            .expect("write first");
        tokio::fs::write(&renamed, b"provider media")
            .await
            .expect("write renamed");
        let revision = RevAsrModelRevision::current();

        let baseline = RevAsrEvidenceRequest::from_audio(
            &first,
            &LanguageSpec::Resolved(LanguageCode3::eng()),
            NumSpeakers(2),
            &revision,
        )
        .await
        .expect("baseline request");
        let copied = RevAsrEvidenceRequest::from_audio(
            &renamed,
            &LanguageSpec::Resolved(LanguageCode3::eng()),
            NumSpeakers(2),
            &revision,
        )
        .await
        .expect("copied request");
        let auto_language = RevAsrEvidenceRequest::from_audio(
            &first,
            &LanguageSpec::Auto,
            NumSpeakers(2),
            &revision,
        )
        .await
        .expect("auto-language request");
        let three_speakers = RevAsrEvidenceRequest::from_audio(
            &first,
            &LanguageSpec::Resolved(LanguageCode3::eng()),
            NumSpeakers(3),
            &revision,
        )
        .await
        .expect("three-speaker request");
        tokio::fs::write(&renamed, b"changed provider media")
            .await
            .expect("change copied media");
        let changed_media = RevAsrEvidenceRequest::from_audio(
            &renamed,
            &LanguageSpec::Resolved(LanguageCode3::eng()),
            NumSpeakers(2),
            &revision,
        )
        .await
        .expect("changed-media request");

        assert_eq!(baseline.cache_key(), copied.cache_key());
        assert_ne!(baseline.cache_key(), auto_language.cache_key());
        assert_ne!(baseline.cache_key(), three_speakers.cache_key());
        assert_ne!(baseline.cache_key(), changed_media.cache_key());
    }

    #[tokio::test]
    async fn durable_replay_does_not_cross_rev_service_again() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"provider media")
            .await
            .expect("write audio");
        let cache_dir = tempdir.path().join("cache");
        let cache = UtteranceCache::sqlite(Some(cache_dir.clone()))
            .await
            .expect("cache");
        let request = RevAsrEvidenceRequest::from_audio(
            &audio,
            &LanguageSpec::Resolved(LanguageCode3::eng()),
            NumSpeakers(2),
            &RevAsrModelRevision::current(),
        )
        .await
        .expect("request");
        let service = CountingRevService {
            calls: AtomicUsize::new(0),
            delay_ms: 0,
        };
        let cold = resolve_rev_asr_evidence(&request, &cache, CachePolicy::UseCache, &service)
            .await
            .expect("cold");
        let cold_evidence = match cold {
            RevAsrEvidenceResolution::Inferred { evidence, .. } => evidence,
            RevAsrEvidenceResolution::Replayed(_) => panic!("empty cache must infer"),
        };
        let cold_response = crate::revai::rev_evidence_to_asr_response(&cold_evidence);
        drop(cache);

        let reopened = UtteranceCache::sqlite(Some(cache_dir))
            .await
            .expect("reopen");
        let warm = resolve_rev_asr_evidence(&request, &reopened, CachePolicy::UseCache, &service)
            .await
            .expect("warm");
        let warm_evidence = match warm {
            RevAsrEvidenceResolution::Replayed(evidence) => evidence,
            RevAsrEvidenceResolution::Inferred { .. } => panic!("durable cache must replay"),
        };
        let warm_response = crate::revai::rev_evidence_to_asr_response(&warm_evidence);
        assert_eq!(
            serde_json::to_value(cold_response).expect("cold response JSON"),
            serde_json::to_value(warm_response).expect("warm response JSON")
        );
        assert_eq!(service.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn concurrent_rev_resolvers_cross_service_once() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"provider media")
            .await
            .expect("write audio");
        let cache = UtteranceCache::sqlite(Some(tempdir.path().join("cache")))
            .await
            .expect("cache");
        let request = RevAsrEvidenceRequest::from_audio(
            &audio,
            &LanguageSpec::Resolved(LanguageCode3::eng()),
            NumSpeakers(2),
            &RevAsrModelRevision::current(),
        )
        .await
        .expect("request");
        let service = CountingRevService {
            calls: AtomicUsize::new(0),
            delay_ms: 25,
        };

        let (first, second) = tokio::join!(
            resolve_rev_asr_evidence(&request, &cache, CachePolicy::UseCache, &service),
            resolve_rev_asr_evidence(&request, &cache, CachePolicy::UseCache, &service),
        );
        let first = first.expect("first resolution");
        let second = second.expect("second resolution");
        assert!(matches!(
            (&first, &second),
            (
                RevAsrEvidenceResolution::Inferred { .. },
                RevAsrEvidenceResolution::Replayed(_)
            ) | (
                RevAsrEvidenceResolution::Replayed(_),
                RevAsrEvidenceResolution::Inferred { .. }
            )
        ));
        assert_eq!(service.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn concurrent_forced_refreshes_share_one_fresh_service_result() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"provider media")
            .await
            .expect("write audio");
        let cache = UtteranceCache::sqlite(Some(tempdir.path().join("cache")))
            .await
            .expect("cache");
        let request = RevAsrEvidenceRequest::from_audio(
            &audio,
            &LanguageSpec::Resolved(LanguageCode3::eng()),
            NumSpeakers(2),
            &RevAsrModelRevision::current(),
        )
        .await
        .expect("request");
        let service = CountingRevService {
            calls: AtomicUsize::new(0),
            delay_ms: 25,
        };

        let (first, second) = tokio::join!(
            resolve_rev_asr_evidence(&request, &cache, CachePolicy::SkipCache, &service),
            resolve_rev_asr_evidence(&request, &cache, CachePolicy::SkipCache, &service),
        );
        let first = first.expect("first refresh");
        let second = second.expect("second refresh");

        assert!(matches!(
            (&first, &second),
            (
                RevAsrEvidenceResolution::Inferred { .. },
                RevAsrEvidenceResolution::Replayed(_)
            ) | (
                RevAsrEvidenceResolution::Replayed(_),
                RevAsrEvidenceResolution::Inferred { .. }
            )
        ));
        assert_eq!(service.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn corrupt_rev_evidence_fails_without_service_call() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"provider media")
            .await
            .expect("write audio");
        let cache = UtteranceCache::sqlite(Some(tempdir.path().join("cache")))
            .await
            .expect("cache");
        let request = RevAsrEvidenceRequest::from_audio(
            &audio,
            &LanguageSpec::Resolved(LanguageCode3::eng()),
            NumSpeakers(2),
            &RevAsrModelRevision::current(),
        )
        .await
        .expect("request");
        request
            .store_unchecked_for_test(&cache, serde_json::json!({"evidence": "broken"}))
            .await
            .expect("seed");
        let service = CountingRevService {
            calls: AtomicUsize::new(0),
            delay_ms: 0,
        };
        let error = resolve_rev_asr_evidence(&request, &cache, CachePolicy::UseCache, &service)
            .await
            .expect_err("corruption must fail");
        assert!(error.to_string().contains("invalid cached Rev.AI evidence"));
        assert_eq!(service.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn successful_inference_with_failed_commit_is_not_reported_as_resolved() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"provider media")
            .await
            .expect("write audio");
        let cache = UtteranceCache::from_backend(Box::new(FailingCommitBackend));
        let request = RevAsrEvidenceRequest::from_audio(
            &audio,
            &LanguageSpec::Resolved(LanguageCode3::eng()),
            NumSpeakers(2),
            &RevAsrModelRevision::current(),
        )
        .await
        .expect("request");
        let service = CountingRevService {
            calls: AtomicUsize::new(0),
            delay_ms: 0,
        };

        let error = resolve_rev_asr_evidence(&request, &cache, CachePolicy::UseCache, &service)
            .await
            .expect_err("commit failure must fail the resolution");

        assert!(
            error
                .to_string()
                .contains("injected durable commit failure")
        );
        assert_eq!(service.calls.load(Ordering::SeqCst), 1);
    }
}
