//! Durable evidence captured at paid inference boundaries.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;

use crate::api::{EngineVersion, NumSpeakers};
use crate::cache::{CacheBackend, CacheError, InferenceLease, UtteranceCache};
use crate::chat_ops::{CacheKey, CacheTaskName};
use crate::error::ServerError;
use crate::params::CachePolicy;
use crate::types::worker_v2::{SpeakerBackendV2, SpeakerInferenceEvidenceV2, SpeakerSegmentV2};
use crate::worker::speaker_result_v2::normalize_speaker_evidence_v2;

const RAW_SPEAKER_EVIDENCE_SCHEMA_VERSION: u32 = 1;
const DERIVED_SPEAKER_EVIDENCE_SCHEMA_VERSION: u32 = 1;
const SPEAKER_AUDIO_PREPARATION_REVISION: &str = "mono-16khz-f32le-v1";
const SPEAKER_NORMALIZATION_REVISION: &str = "speaker-evidence-normalizer-v1";

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

#[derive(Debug, Serialize)]
struct DerivedSpeakerEvidenceKeyMaterial<'a> {
    schema_version: u32,
    raw_evidence_fingerprint: &'a str,
    normalization_revision: &'a str,
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

/// Identity of the local raw-evidence projection algorithm.
///
/// This cannot be substituted with a speaker model revision: changing this
/// value invalidates only derived segments and deliberately preserves paid raw
/// evidence.
#[derive(Debug, Clone)]
struct SpeakerNormalizationRevision(EngineVersion);

