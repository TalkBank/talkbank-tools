//! Durable evidence captured at paid inference boundaries.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::api::{EngineVersion, NumSpeakers};
use crate::cache::{CacheBackend, CacheError, InferenceLease, UtteranceCache};
use crate::chat_ops::{CacheKey, CacheTaskName};
use crate::error::ServerError;
use crate::params::CachePolicy;
use crate::types::worker_v2::{SpeakerBackendV2, SpeakerSegmentV2};

const SPEAKER_EVIDENCE_SCHEMA_VERSION: u32 = 1;
const SPEAKER_AUDIO_PREPARATION_REVISION: &str = "mono-16khz-f32le-v1";

/// A digest of the media-source bytes passed to canonical audio preparation.
///
/// Unlike the legacy audio identity, this contains neither a path nor an
/// mtime. Copies and renames of identical source media therefore share paid
/// evidence. The preparation recipe has its own revision in the semantic key;
/// changing how source media becomes model-ready PCM must bump that revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct SpeakerAudioSourceDigest(String);

impl SpeakerAudioSourceDigest {
    async fn from_path(path: &Path) -> Result<Self, SpeakerEvidenceCacheError> {
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
struct SpeakerEvidenceKeyMaterial<'a> {
    schema_version: u32,
    audio_source_blake3: &'a SpeakerAudioSourceDigest,
    audio_preparation_revision: &'static str,
    backend: SpeakerBackendV2,
    expected_speakers: u32,
    model_revision: &'a str,
}

/// Revision identity for speaker evidence, distinct from an ASR engine version.
///
/// The private field prevents the transcribe pipeline from accidentally using
/// its ASR worker version to scope speaker-model evidence.
#[derive(Debug, Clone)]
pub(crate) struct SpeakerEvidenceModelRevision(EngineVersion);

impl SpeakerEvidenceModelRevision {
    pub(crate) fn for_backend(backend: SpeakerBackendV2) -> Self {
        let revision = match backend {
            SpeakerBackendV2::PyannoteAi => "pyannote-ai:precision-2".to_owned(),
            SpeakerBackendV2::Pyannote => format!(
                "pyannote-local:talkbank/dia-fork:ba-{}",
                env!("CARGO_PKG_VERSION")
            ),
            SpeakerBackendV2::Nemo => {
                format!("nemo:diar_infer_general:ba-{}", env!("CARGO_PKG_VERSION"))
            }
        };
        Self(EngineVersion::from(revision.as_str()))
    }

    fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    #[cfg(test)]
    fn for_test(label: &str) -> Self {
        Self(EngineVersion::from(label))
    }
}

/// Semantic identity of one speaker-diarization request.
///
/// Construction reads and hashes the prepared audio. All fields that can
/// change inference semantics are owned here so downstream stages cannot
/// accidentally invent partial cache keys.
#[derive(Debug, Clone)]
pub(crate) struct SpeakerEvidenceRequest {
    cache_key: CacheKey,
    model_revision: SpeakerEvidenceModelRevision,
}

impl SpeakerEvidenceRequest {
    pub(crate) async fn from_audio(
        audio_path: &Path,
        backend: SpeakerBackendV2,
        expected_speakers: NumSpeakers,
        model_revision: &SpeakerEvidenceModelRevision,
    ) -> Result<Self, SpeakerEvidenceCacheError> {
        let digest = SpeakerAudioSourceDigest::from_path(audio_path).await?;
        let key_material = SpeakerEvidenceKeyMaterial {
            schema_version: SPEAKER_EVIDENCE_SCHEMA_VERSION,
            audio_source_blake3: &digest,
            audio_preparation_revision: SPEAKER_AUDIO_PREPARATION_REVISION,
            backend,
            expected_speakers: expected_speakers.0,
            model_revision: model_revision.as_str(),
        };
        let canonical = serde_json::to_string(&key_material)?;
        Ok(Self {
            cache_key: CacheKey::from_content(&canonical),
            model_revision: model_revision.clone(),
        })
    }

    pub(crate) fn cache_key(&self) -> &CacheKey {
        &self.cache_key
    }

