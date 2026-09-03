//! Rust-side request builder and response reader for speaker embedding.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use thiserror::Error;

use crate::chat_ops::speaker_identity::{
    EmbeddingRequest, EmbeddingResponse, NotAPreparedDecode, NotAnEmbedding, PreparedPcm,
    SpanOutcome, SpeakerEmbedding,
};
use crate::types::worker_v2::{
    ArtifactRefV2, ExecuteRequestV2, ExecuteResponseV2, InferenceTaskV2, SpeakerEmbeddingBackendV2,
    SpeakerEmbeddingOutcomeV2, SpeakerEmbeddingRequestV2, SpeakerEmbeddingSpanIdV2,
    SpeakerEmbeddingSpanV2, TaskRequestV2, TaskResultV2, WorkerArtifactIdV2, WorkerRequestIdV2,
};

use super::artifacts_v2::{PreparedArtifactErrorV2, PreparedArtifactStoreV2};
use super::execute_result_v2::require_success_result;

/// Process-unique ids for one embedding execution.
///
/// Same reason as the ASR and speaker sequences: shared-GPU-worker response
/// routing is keyed by `request_id`, so a duplicate orphans a response.
static EMBEDDING_REQUEST_SEQUENCE_V2: AtomicU64 = AtomicU64::new(1);