impl SpeakerNormalizationRevision {
    fn current() -> Self {
        Self(EngineVersion::from(SPEAKER_NORMALIZATION_REVISION))
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
    raw_cache_key: CacheKey,
    derived_cache_key: CacheKey,
    backend: SpeakerBackendV2,
    model_revision: SpeakerEvidenceModelRevision,
    normalization_revision: SpeakerNormalizationRevision,
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
            schema_version: RAW_SPEAKER_EVIDENCE_SCHEMA_VERSION,
            audio_source_blake3: &digest,
            audio_preparation_revision: SPEAKER_AUDIO_PREPARATION_REVISION,
            backend,
            expected_speakers: expected_speakers.0,
            model_revision: model_revision.as_str(),
        };
        let canonical = serde_json::to_string(&key_material)?;
        let raw_cache_key = CacheKey::from_content(&canonical);
        let normalization_revision = SpeakerNormalizationRevision::current();
        let derived_cache_key = derived_cache_key(&raw_cache_key, &normalization_revision)?;
        Ok(Self {
            raw_cache_key,
            derived_cache_key,
            backend,
            model_revision: model_revision.clone(),
            normalization_revision,
        })
    }

    pub(crate) fn cache_key(&self) -> &CacheKey {
        &self.raw_cache_key
    }

    #[cfg(test)]
    fn with_normalization_revision_for_test(mut self, revision: &str) -> Self {
        self.normalization_revision = SpeakerNormalizationRevision::for_test(revision);
        self.derived_cache_key =
            derived_cache_key(&self.raw_cache_key, &self.normalization_revision)
                .expect("test normalization revision should serialize");
        self
    }

    pub(crate) async fn lookup(
        &self,
        cache: &UtteranceCache,
        policy: CachePolicy,
    ) -> Result<SpeakerEvidenceLookup, SpeakerEvidenceCacheError> {
        let lease = InferenceLease::acquire(&self.raw_cache_key, cache).await;
        if policy.should_skip() && !lease.observed_commit_while_waiting() {
            return Ok(SpeakerEvidenceLookup::Miss(SpeakerEvidenceMiss {
                request: self.clone(),
                reason: SpeakerEvidenceMissReason::ForcedRefresh,
                _lease: lease,
            }));
        }

        let stored_derived = cache
            .get(
                self.derived_cache_key.as_str(),
                CacheTaskName::SpeakerDiarizationSegments.as_str(),
                self.normalization_revision.as_str(),
            )
            .await?;
        if let Some(stored) = stored_derived {
            let envelope: StoredDerivedSpeakerEvidence =
                serde_json::from_value(stored).map_err(|error| {
                    SpeakerEvidenceCacheError::InvalidCachedEvidence(error.to_string())
                })?;
            envelope.validate_for(self)?;
            return Ok(SpeakerEvidenceLookup::DerivedHit(
                ValidatedSpeakerEvidence {
                    segments: envelope.segments,
                },
            ));
        }

        let stored_raw = cache
            .get(
                self.raw_cache_key.as_str(),
                CacheTaskName::SpeakerDiarizationRawEvidence.as_str(),
                self.model_revision.as_str(),
            )
            .await?;
        if let Some(stored) = stored_raw {
            let envelope: StoredRawSpeakerEvidence =
                serde_json::from_value(stored).map_err(|error| {
                    SpeakerEvidenceCacheError::InvalidCachedEvidence(error.to_string())
                })?;
            let segments = envelope.validate_and_normalize_for(self)?;
            store_derived_evidence(self, cache, &segments).await?;
            return Ok(SpeakerEvidenceLookup::RawHit(ValidatedSpeakerEvidence {
                segments,
            }));
        }

        Ok(SpeakerEvidenceLookup::Miss(SpeakerEvidenceMiss {
            request: self.clone(),
            reason: SpeakerEvidenceMissReason::NotFound,
            _lease: lease,
        }))
    }

    #[cfg(test)]
    async fn store_unchecked_raw_for_test(
        &self,
        cache: &UtteranceCache,
        value: serde_json::Value,
    ) -> Result<(), SpeakerEvidenceCacheError> {
        cache
            .put(
                self.raw_cache_key.as_str(),
                CacheTaskName::SpeakerDiarizationRawEvidence.as_str(),
                self.model_revision.as_str(),
                env!("CARGO_PKG_VERSION"),
                &value,
            )
            .await?;
        Ok(())
    }

    #[cfg(test)]
    async fn store_unchecked_derived_for_test(
        &self,
        cache: &UtteranceCache,
        value: serde_json::Value,
    ) -> Result<(), SpeakerEvidenceCacheError> {
        cache
            .put(
                self.derived_cache_key.as_str(),
                CacheTaskName::SpeakerDiarizationSegments.as_str(),
                self.normalization_revision.as_str(),
                env!("CARGO_PKG_VERSION"),
                &value,
            )
            .await?;
        Ok(())
    }
}

fn derived_cache_key(
    raw_cache_key: &CacheKey,
    normalization_revision: &SpeakerNormalizationRevision,
) -> Result<CacheKey, serde_json::Error> {
    let material = DerivedSpeakerEvidenceKeyMaterial {
        schema_version: DERIVED_SPEAKER_EVIDENCE_SCHEMA_VERSION,
        raw_evidence_fingerprint: raw_cache_key.as_str(),
        normalization_revision: normalization_revision.as_str(),
    };
    serde_json::to_string(&material).map(|canonical| CacheKey::from_content(&canonical))
}

/// Result of checking durable speaker evidence.
///
/// A hit contains only replayable evidence. Only the miss variant contains the
/// state transition that can authorize an inference call.
#[derive(Debug)]
pub(crate) enum SpeakerEvidenceLookup {
    DerivedHit(ValidatedSpeakerEvidence),
    RawHit(ValidatedSpeakerEvidence),
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
    ) -> Result<SpeakerInferenceEvidenceV2, ServerError>;
}

