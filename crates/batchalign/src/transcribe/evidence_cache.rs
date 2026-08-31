//! Durable evidence captured at paid inference boundaries.

use std::path::{Path, PathBuf};

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
const SPEAKER_SEGMENT_DIGEST_REVISION: &str = "speaker-segments-blake3-v1";

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
    fn from_bytes(bytes: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(bytes);
        Self(hasher.finalize().to_hex().to_string())
    }

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

/// Source media whose exact bytes established the speaker-evidence identity.
///
/// The source path is retained only so an authorized cache miss can reread and
/// verify those bytes. It is never part of the semantic cache key.
#[derive(Debug, Clone)]
struct PreparedSpeakerProviderMedia {
    source_path: PathBuf,
    source_digest: SpeakerAudioSourceDigest,
}

impl PreparedSpeakerProviderMedia {
    async fn from_source(path: &Path) -> Result<Self, SpeakerEvidenceCacheError> {
        Ok(Self {
            source_path: path.to_owned(),
            source_digest: SpeakerAudioSourceDigest::from_path(path).await?,
        })
    }

    async fn verify(self) -> Result<Vec<u8>, SpeakerEvidenceCacheError> {
        let bytes = tokio::fs::read(&self.source_path).await?;
        if SpeakerAudioSourceDigest::from_bytes(&bytes) != self.source_digest {
            return Err(SpeakerEvidenceCacheError::ProviderMediaDrift(
                self.source_path,
            ));
        }
        Ok(bytes)
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
    provider_media: PreparedSpeakerProviderMedia,
    backend: SpeakerBackendV2,
    expected_speakers: NumSpeakers,
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
        let provider_media = PreparedSpeakerProviderMedia::from_source(audio_path).await?;
        let key_material = SpeakerEvidenceKeyMaterial {
            schema_version: RAW_SPEAKER_EVIDENCE_SCHEMA_VERSION,
            audio_source_blake3: &provider_media.source_digest,
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
            provider_media,
            backend,
            expected_speakers,
            model_revision: model_revision.clone(),
            normalization_revision,
        })
    }

    pub(crate) fn cache_key(&self) -> &CacheKey {
        &self.raw_cache_key
    }

    fn trace_seed(&self) -> SpeakerEvidenceTraceSeed {
        SpeakerEvidenceTraceSeed {
            trace_schema_version: 1,
            source_media_blake3: self.provider_media.source_digest.0.clone(),
            audio_preparation_revision: SPEAKER_AUDIO_PREPARATION_REVISION,
            backend: self.backend,
            expected_speakers: self.expected_speakers.0,
            model_revision: self.model_revision.as_str().to_owned(),
            raw_evidence_key: self.raw_cache_key.to_string(),
            normalization_revision: self.normalization_revision.as_str().to_owned(),
            derived_evidence_key: self.derived_cache_key.to_string(),
        }
    }

    #[cfg(test)]
    fn with_normalization_revision_for_test(mut self, revision: &str) -> Self {
        self.normalization_revision = SpeakerNormalizationRevision::for_test(revision);
        self.derived_cache_key =
            derived_cache_key(&self.raw_cache_key, &self.normalization_revision)
                .expect("test normalization revision should serialize");
        self
    }

    async fn lookup(
        &self,
        cache: &UtteranceCache,
        policy: CachePolicy,
    ) -> Result<SpeakerEvidenceLookup, SpeakerEvidenceCacheError> {
        let lease = InferenceLease::acquire(&self.raw_cache_key, cache).await;
        match policy {
            CachePolicy::SkipCache if !lease.observed_commit_while_waiting() => {
                return Ok(SpeakerEvidenceLookup::Miss(SpeakerEvidenceMiss {
                    request: self.clone(),
                    reason: SpeakerEvidenceMissReason::ForcedRefresh,
                    _lease: lease,
                }));
            }
            CachePolicy::UseCache | CachePolicy::RequireCache | CachePolicy::SkipCache => {}
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
                ValidatedSpeakerEvidence::new(envelope.segments),
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
            return Ok(SpeakerEvidenceLookup::RawHit(
                ValidatedSpeakerEvidence::new(segments),
            ));
        }

        match policy {
            CachePolicy::RequireCache => {
                return Err(SpeakerEvidenceCacheError::RequiredEvidenceMissing(
                    self.raw_cache_key.to_string(),
                ));
            }
            CachePolicy::UseCache | CachePolicy::SkipCache => {}
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
enum SpeakerEvidenceLookup {
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
        run: VerifiedSpeakerEvidenceRun,
    ) -> Result<SpeakerInferenceEvidenceV2, ServerError>;
}

/// Stable causal origin of the speaker evidence used for one projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpeakerEvidenceSource {
    ReplayedDerived,
    DerivedFromRaw,
    Inferred(SpeakerEvidenceMissReason),
}

