//! Configuration for the native Whisper inference path.

use std::path::PathBuf;

/// Where the model file lives + how to invoke whisper.cpp's decoder.
///
/// Defaults are platform-aware:
/// - On macOS the model directory is searched for a sibling
///   `<model>-encoder.mlmodelc` bundle; if present and the
///   `whisper-rs-coreml` feature is enabled, CoreML acceleration kicks in
///   automatically.
/// - On Linux/Windows the path-suffix CoreML lookup is irrelevant and the
///   model loads CPU-only (or CUDA if `whisper-rs-cuda` feature is on).
#[derive(Debug, Clone)]
pub struct WhisperNativeConfig {
    /// Absolute path to a ggml-format model file (`.bin`).
    pub model_path: PathBuf,
    /// Number of CPU threads to use during decoding. `None` lets
    /// whisper.cpp pick a sane default (typically 4 or `min(4, ncpu)`).
    pub n_threads: Option<i32>,
    /// Whisper's `--max-context` flag. Setting this to `Some(0)` resets
    /// the decoder's prompt context between 30-second chunks, mirroring
    /// the Python transformers pipeline's chunk isolation. Found in
    /// pilot testing to suppress end-of-audio token-loop hallucinations
    /// AND speed runs up by ~45% (cross-chunk attention is expensive).
    /// `None` uses whisper.cpp's default (-1, full context).
    pub max_context: Option<i32>,
    /// Translate to English (`--translate`). Set false for transcription.
    pub translate: bool,
}

impl WhisperNativeConfig {
    /// New config pointing at the given model file. Other fields take
    /// the recommended pilot defaults: 8 threads, `--max-context 0`,
    /// transcribe (not translate).
    pub fn for_model(model_path: PathBuf) -> Self {
        Self {
            model_path,
            n_threads: Some(8),
            max_context: Some(0),
            translate: false,
        }
    }

    /// Resolve the model path from the `BATCHALIGN_WHISPER_RS_MODEL` env
    /// var. Returns `None` if unset; callers can fall back to a default
    /// or surface a clear error.
    pub fn from_env() -> Option<Self> {
        std::env::var_os("BATCHALIGN_WHISPER_RS_MODEL").map(|p| Self::for_model(PathBuf::from(p)))
    }
}