/// How one request acquired validated speaker evidence.
#[derive(Debug)]
pub(crate) enum SpeakerEvidenceResolution {
    Replayed(ValidatedSpeakerEvidence),
    DerivedFromRaw(ValidatedSpeakerEvidence),
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
        SpeakerEvidenceLookup::DerivedHit(evidence) => {
            Ok(SpeakerEvidenceResolution::Replayed(evidence))
        }
        SpeakerEvidenceLookup::RawHit(evidence) => {
            Ok(SpeakerEvidenceResolution::DerivedFromRaw(evidence))
        }
        SpeakerEvidenceLookup::Miss(miss) => {
            let authorization = miss.authorize_billable_inference();
            let reason = authorization.reason();
            let raw_evidence = inference.infer(&authorization).await?;
            let evidence = authorization.commit(cache, raw_evidence).await?;
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
        raw_evidence: SpeakerInferenceEvidenceV2,
    ) -> Result<ValidatedSpeakerEvidence, SpeakerEvidenceCacheError> {
        validate_evidence_backend(&raw_evidence, self.request.backend)?;
        let segments = normalize_speaker_evidence_v2(&raw_evidence)
            .map_err(SpeakerEvidenceCacheError::InvalidCachedEvidence)?;
        validate_segments(&segments)?;
        let envelope = StoredRawSpeakerEvidence {
            schema_version: RAW_SPEAKER_EVIDENCE_SCHEMA_VERSION,
            request_fingerprint: self.request.raw_cache_key.to_string(),
            evidence: raw_evidence,
        };
        let value = serde_json::to_value(&envelope)?;
        cache
            .put(
                self.request.raw_cache_key.as_str(),
                CacheTaskName::SpeakerDiarizationRawEvidence.as_str(),
                self.request.model_revision.as_str(),
                env!("CARGO_PKG_VERSION"),
                &value,
            )
            .await?;
        store_derived_evidence(&self.request, cache, &segments).await?;
        self._lease.mark_committed();
        Ok(ValidatedSpeakerEvidence { segments })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredRawSpeakerEvidence {
    schema_version: u32,
    request_fingerprint: String,
    evidence: SpeakerInferenceEvidenceV2,
}

impl StoredRawSpeakerEvidence {
    fn validate_and_normalize_for(
        &self,
        request: &SpeakerEvidenceRequest,
    ) -> Result<Vec<SpeakerSegmentV2>, SpeakerEvidenceCacheError> {
        if self.schema_version != RAW_SPEAKER_EVIDENCE_SCHEMA_VERSION {
            return Err(SpeakerEvidenceCacheError::InvalidCachedEvidence(format!(
                "schema version {} does not match {}",
                self.schema_version, RAW_SPEAKER_EVIDENCE_SCHEMA_VERSION
            )));
        }
        if self.request_fingerprint != request.raw_cache_key.as_str() {
            return Err(SpeakerEvidenceCacheError::InvalidCachedEvidence(
                "request fingerprint does not match cache key".to_owned(),
            ));
        }
        validate_evidence_backend(&self.evidence, request.backend)?;
        let segments = normalize_speaker_evidence_v2(&self.evidence)
            .map_err(SpeakerEvidenceCacheError::InvalidCachedEvidence)?;
        validate_segments(&segments)?;
        Ok(segments)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredDerivedSpeakerEvidence {
    schema_version: u32,
    raw_evidence_fingerprint: String,
    normalization_revision: String,
    segments: Vec<SpeakerSegmentV2>,
}

impl StoredDerivedSpeakerEvidence {
    fn validate_for(
        &self,
        request: &SpeakerEvidenceRequest,
    ) -> Result<(), SpeakerEvidenceCacheError> {
        if self.schema_version != DERIVED_SPEAKER_EVIDENCE_SCHEMA_VERSION {
            return Err(SpeakerEvidenceCacheError::InvalidCachedEvidence(format!(
                "derived schema version {} does not match {}",
                self.schema_version, DERIVED_SPEAKER_EVIDENCE_SCHEMA_VERSION
            )));
        }
        if self.raw_evidence_fingerprint != request.raw_cache_key.as_str() {
            return Err(SpeakerEvidenceCacheError::InvalidCachedEvidence(
                "derived raw-evidence fingerprint does not match request".to_owned(),
            ));
        }
        if self.normalization_revision != request.normalization_revision.as_str() {
            return Err(SpeakerEvidenceCacheError::InvalidCachedEvidence(
                "derived normalization revision does not match request".to_owned(),
            ));
        }
        validate_segments(&self.segments)
    }
}

async fn store_derived_evidence(
    request: &SpeakerEvidenceRequest,
    cache: &UtteranceCache,
    segments: &[SpeakerSegmentV2],
) -> Result<(), SpeakerEvidenceCacheError> {
    validate_segments(segments)?;
    let envelope = StoredDerivedSpeakerEvidence {
        schema_version: DERIVED_SPEAKER_EVIDENCE_SCHEMA_VERSION,
        raw_evidence_fingerprint: request.raw_cache_key.to_string(),
        normalization_revision: request.normalization_revision.as_str().to_owned(),
        segments: segments.to_vec(),
    };
    cache
        .put(
            request.derived_cache_key.as_str(),
            CacheTaskName::SpeakerDiarizationSegments.as_str(),
            request.normalization_revision.as_str(),
            env!("CARGO_PKG_VERSION"),
            &serde_json::to_value(envelope)?,
        )
        .await?;
    Ok(())
}

fn validate_evidence_backend(
    evidence: &SpeakerInferenceEvidenceV2,
    backend: SpeakerBackendV2,
) -> Result<(), SpeakerEvidenceCacheError> {
    let matches = matches!(
        (evidence, backend),
        (
            SpeakerInferenceEvidenceV2::PyannoteAi { .. },
            SpeakerBackendV2::PyannoteAi
        ) | (
            SpeakerInferenceEvidenceV2::Pyannote { .. },
            SpeakerBackendV2::Pyannote
        ) | (
            SpeakerInferenceEvidenceV2::Nemo { .. },
            SpeakerBackendV2::Nemo
        )
    );
    if matches {
        Ok(())
    } else {
        Err(SpeakerEvidenceCacheError::InvalidCachedEvidence(format!(
            "speaker evidence provenance does not match requested backend {backend:?}"
        )))
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
    use crate::types::worker_v2::{
        SpeakerBackendV2, SpeakerInferenceEvidenceV2, SpeakerProviderJobIdV2, SpeakerSegmentV2,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingSpeakerService {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl SpeakerEvidenceInference for CountingSpeakerService {
        async fn infer(
            &self,
            _authorization: &SpeakerInferenceAuthorization,
        ) -> Result<SpeakerInferenceEvidenceV2, ServerError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(raw_evidence(&[segment("SPEAKER_00")]))
        }
    }

    fn segment(speaker: &str) -> SpeakerSegmentV2 {
        SpeakerSegmentV2 {
            start_ms: DurationMs(0),
            end_ms: DurationMs(750),
            speaker: speaker.to_owned(),
        }
    }

    fn raw_evidence(segments: &[SpeakerSegmentV2]) -> SpeakerInferenceEvidenceV2 {
        SpeakerInferenceEvidenceV2::PyannoteAi {
            job_id: SpeakerProviderJobIdV2::from("job-test"),
            output: serde_json::from_value(serde_json::json!({
                "exclusiveDiarization": segments
                    .iter()
                    .map(|segment| serde_json::json!({
                        "start": segment.start_ms.0 as f64 / 1000.0,
                        "end": segment.end_ms.0 as f64 / 1000.0,
                        "speaker": segment.speaker,
                    }))
                    .collect::<Vec<_>>()
            }))
            .expect("provider output object"),
            warning: None,
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
            SpeakerEvidenceLookup::DerivedHit(_) | SpeakerEvidenceLookup::RawHit(_) => {
                panic!("empty cache must miss")
            }
            SpeakerEvidenceLookup::Miss(miss) => miss,
        };
        let billable = miss.authorize_billable_inference();
        let committed = billable
            .commit(&cache, raw_evidence(&[segment("SPEAKER_00")]))
            .await
            .expect("commit");
        assert_eq!(committed.segments(), &[segment("SPEAKER_00")]);

        let hit = request
            .lookup(&cache, CachePolicy::UseCache)
            .await
            .expect("lookup hit");
        match hit {
            SpeakerEvidenceLookup::DerivedHit(evidence) => {
                assert_eq!(evidence.segments(), &[segment("SPEAKER_00")]);
            }
            SpeakerEvidenceLookup::RawHit(_) => {
                panic!("committed derived evidence should hit directly")
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
            SpeakerEvidenceLookup::DerivedHit(_) | SpeakerEvidenceLookup::RawHit(_) => {
                panic!("empty cache must miss")
            }
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
            .commit(&cache, raw_evidence(&[segment("SPEAKER_00")]))
            .await
            .expect("commit first inference");
        let second_lookup = second.await.expect("second task").expect("second lookup");
        assert!(matches!(
            second_lookup,
            SpeakerEvidenceLookup::DerivedHit(_)
        ));
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
    async fn a_new_normalizer_revision_reuses_raw_evidence_without_another_service_call() {
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
        let revised_request = request
            .clone()
            .with_normalization_revision_for_test("speaker-normalizer-v-next");
        let service = CountingSpeakerService {
            calls: AtomicUsize::new(0),
        };

        resolve_speaker_evidence(&request, &cache, CachePolicy::UseCache, &service)
            .await
            .expect("cold resolution");
        let replay =
            resolve_speaker_evidence(&revised_request, &cache, CachePolicy::UseCache, &service)
                .await
                .expect("re-normalize raw evidence");

        assert!(matches!(
            replay,
            SpeakerEvidenceResolution::DerivedFromRaw(_)
        ));
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
            .store_unchecked_raw_for_test(&cache, serde_json::json!({"evidence": "invalid"}))
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
            SpeakerEvidenceLookup::DerivedHit(_) | SpeakerEvidenceLookup::RawHit(_) => {
                panic!("empty cache must miss")
            }
            SpeakerEvidenceLookup::Miss(miss) => miss,
        };
        miss.authorize_billable_inference()
            .commit(&cache, raw_evidence(&[segment("SPEAKER_00")]))
            .await
            .expect("commit");

        let refresh = request
            .lookup(&cache, CachePolicy::SkipCache)
            .await
            .expect("refresh lookup");
        let authorization = match refresh {
            SpeakerEvidenceLookup::DerivedHit(_) | SpeakerEvidenceLookup::RawHit(_) => {
                panic!("refresh must not hit")
            }
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
            "schema_version": DERIVED_SPEAKER_EVIDENCE_SCHEMA_VERSION,
            "raw_evidence_fingerprint": request.raw_cache_key.as_str(),
            "normalization_revision": request.normalization_revision.as_str(),
            "segments": [{
                "start_ms": 900,
                "end_ms": 100,
                "speaker": "SPEAKER_00"
            }]
        });
        request
            .store_unchecked_derived_for_test(&cache, invalid)
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
            SpeakerEvidenceLookup::DerivedHit(_) | SpeakerEvidenceLookup::RawHit(_) => {
                panic!("empty cache must miss")
            }
            SpeakerEvidenceLookup::Miss(miss) => miss,
        };
        miss.authorize_billable_inference()
            .commit(&cache, raw_evidence(std::slice::from_ref(&zero_duration)))
            .await
            .expect("worker-valid zero-duration evidence should commit");

        let hit = request
            .lookup(&cache, CachePolicy::UseCache)
            .await
            .expect("replay");
        let SpeakerEvidenceLookup::DerivedHit(evidence) = hit else {
            panic!("committed evidence must replay");
        };
        assert_eq!(evidence.segments(), &[zero_duration]);
    }
}