/// Stable revision of the local raw-evidence-to-segment projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub(crate) enum SpeakerProjectionRevision {
    #[serde(rename = "speaker-evidence-to-segments-v1")]
    SegmentsV1,
}

/// Causal result of the cache-or-infer decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SpeakerEvidenceCacheOutcome {
    ReplayedDerived,
    DerivedFromRaw,
    InferredNotFound,
    InferredForcedRefresh,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct SpeakerEvidenceTraceSeed {
    trace_schema_version: u32,
    source_media_blake3: String,
    audio_preparation_revision: &'static str,
    backend: SpeakerBackendV2,
    expected_speakers: u32,
    model_revision: String,
    raw_evidence_key: String,
    normalization_revision: String,
    derived_evidence_key: String,
}

/// Content identity of the exact normalized segments used downstream.
///
/// The digest is constructed only by [`ValidatedSpeakerEvidence`], after
/// segment geometry has passed validation. It is independent of JSON
/// formatting and cache location.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct SpeakerSegmentsDigest(String);

impl SpeakerSegmentsDigest {
    fn from_segments(segments: &[SpeakerSegmentV2]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(SPEAKER_SEGMENT_DIGEST_REVISION.as_bytes());
        hasher.update(&[0]);
        hasher.update(&(segments.len() as u64).to_le_bytes());
        for segment in segments {
            hasher.update(&segment.start_ms.0.to_le_bytes());
            hasher.update(&segment.end_ms.0.to_le_bytes());
            let speaker = segment.speaker.as_bytes();
            hasher.update(&(speaker.len() as u64).to_le_bytes());
            hasher.update(speaker);
        }
        Self(hasher.finalize().to_hex().to_string())
    }
}

/// Versioned causal receipt for one dedicated-speaker projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SpeakerEvidenceTrace {
    #[serde(flatten)]
    request: SpeakerEvidenceTraceSeed,
    cache_outcome: SpeakerEvidenceCacheOutcome,
    projection_revision: SpeakerProjectionRevision,
    segment_digest_revision: &'static str,
    projected_segment_count: usize,
    projected_segments_blake3: SpeakerSegmentsDigest,
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SpeakerEvidenceSemanticProjection<'a> {
    request: &'a SpeakerEvidenceTraceSeed,
    projection_revision: SpeakerProjectionRevision,
    projected_segment_count: usize,
    projected_segments_blake3: &'a SpeakerSegmentsDigest,
}

impl SpeakerEvidenceTrace {
    #[cfg(test)]
    pub(crate) fn cache_outcome(&self) -> SpeakerEvidenceCacheOutcome {
        self.cache_outcome
    }

    #[cfg(test)]
    pub(crate) fn semantic_projection(&self) -> SpeakerEvidenceSemanticProjection<'_> {
        SpeakerEvidenceSemanticProjection {
            request: &self.request,
            projection_revision: self.projection_revision,
            projected_segment_count: self.projected_segment_count,
            projected_segments_blake3: &self.projected_segments_blake3,
        }
    }
}

/// How one request acquired validated speaker evidence.
#[derive(Debug)]
pub(crate) struct SpeakerEvidenceResolution {
    evidence: ValidatedSpeakerEvidence,
    source: SpeakerEvidenceSource,
    trace_seed: SpeakerEvidenceTraceSeed,
}