/// Errors produced while building a live V2 embedding request.
#[derive(Debug, Error)]
pub enum SpeakerEmbeddingRequestBuildErrorV2 {
    /// The request referenced an empty audio path.
    #[error("worker protocol V2 speaker embedding request is missing an audio path")]
    MissingAudioPath,
    /// Rust-owned prepared-artifact creation failed.
    #[error(transparent)]
    Artifact(#[from] PreparedArtifactErrorV2),
    /// The prepared decode cannot index anything.
    #[error(transparent)]
    Decode(#[from] NotAPreparedDecode),
}

/// One decoded recording, and the description of it the worker will receive.
///
/// The two travel as one value because a frame span is meaningless against a
/// different decode. Returning them separately would let a caller locate spans
/// in one decode and send them against another.
#[derive(Debug, Clone)]
pub struct PreparedRecording {
    descriptor: crate::types::worker_v2::PreparedAudioRefV2,
    /// The decode every span must be located against.
    pub prepared: PreparedPcm,
    request_id: WorkerRequestIdV2,
}

/// Decode one recording to canonical mono PCM, ONCE per file.
///
/// Separate from [`PreparedRecording::request_for`] because span locating needs
/// the decode's shape before any request can exist, and decoding twice would
/// both cost a second ffmpeg pass and create two decodes that a reader could
/// no longer tell apart.
pub async fn prepare_recording_for_embedding(
    store: &PreparedArtifactStoreV2,
    audio_path: &Path,
) -> Result<PreparedRecording, SpeakerEmbeddingRequestBuildErrorV2> {
    if audio_path.as_os_str().is_empty() {
        return Err(SpeakerEmbeddingRequestBuildErrorV2::MissingAudioPath);
    }
    let sequence = EMBEDDING_REQUEST_SEQUENCE_V2.fetch_add(1, Ordering::Relaxed);
    let request_id = WorkerRequestIdV2::from(format!("speaker-embedding-v2-request-{sequence}"));
    let audio_ref_id = WorkerArtifactIdV2::from(format!("speaker-embedding-v2-audio-{sequence}"));

    let descriptor = store
        .prepare_audio_file_f32le(&audio_ref_id, audio_path)
        .await?;
    // Built from the descriptor that just described this decode, so its rate
    // and its length cannot arrive from two different places.
    let prepared = PreparedPcm::new(descriptor.sample_rate_hz.0, descriptor.frame_count.0)?;

    Ok(PreparedRecording {
        descriptor,
        prepared,
        request_id,
    })
}

impl PreparedRecording {
    /// Build the request that asks about `spans` of THIS decode.
    ///
    /// Takes the already-located [`EmbeddingRequest`], which only
    /// `chat_ops::speaker_identity::identify_speakers` builds and which only
    /// [`PreparedPcm::locate`] can put spans into, so a caller cannot ask the
    /// worker about frames nobody located.
    #[must_use]
    pub fn request_for(&self, spans: &EmbeddingRequest) -> ExecuteRequestV2 {
        let wire_spans = spans
            .spans()
            .iter()
            .map(|span| SpeakerEmbeddingSpanV2 {
                span_id: SpeakerEmbeddingSpanIdV2::from(span.span_id.as_str()),
                start_frame: crate::types::worker_v2::FrameCountV2(span.frames.start()),
                end_frame: crate::types::worker_v2::FrameCountV2(span.frames.end()),
            })
            .collect();

        ExecuteRequestV2 {
            request_id: self.request_id.clone(),
            task: InferenceTaskV2::SpeakerEmbedding,
            payload: TaskRequestV2::SpeakerEmbedding(SpeakerEmbeddingRequestV2 {
                backend: SpeakerEmbeddingBackendV2::Pyannote,
                audio_ref_id: WorkerArtifactIdV2::from(self.descriptor.id.as_ref()),
                spans: wire_spans,
            }),
            attachments: vec![ArtifactRefV2::PreparedAudio(self.descriptor.clone())],
        }
    }
}

/// Why a worker response is not a usable set of embeddings.
#[derive(Debug, Error)]
pub enum SpeakerEmbeddingResultParseError {
    /// The response was not a successful embedding result.
    #[error("{0}")]
    UnexpectedPayload(String),
    /// A returned vector is not an embedding.
    #[error("worker protocol V2 speaker embedding response carried an unusable vector: {0}")]
    Vector(#[from] NotAnEmbedding),
}

/// Read one V2 embedding response into the typed outcomes the control plane
/// reasons with.
///
/// The width every vector is checked against is the one the RESPONSE declares,
/// not a constant here: the worker reports the loaded model's own dimension, so
/// a constant on this side would go on agreeing with a model that had moved.
pub fn parse_speaker_embedding_response_v2(
    response: &ExecuteResponseV2,
) -> Result<EmbeddingResponse, SpeakerEmbeddingResultParseError> {
    let result = require_success_result(response, "speaker embedding")
        .map_err(|failure| SpeakerEmbeddingResultParseError::UnexpectedPayload(failure.into()))?;
    let TaskResultV2::SpeakerEmbeddingResult(result) = result else {
        return Err(SpeakerEmbeddingResultParseError::UnexpectedPayload(
            format!(
                "worker protocol V2 speaker embedding response returned {} data",
                result_kind(result)
            ),
        ));
    };

    let declared_width = result.dimension as usize;
    let mut outcomes = std::collections::BTreeMap::new();
    for span in &result.spans {
        let outcome = match &span.outcome {
            SpeakerEmbeddingOutcomeV2::Embedded { vector } => SpanOutcome::Embedded(
                SpeakerEmbedding::from_worker(vector.clone(), declared_width)?,
            ),
            SpeakerEmbeddingOutcomeV2::TooShort { frame_count } => SpanOutcome::TooShort {
                frames: frame_count.0,
            },
        };
        outcomes.insert(span.span_id.as_ref().to_owned(), outcome);
    }

    Ok(EmbeddingResponse {
        outcomes,
        minimum_frames: result.minimum_frames.0,
        dimension: result.dimension,
        // Filled in by the caller, which owns the pinned graph identity. The
        // wire deliberately does not carry it: the revision is a property of
        // the manifest this binary ships, so asking the worker would be asking
        // a second party about a fact we already hold.
        model_revision: String::new(),
    })
}

/// A short label for a result variant, for the "returned X data" refusals.
fn result_kind(result: &TaskResultV2) -> &'static str {
    match result {
        TaskResultV2::WhisperChunkResult(_) => "ASR chunk",
        TaskResultV2::MonologueAsrResult(_) => "monologue ASR",
        TaskResultV2::WhisperTokenTimingResult(_) => "forced-alignment token",
        TaskResultV2::IndexedWordTimingResult(_) => "indexed timing",
        TaskResultV2::MorphosyntaxResult(_) => "morphosyntax",
        TaskResultV2::UtsegResult(_) => "utterance-segmentation",
        TaskResultV2::TranslationResult(_) => "translation",
        TaskResultV2::CorefResult(_) => "coreference",
        TaskResultV2::SpeakerResult(_) => "speaker diarization",
        TaskResultV2::SpeakerEmbeddingResult(_) => "speaker embedding",
        TaskResultV2::OpensmileResult(_) => "openSMILE feature",
        TaskResultV2::AvqiResult(_) => "AVQI feature",
    }
}