    pub(crate) async fn lookup(
        &self,
        cache: &UtteranceCache,
        policy: CachePolicy,
    ) -> Result<SpeakerEvidenceLookup, SpeakerEvidenceCacheError> {
        let lease = InferenceLease::acquire(&self.cache_key, cache).await;
        if policy.should_skip() && !lease.observed_commit_while_waiting() {
            return Ok(SpeakerEvidenceLookup::Miss(SpeakerEvidenceMiss {
                request: self.clone(),
                reason: SpeakerEvidenceMissReason::ForcedRefresh,
                _lease: lease,
            }));
        }

        let stored = cache
            .get(
                self.cache_key.as_str(),
                CacheTaskName::SpeakerDiarizationEvidence.as_str(),
                self.model_revision.as_str(),
            )
            .await?;
        let Some(stored) = stored else {
            return Ok(SpeakerEvidenceLookup::Miss(SpeakerEvidenceMiss {
                request: self.clone(),
                reason: SpeakerEvidenceMissReason::NotFound,
                _lease: lease,
            }));
        };

        let envelope: StoredSpeakerEvidence = serde_json::from_value(stored)
            .map_err(|error| SpeakerEvidenceCacheError::InvalidCachedEvidence(error.to_string()))?;
        envelope.validate_for(self)?;
        Ok(SpeakerEvidenceLookup::Hit(ValidatedSpeakerEvidence {
            segments: envelope.segments,
        }))
    }