impl SpeakerEvidenceResolution {
    pub(crate) fn source(&self) -> SpeakerEvidenceSource {
        self.source
    }

    pub(crate) fn segments(&self) -> &[SpeakerSegmentV2] {
        self.evidence.segments()
    }

    pub(crate) fn into_segments(self) -> Vec<SpeakerSegmentV2> {
        self.evidence.into_segments()
    }

    pub(crate) fn trace(
        &self,
        projection_revision: SpeakerProjectionRevision,
    ) -> SpeakerEvidenceTrace {
        let cache_outcome = match self.source {
            SpeakerEvidenceSource::ReplayedDerived => SpeakerEvidenceCacheOutcome::ReplayedDerived,
            SpeakerEvidenceSource::DerivedFromRaw => SpeakerEvidenceCacheOutcome::DerivedFromRaw,
            SpeakerEvidenceSource::Inferred(SpeakerEvidenceMissReason::NotFound) => {
                SpeakerEvidenceCacheOutcome::InferredNotFound
            }
            SpeakerEvidenceSource::Inferred(SpeakerEvidenceMissReason::ForcedRefresh) => {
                SpeakerEvidenceCacheOutcome::InferredForcedRefresh
            }
        };
        SpeakerEvidenceTrace {
            request: self.trace_seed.clone(),
            cache_outcome,
            projection_revision,
            segment_digest_revision: SPEAKER_SEGMENT_DIGEST_REVISION,
            projected_segment_count: self.evidence.segments.len(),
            projected_segments_blake3: self.evidence.segments_digest.clone(),
        }
    }
}

