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

    /// Resolve a usable config unconditionally: the env override when
    /// set, otherwise the DEFAULT model (`ggml-large-v3.bin` from the
    /// upstream `ggerganov/whisper.cpp` conversions), fetched once via
    /// hf-hub into its cache and reused from there on every later call.
    ///
    /// This is what makes `whisper_rs` usable out of the box, which is a
    /// requirement, not a convenience: a fresh machine needs no env var,
    /// only network access on first use. Without the
    /// `whisper-rs-backend` feature there is no hf-hub dependency, so
    /// only the env override can succeed.
    pub fn resolve() -> Result<Self, super::WhisperNativeError> {
        if let Some(cfg) = Self::from_env() {
            return Ok(cfg);
        }
        #[cfg(feature = "whisper-rs-backend")]
        {
            // Memoized: hf-hub's `download_file` re-walks the cache and
            // issues an HTTPS HEAD on every call for a floating revision,
            // which at one call per job means a network round-trip (and a
            // soft HF-reachability dependency) per file. The default
            // model cannot change within a process, so resolve once.
            use std::sync::OnceLock;
            static RESOLVED_DEFAULT: OnceLock<PathBuf> = OnceLock::new();
            if let Some(path) = RESOLVED_DEFAULT.get() {
                return Ok(Self::for_model(path.clone()));
            }
            let client = hf_hub::HFClientSync::new().map_err(|e| {
                super::WhisperNativeError::ModelResolution {
                    reason: format!("hf-hub client init failed: {e}"),
                }
            })?;
            let (owner, name) = (DEFAULT_MODEL_REPO_OWNER, DEFAULT_MODEL_REPO_NAME);
            // Time transparency: the first-ever resolution downloads a
            // ~3.1 GB model; later calls are cache hits inside hf-hub.
            tracing::info!(
                model = %format!("{owner}/{name}/{DEFAULT_MODEL_FILE}"),
                "resolving default whisper-rs model (first use downloads ~3.1 GB)"
            );
            let path = client
                .model(owner, name)
                .download_file()
                .filename(DEFAULT_MODEL_FILE.to_owned())
                .send()
                .map_err(|e| super::WhisperNativeError::ModelResolution {
                    reason: format!(
                        "download of {owner}/{name}/{DEFAULT_MODEL_FILE} failed: {e}"
                    ),
                })?;
            let path = RESOLVED_DEFAULT.get_or_init(|| path).clone();
            Ok(Self::for_model(path))
        }
        #[cfg(not(feature = "whisper-rs-backend"))]
        {
            Err(super::WhisperNativeError::ModelPathMissing)
        }
    }
}

/// Upstream repo carrying whisper.cpp's official ggml conversions.
pub const DEFAULT_MODEL_REPO_OWNER: &str = "ggerganov";
/// Repo name half of the default-model coordinates.
pub const DEFAULT_MODEL_REPO_NAME: &str = "whisper.cpp";
/// Default model file: large-v3, matching the quality tier the Python
/// whisper paths default to.
///
/// NOTE (CoreML): a model is an artifact SET when the
/// `whisper-rs-coreml` feature is on: acceleration needs a sibling
/// `<model>-encoder.mlmodelc` bundle next to the `.bin`, which this
/// single-file default cannot fetch. A CoreML build on the auto-fetched
/// default silently runs without CoreML; supply
/// `BATCHALIGN_WHISPER_RS_MODEL` pointing at a directory that carries
/// both artifacts, or extend this to a `DefaultModel { repo, file,
/// coreml_bundle }` set when CoreML prefetch lands.
pub const DEFAULT_MODEL_FILE: &str = "ggml-large-v3.bin";