    #[cfg(test)]
    async fn store_unchecked_for_test(
        &self,
        cache: &UtteranceCache,
        value: serde_json::Value,
    ) -> Result<(), SpeakerEvidenceCacheError> {
        cache
            .put(
                self.cache_key.as_str(),
                CacheTaskName::SpeakerDiarizationEvidence.as_str(),
                self.model_revision.as_str(),
                env!("CARGO_PKG_VERSION"),
                &value,
            )
            .await?;
        Ok(())
    }
}

/// Result of checking durable speaker evidence.
///
/// A hit contains only replayable evidence. Only the miss variant contains the
/// state transition that can authorize an inference call.
#[derive(Debug)]
pub(crate) enum SpeakerEvidenceLookup {
    Hit(ValidatedSpeakerEvidence),
    Miss(SpeakerEvidenceMiss),
}

/// The only service boundary accepted by the production evidence resolver.
///
/// Tests implement this trait with a call-counting fake. Production implements
/// it with the V2 speaker worker adapter. Both therefore exercise the same
/// cache-hit/miss and commit control flow.
#[async_trait::async_trait]
pub(crate) trait SpeakerEvidenceInference: Sync {
    async fn infer(
        &self,
        authorization: &SpeakerInferenceAuthorization,
    ) -> Result<Vec<SpeakerSegmentV2>, ServerError>;
}

/// How one request acquired validated speaker evidence.
#[derive(Debug)]
pub(crate) enum SpeakerEvidenceResolution {
    Replayed(ValidatedSpeakerEvidence),
    Inferred {
        evidence: ValidatedSpeakerEvidence,
        reason: SpeakerEvidenceMissReason,
    },
}

/// Resolve evidence through the same cache-or-infer state machine in tests and
/// production.
pub(crate) async fn resolve_speaker_evidence<I: SpeakerEvidenceInference>(
    request: &SpeakerEvidenceRequest,
    cache: &UtteranceCache,
    policy: CachePolicy,
    inference: &I,
) -> Result<SpeakerEvidenceResolution, SpeakerEvidenceResolutionError> {
    match request.lookup(cache, policy).await? {
        SpeakerEvidenceLookup::Hit(evidence) => Ok(SpeakerEvidenceResolution::Replayed(evidence)),
        SpeakerEvidenceLookup::Miss(miss) => {
            let authorization = miss.authorize_billable_inference();
            let reason = authorization.reason();
            let segments = inference.infer(&authorization).await?;
            let evidence = authorization.commit(cache, segments).await?;
            Ok(SpeakerEvidenceResolution::Inferred { evidence, reason })
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SpeakerEvidenceResolutionError {
    #[error(transparent)]
    Evidence(#[from] SpeakerEvidenceCacheError),
    #[error(transparent)]
    Inference(#[from] ServerError),
}

/// Speaker turns that passed cache-envelope and timing validation.
#[derive(Debug, Clone)]
pub(crate) struct ValidatedSpeakerEvidence {
    segments: Vec<SpeakerSegmentV2>,
}

impl ValidatedSpeakerEvidence {
    pub(crate) fn segments(&self) -> &[SpeakerSegmentV2] {
        &self.segments
    }

    pub(crate) fn into_segments(self) -> Vec<SpeakerSegmentV2> {
        self.segments
    }
}

/// Why no reusable entry was returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpeakerEvidenceMissReason {
    NotFound,
    ForcedRefresh,
}

/// A proven cache miss. Its fields are private so other modules cannot forge
/// permission to call the inference service.
#[derive(Debug)]
pub(crate) struct SpeakerEvidenceMiss {
    request: SpeakerEvidenceRequest,
    reason: SpeakerEvidenceMissReason,
    _lease: InferenceLease,
}

impl SpeakerEvidenceMiss {
    pub(crate) fn authorize_billable_inference(self) -> SpeakerInferenceAuthorization {
        SpeakerInferenceAuthorization {
            request: self.request,
            reason: self.reason,
            _lease: self._lease,
        }
    }
}

/// Capability required by the raw speaker-inference function.
///
/// The only constructor consumes [`SpeakerEvidenceMiss`]. A cache hit has no
/// route to this type, so it cannot accidentally fall through to a paid call.
#[derive(Debug)]
pub(crate) struct SpeakerInferenceAuthorization {
    request: SpeakerEvidenceRequest,
    reason: SpeakerEvidenceMissReason,
    _lease: InferenceLease,
}

impl SpeakerInferenceAuthorization {
    pub(crate) fn reason(&self) -> SpeakerEvidenceMissReason {
        self.reason
    }

    pub(crate) async fn commit(
        self,
        cache: &UtteranceCache,
        segments: Vec<SpeakerSegmentV2>,
    ) -> Result<ValidatedSpeakerEvidence, SpeakerEvidenceCacheError> {
        validate_segments(&segments)?;
        let envelope = StoredSpeakerEvidence {
            schema_version: SPEAKER_EVIDENCE_SCHEMA_VERSION,
            request_fingerprint: self.request.cache_key.to_string(),
            segments,
        };
        let value = serde_json::to_value(&envelope)?;
        cache
            .put(
                self.request.cache_key.as_str(),
                CacheTaskName::SpeakerDiarizationEvidence.as_str(),
                self.request.model_revision.as_str(),
                env!("CARGO_PKG_VERSION"),
                &value,
            )
            .await?;
        self._lease.mark_committed();
        Ok(ValidatedSpeakerEvidence {
            segments: envelope.segments,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredSpeakerEvidence {
    schema_version: u32,
    request_fingerprint: String,
    segments: Vec<SpeakerSegmentV2>,
}

impl StoredSpeakerEvidence {
    fn validate_for(
        &self,
        request: &SpeakerEvidenceRequest,
    ) -> Result<(), SpeakerEvidenceCacheError> {
        if self.schema_version != SPEAKER_EVIDENCE_SCHEMA_VERSION {
            return Err(SpeakerEvidenceCacheError::InvalidCachedEvidence(format!(
                "schema version {} does not match {}",
                self.schema_version, SPEAKER_EVIDENCE_SCHEMA_VERSION
            )));
        }
        if self.request_fingerprint != request.cache_key.as_str() {
            return Err(SpeakerEvidenceCacheError::InvalidCachedEvidence(
                "request fingerprint does not match cache key".to_owned(),
            ));
        }
        validate_segments(&self.segments)
    }
}

fn validate_segments(segments: &[SpeakerSegmentV2]) -> Result<(), SpeakerEvidenceCacheError> {
    let mut previous_start = 0_u64;
    for (index, segment) in segments.iter().enumerate() {
        if segment.speaker.trim().is_empty() {
            return Err(SpeakerEvidenceCacheError::InvalidCachedEvidence(format!(
                "segment {index} has an empty speaker label"
            )));
        }
        if segment.end_ms.0 < segment.start_ms.0 {
            return Err(SpeakerEvidenceCacheError::InvalidCachedEvidence(format!(
                "segment {index} has an inverted interval {}..{}",
                segment.start_ms.0, segment.end_ms.0
            )));
        }
        if index > 0 && segment.start_ms.0 < previous_start {
            return Err(SpeakerEvidenceCacheError::InvalidCachedEvidence(format!(
                "segment {index} starts before the preceding segment"
            )));
        }
        previous_start = segment.start_ms.0;
    }
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SpeakerEvidenceCacheError {
    #[error("could not read prepared audio for speaker evidence: {0}")]
    Io(#[from] std::io::Error),
    #[error("speaker evidence cache failed: {0}")]
    Cache(#[from] CacheError),
    #[error("speaker evidence key/envelope serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid cached speaker evidence: {0}")]
    InvalidCachedEvidence(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{DurationMs, NumSpeakers};
    use crate::cache::UtteranceCache;
    use crate::error::ServerError;
    use crate::types::worker_v2::{SpeakerBackendV2, SpeakerSegmentV2};
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingSpeakerService {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl SpeakerEvidenceInference for CountingSpeakerService {
        async fn infer(
            &self,
            _authorization: &SpeakerInferenceAuthorization,
        ) -> Result<Vec<SpeakerSegmentV2>, ServerError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![segment("SPEAKER_00")])
        }
    }

    fn segment(speaker: &str) -> SpeakerSegmentV2 {
        SpeakerSegmentV2 {
            start_ms: DurationMs(0),
            end_ms: DurationMs(750),
            speaker: speaker.to_owned(),
        }
    }

    #[tokio::test]
    async fn identical_audio_bytes_share_a_key_across_paths() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let first = tempdir.path().join("first.wav");
        let renamed = tempdir.path().join("renamed.wav");
        tokio::fs::write(&first, b"same prepared audio")
            .await
            .expect("write first");
        tokio::fs::write(&renamed, b"same prepared audio")
            .await
            .expect("write renamed");

        let first_request = SpeakerEvidenceRequest::from_audio(
            &first,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(2),
            &SpeakerEvidenceModelRevision::for_test("precision-2"),
        )
        .await
        .expect("first request");
        let renamed_request = SpeakerEvidenceRequest::from_audio(
            &renamed,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(2),
            &SpeakerEvidenceModelRevision::for_test("precision-2"),
        )
        .await
        .expect("renamed request");

        assert_eq!(first_request.cache_key(), renamed_request.cache_key());
    }

    #[tokio::test]
    async fn semantic_request_changes_invalidate_the_key() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"prepared audio")
            .await
            .expect("write audio");

        let two_speakers = SpeakerEvidenceRequest::from_audio(
            &audio,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(2),
            &SpeakerEvidenceModelRevision::for_test("precision-2"),
        )
        .await
        .expect("two-speaker request");
        let three_speakers = SpeakerEvidenceRequest::from_audio(
            &audio,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(3),
            &SpeakerEvidenceModelRevision::for_test("precision-2"),
        )
        .await
        .expect("three-speaker request");
        let revised_model = SpeakerEvidenceRequest::from_audio(
            &audio,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(2),
            &SpeakerEvidenceModelRevision::for_test("precision-3"),
        )
        .await
        .expect("revised-model request");

        assert_ne!(two_speakers.cache_key(), three_speakers.cache_key());
        assert_ne!(two_speakers.cache_key(), revised_model.cache_key());
    }

    #[tokio::test]
    async fn only_a_miss_can_become_billable_and_commit_evidence() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"prepared audio")
            .await
            .expect("write audio");
        let cache_dir = tempdir.path().join("cache");
        let cache = UtteranceCache::sqlite(Some(cache_dir.clone()))
            .await
            .expect("cache");
        let request = SpeakerEvidenceRequest::from_audio(
            &audio,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(2),
            &SpeakerEvidenceModelRevision::for_test("precision-2"),
        )
        .await
        .expect("request");

        let lookup = request
            .lookup(&cache, CachePolicy::UseCache)
            .await
            .expect("lookup");
        let miss = match lookup {
            SpeakerEvidenceLookup::Hit(_) => panic!("empty cache must miss"),
            SpeakerEvidenceLookup::Miss(miss) => miss,
        };
        let billable = miss.authorize_billable_inference();
        let committed = billable
            .commit(&cache, vec![segment("SPEAKER_00")])
            .await
            .expect("commit");
        assert_eq!(committed.segments(), &[segment("SPEAKER_00")]);

        let hit = request
            .lookup(&cache, CachePolicy::UseCache)
            .await
            .expect("lookup hit");
        match hit {
            SpeakerEvidenceLookup::Hit(evidence) => {
                assert_eq!(evidence.segments(), &[segment("SPEAKER_00")]);
            }
            SpeakerEvidenceLookup::Miss(_) => panic!("committed evidence must hit"),
        }
    }

    #[tokio::test]
    async fn concurrent_identical_lookup_waits_for_first_inference_then_hits() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"prepared audio")
            .await
            .expect("write audio");
        let cache = std::sync::Arc::new(
            UtteranceCache::sqlite(Some(tempdir.path().join("cache")))
                .await
                .expect("cache"),
        );
        let request = SpeakerEvidenceRequest::from_audio(
            &audio,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(2),
            &SpeakerEvidenceModelRevision::for_test("precision-2"),
        )
        .await
        .expect("request");
        let first = match request
            .lookup(&cache, CachePolicy::UseCache)
            .await
            .expect("first lookup")
        {
            SpeakerEvidenceLookup::Hit(_) => panic!("empty cache must miss"),
            SpeakerEvidenceLookup::Miss(miss) => miss.authorize_billable_inference(),
        };

        let second_request = request.clone();
        let second_cache = cache.clone();
        let mut second = tokio::spawn(async move {
            second_request
                .lookup(&second_cache, CachePolicy::UseCache)
                .await
        });
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(20), &mut second)
                .await
                .is_err(),
            "a duplicate lookup must wait while the first miss owns inference"
        );

        first
            .commit(&cache, vec![segment("SPEAKER_00")])
            .await
            .expect("commit first inference");
        let second_lookup = second.await.expect("second task").expect("second lookup");
        assert!(matches!(second_lookup, SpeakerEvidenceLookup::Hit(_)));
    }

    #[tokio::test]
    async fn production_resolver_crosses_billable_boundary_once_then_replays() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"prepared audio")
            .await
            .expect("write audio");
        let cache_dir = tempdir.path().join("cache");
        let cache = UtteranceCache::sqlite(Some(cache_dir.clone()))
            .await
            .expect("cache");
        let request = SpeakerEvidenceRequest::from_audio(
            &audio,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(2),
            &SpeakerEvidenceModelRevision::for_test("precision-2"),
        )
        .await
        .expect("request");
        let service = CountingSpeakerService {
            calls: AtomicUsize::new(0),
        };

        let cold = resolve_speaker_evidence(&request, &cache, CachePolicy::UseCache, &service)
            .await
            .expect("cold resolution");
        assert!(matches!(cold, SpeakerEvidenceResolution::Inferred { .. }));
        drop(cache);
        let reopened_cache = UtteranceCache::sqlite(Some(cache_dir))
            .await
            .expect("reopened cache");
        let warm =
            resolve_speaker_evidence(&request, &reopened_cache, CachePolicy::UseCache, &service)
                .await
                .expect("warm resolution");
        assert!(matches!(warm, SpeakerEvidenceResolution::Replayed(_)));
        assert_eq!(service.calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn corrupt_cached_evidence_fails_closed_instead_of_rebilling() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"prepared audio")
            .await
            .expect("write audio");
        let cache = UtteranceCache::sqlite(Some(tempdir.path().join("cache")))
            .await
            .expect("cache");
        let request = SpeakerEvidenceRequest::from_audio(
            &audio,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(2),
            &SpeakerEvidenceModelRevision::for_test("precision-2"),
        )
        .await
        .expect("request");
        request
            .store_unchecked_for_test(&cache, serde_json::json!({"segments": "not-an-array"}))
            .await
            .expect("seed corrupt entry");

        let error = request
            .lookup(&cache, CachePolicy::UseCache)
            .await
            .expect_err("corruption must not become a miss");
        assert!(
            error
                .to_string()
                .contains("invalid cached speaker evidence")
        );
    }

    #[tokio::test]
    async fn forced_refresh_is_a_typed_miss_even_when_evidence_exists() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"prepared audio")
            .await
            .expect("write audio");
        let cache = UtteranceCache::sqlite(Some(tempdir.path().join("cache")))
            .await
            .expect("cache");
        let request = SpeakerEvidenceRequest::from_audio(
            &audio,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(2),
            &SpeakerEvidenceModelRevision::for_test("precision-2"),
        )
        .await
        .expect("request");
        let miss = match request
            .lookup(&cache, CachePolicy::UseCache)
            .await
            .expect("initial miss")
        {
            SpeakerEvidenceLookup::Hit(_) => panic!("empty cache must miss"),
            SpeakerEvidenceLookup::Miss(miss) => miss,
        };
        miss.authorize_billable_inference()
            .commit(&cache, vec![segment("SPEAKER_00")])
            .await
            .expect("commit");