/// Resolve evidence through the same cache-or-infer state machine in tests and
/// production.
pub(crate) async fn resolve_speaker_evidence<I: SpeakerEvidenceInference>(
    request: &SpeakerEvidenceRequest,
    cache: &UtteranceCache,
    policy: CachePolicy,
    inference: &I,
) -> Result<SpeakerEvidenceResolution, SpeakerEvidenceResolutionError> {
    let trace_seed = request.trace_seed();
    match request.lookup(cache, policy).await? {
        SpeakerEvidenceLookup::DerivedHit(evidence) => Ok(SpeakerEvidenceResolution {
            evidence,
            source: SpeakerEvidenceSource::ReplayedDerived,
            trace_seed,
        }),
        SpeakerEvidenceLookup::RawHit(evidence) => Ok(SpeakerEvidenceResolution {
            evidence,
            source: SpeakerEvidenceSource::DerivedFromRaw,
            trace_seed,
        }),
        SpeakerEvidenceLookup::Miss(miss) => {
            let authorization = miss.authorize_billable_inference();
            let (run, permit) = authorization.into_run();
            let reason = permit.reason();
            let verified_run = run.verify().await?;
            let raw_evidence = inference.infer(verified_run).await?;
            let evidence = permit.commit(cache, raw_evidence).await?;
            Ok(SpeakerEvidenceResolution {
                evidence,
                source: SpeakerEvidenceSource::Inferred(reason),
                trace_seed,
            })
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
    segments_digest: SpeakerSegmentsDigest,
}

impl ValidatedSpeakerEvidence {
    fn new(segments: Vec<SpeakerSegmentV2>) -> Self {
        let segments_digest = SpeakerSegmentsDigest::from_segments(&segments);
        Self {
            segments,
            segments_digest,
        }
    }

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
struct SpeakerEvidenceMiss {
    request: SpeakerEvidenceRequest,
    reason: SpeakerEvidenceMissReason,
    _lease: InferenceLease,
}

impl SpeakerEvidenceMiss {
    fn authorize_billable_inference(self) -> SpeakerInferenceAuthorization {
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
struct SpeakerInferenceAuthorization {
    request: SpeakerEvidenceRequest,
    reason: SpeakerEvidenceMissReason,
    _lease: InferenceLease,
}

/// Single-use capability accepted by the live speaker inference boundary.
///
/// Its field is private, so only a proven cache miss can create it. The
/// inference trait consumes the capability, which rules out issuing a second
/// paid request from the same authorization.
#[derive(Debug)]
struct AuthorizedSpeakerEvidenceRun {
    provider_media: PreparedSpeakerProviderMedia,
    backend: SpeakerBackendV2,
    expected_speakers: NumSpeakers,
}

/// Exact source bytes and request semantics admitted for one speaker run.
///
/// Construction rereads and verifies the source after cache authorization.
/// The live adapter consumes these bytes, not a separately supplied path.
pub(crate) struct VerifiedSpeakerEvidenceRun {
    source_bytes: Vec<u8>,
    backend: SpeakerBackendV2,
    expected_speakers: NumSpeakers,
}

impl VerifiedSpeakerEvidenceRun {
    pub(super) fn into_worker_input(self) -> (Vec<u8>, SpeakerBackendV2, NumSpeakers) {
        (self.source_bytes, self.backend, self.expected_speakers)
    }
}

impl AuthorizedSpeakerEvidenceRun {
    async fn verify(self) -> Result<VerifiedSpeakerEvidenceRun, SpeakerEvidenceCacheError> {
        Ok(VerifiedSpeakerEvidenceRun {
            source_bytes: self.provider_media.verify().await?,
            backend: self.backend,
            expected_speakers: self.expected_speakers,
        })
    }
}

impl SpeakerInferenceAuthorization {
    fn into_run(self) -> (AuthorizedSpeakerEvidenceRun, SpeakerEvidenceCommitPermit) {
        let Self {
            request,
            reason,
            _lease,
        } = self;
        let run = AuthorizedSpeakerEvidenceRun {
            provider_media: request.provider_media.clone(),
            backend: request.backend,
            expected_speakers: request.expected_speakers,
        };
        (
            run,
            SpeakerEvidenceCommitPermit {
                request,
                reason,
                _lease,
            },
        )
    }
}

/// Capability to durably commit exactly the evidence produced by one
/// authorized speaker-inference run.
#[derive(Debug)]
struct SpeakerEvidenceCommitPermit {
    request: SpeakerEvidenceRequest,
    reason: SpeakerEvidenceMissReason,
    _lease: InferenceLease,
}

impl SpeakerEvidenceCommitPermit {
    fn reason(&self) -> SpeakerEvidenceMissReason {
        self.reason
    }

    async fn commit(
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
        Ok(ValidatedSpeakerEvidence::new(segments))
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
    #[error("required speaker evidence is missing for cache key {0}")]
    RequiredEvidenceMissing(String),
    #[error("speaker source media changed after speaker evidence preparation: {0}")]
    ProviderMediaDrift(PathBuf),
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
            _run: VerifiedSpeakerEvidenceRun,
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
        let authorization = miss.authorize_billable_inference();
        let (run, permit) = authorization.into_run();
        // The live boundary must consume this capability.  Keeping commit
        // permission separate means the inference adapter cannot reuse the
        // same cache miss for a second paid call.
        let _: AuthorizedSpeakerEvidenceRun = run;
        let committed = permit
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
        let first_authorization = match request
            .lookup(&cache, CachePolicy::UseCache)
            .await
            .expect("first lookup")
        {
            SpeakerEvidenceLookup::DerivedHit(_) | SpeakerEvidenceLookup::RawHit(_) => {
                panic!("empty cache must miss")
            }
            SpeakerEvidenceLookup::Miss(miss) => miss.authorize_billable_inference(),
        };
        let (_run, first_permit) = first_authorization.into_run();

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

        first_permit
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
        let cold_trace = cold.trace(SpeakerProjectionRevision::SegmentsV1);
        assert_eq!(
            cold_trace.cache_outcome(),
            SpeakerEvidenceCacheOutcome::InferredNotFound
        );
        drop(cache);
        let reopened_cache = UtteranceCache::sqlite(Some(cache_dir))
            .await
            .expect("reopened cache");
        let warm =
            resolve_speaker_evidence(&request, &reopened_cache, CachePolicy::UseCache, &service)
                .await
                .expect("warm resolution");
        let warm_trace = warm.trace(SpeakerProjectionRevision::SegmentsV1);
        assert_eq!(
            warm_trace.cache_outcome(),
            SpeakerEvidenceCacheOutcome::ReplayedDerived
        );
        assert_eq!(
            cold_trace.semantic_projection(),
            warm_trace.semantic_projection()
        );
        let trace_json = serde_json::to_value(&cold_trace).expect("speaker trace JSON");
        assert_eq!(trace_json["trace_schema_version"], 1);
        assert_eq!(trace_json["backend"], "pyannote_ai");
        assert_eq!(trace_json["expected_speakers"], 2);
        assert_eq!(
            trace_json["audio_preparation_revision"],
            SPEAKER_AUDIO_PREPARATION_REVISION
        );
        assert_eq!(
            trace_json["projection_revision"],
            "speaker-evidence-to-segments-v1"
        );
        assert_eq!(
            trace_json["segment_digest_revision"],
            SPEAKER_SEGMENT_DIGEST_REVISION
        );
        assert_eq!(trace_json["projected_segment_count"], 1);
        assert_eq!(
            trace_json["projected_segments_blake3"]
                .as_str()
                .expect("segment digest string")
                .len(),
            64
        );
        assert!(trace_json.get("source_path").is_none());
        assert_eq!(service.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn segment_projection_digest_changes_with_timing_or_speaker() {
        let baseline = ValidatedSpeakerEvidence::new(vec![segment("SPEAKER_00")]);
        let mut shifted_segment = segment("SPEAKER_00");
        shifted_segment.end_ms = DurationMs(1_001);
        let shifted = ValidatedSpeakerEvidence::new(vec![shifted_segment]);
        let relabeled = ValidatedSpeakerEvidence::new(vec![segment("SPEAKER_01")]);

        assert_ne!(baseline.segments_digest, shifted.segments_digest);
        assert_ne!(baseline.segments_digest, relabeled.segments_digest);
    }

    #[tokio::test]
    async fn source_drift_after_cache_identity_refuses_speaker_inference() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let audio = tempdir.path().join("audio.wav");
        tokio::fs::write(&audio, b"bytes used for the cache identity")
            .await
            .expect("write original audio");
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
        tokio::fs::write(&audio, b"different bytes at inference time")
            .await
            .expect("replace audio");
        let service = CountingSpeakerService {
            calls: AtomicUsize::new(0),
        };

        let error = resolve_speaker_evidence(&request, &cache, CachePolicy::UseCache, &service)
            .await
            .expect_err("changed source bytes must not cross the inference boundary");

        assert!(
            error
                .to_string()
                .contains("changed after speaker evidence preparation")
        );
        assert_eq!(service.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn required_cache_refuses_cold_speaker_inference() {
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
        let service = CountingSpeakerService {
            calls: AtomicUsize::new(0),
        };

        let error = resolve_speaker_evidence(&request, &cache, CachePolicy::RequireCache, &service)
            .await
            .expect_err("a cold required-cache lookup must refuse inference");

        assert!(error.to_string().contains("required speaker evidence"));
        assert_eq!(service.calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn required_cache_replays_warm_speaker_evidence() {
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
        let service = CountingSpeakerService {
            calls: AtomicUsize::new(0),
        };
        resolve_speaker_evidence(&request, &cache, CachePolicy::UseCache, &service)
            .await
            .expect("seed evidence");

        let replay =
            resolve_speaker_evidence(&request, &cache, CachePolicy::RequireCache, &service)
                .await
                .expect("required-cache replay");

        assert_eq!(replay.source(), SpeakerEvidenceSource::ReplayedDerived);
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

        assert_eq!(replay.source(), SpeakerEvidenceSource::DerivedFromRaw);
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
        let (_run, permit) = miss.authorize_billable_inference().into_run();
        permit
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
        let (_run, permit) = authorization.into_run();
        assert_eq!(permit.reason(), SpeakerEvidenceMissReason::ForcedRefresh);
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
        let (_run, permit) = miss.authorize_billable_inference().into_run();
        permit
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