        let refresh = request
            .lookup(&cache, CachePolicy::SkipCache)
            .await
            .expect("refresh lookup");
        let authorization = match refresh {
            SpeakerEvidenceLookup::Hit(_) => panic!("refresh must not hit"),
            SpeakerEvidenceLookup::Miss(miss) => miss.authorize_billable_inference(),
        };
        assert_eq!(
            authorization.reason(),
            SpeakerEvidenceMissReason::ForcedRefresh
        );
    }

    #[tokio::test]
    async fn invalid_cached_timing_fails_closed() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"prepared audio")
            .await
            .expect("write audio");
        let cache = UtteranceCache::sqlite(Some(tempdir.path().join("cache")))
            .await
            .expect("cache");
        let request = SpeakerEvidenceRequest::from_audio(
            &audio,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(2),
            &SpeakerEvidenceModelRevision::for_test("precision-2"),
        )
        .await
        .expect("request");
        let invalid = serde_json::json!({
            "schema_version": SPEAKER_EVIDENCE_SCHEMA_VERSION,
            "request_fingerprint": request.cache_key().as_str(),
            "segments": [{
                "start_ms": 900,
                "end_ms": 100,
                "speaker": "SPEAKER_00"
            }]
        });
        request
            .store_unchecked_for_test(&cache, invalid)
            .await
            .expect("seed invalid timing");

        let error = request
            .lookup(&cache, CachePolicy::UseCache)
            .await
            .expect_err("invalid timing must not become a miss");
        assert!(error.to_string().contains("inverted interval"));
    }

    #[tokio::test]
    async fn zero_duration_segment_allowed_by_worker_protocol_round_trips() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"prepared audio")
            .await
            .expect("write audio");
        let cache = UtteranceCache::sqlite(Some(tempdir.path().join("cache")))
            .await
            .expect("cache");
        let request = SpeakerEvidenceRequest::from_audio(
            &audio,
            SpeakerBackendV2::PyannoteAi,
            NumSpeakers(2),
            &SpeakerEvidenceModelRevision::for_test("precision-2"),
        )
        .await
        .expect("request");
        let zero_duration = SpeakerSegmentV2 {
            start_ms: DurationMs(500),
            end_ms: DurationMs(500),
            speaker: "SPEAKER_00".to_owned(),
        };

        let miss = match request
            .lookup(&cache, CachePolicy::UseCache)
            .await
            .expect("initial lookup")
        {
            SpeakerEvidenceLookup::Hit(_) => panic!("empty cache must miss"),
            SpeakerEvidenceLookup::Miss(miss) => miss,
        };
        miss.authorize_billable_inference()
            .commit(&cache, vec![zero_duration.clone()])
            .await
            .expect("worker-valid zero-duration evidence should commit");

        let hit = request
            .lookup(&cache, CachePolicy::UseCache)
            .await
            .expect("replay");
        let SpeakerEvidenceLookup::Hit(evidence) = hit else {
            panic!("committed evidence must replay");
        };
        assert_eq!(evidence.segments(), &[zero_duration]);
    }
}
