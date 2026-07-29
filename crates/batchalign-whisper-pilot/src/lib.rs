//! Pilot crate measuring Rust-native Whisper inference (candle) against
//! the Python sidecar baseline.
//!
//! The public entry point is [`transcribe`], which:
//! 1. decodes audio with `symphonia` (any format) → mono f32 PCM,
//! 2. resamples to 16 kHz with `rubato` if needed,
//! 3. computes the log-mel spectrogram with candle-transformers,
//! 4. loads model + tokenizer from HuggingFace Hub (cached locally),
//! 5. runs the Whisper encoder/decoder loop ([`decoder::Decoder`]),
//! 6. extracts per-utterance chunks with timestamps.
//!
//! Output mirrors batchalign3's `WhisperChunkResultV2` field-for-field;
//! see [`PilotResult`].

use anyhow::{Error as E, Result};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, Config};
use hf_hub::HFClientSync;
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;

pub mod audio;
pub mod decoder;
pub mod fa;
pub mod fa_model;

use decoder::{Decoder, Model, Segment, Task, token_id};

/// Whisper model size to load. Mirrors candle-examples' `WhichModel` but
/// pruned to the variants the pilot uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
    Medium,
    /// large-v2: the production Python FA path's default checkpoint, so
    /// the FA parity harness aligns with the same weights.
    LargeV2,
    LargeV3,
}

impl WhisperModel {
    pub fn hf_repo(&self) -> (&'static str, &'static str) {
        match self {
            Self::Tiny => ("openai/whisper-tiny", "main"),
            Self::Base => ("openai/whisper-base", "refs/pr/22"),
            Self::Small => ("openai/whisper-small", "main"),
            Self::Medium => ("openai/whisper-medium", "main"),
            Self::LargeV2 => ("openai/whisper-large-v2", "main"),
            Self::LargeV3 => ("openai/whisper-large-v3", "main"),
        }
    }
}

/// Pilot input bundle.
#[derive(Debug, Clone)]
pub struct PilotConfig {
    /// Path to any symphonia-supported audio file (WAV, MP3, FLAC, …).
    pub audio_path: std::path::PathBuf,
    /// Whisper checkpoint to load.
    pub model: WhisperModel,
    /// 2-letter ISO language code (e.g. `"en"`). Currently mandatory;
    /// auto-detection is left for a follow-up.
    pub language: String,
    /// Inference device. This pilot uses `Device::Cpu`; GPU/Metal acceleration
    /// is intentionally not enabled for the candle arm.
    pub device: Device,
    /// RNG seed for the temperature-fallback sampler.
    pub seed: u64,
}

/// One transcribed text span with timestamp bounds (seconds from audio
/// start). Mirrors `batchalign_types::worker_v2::requests::WhisperChunkSpanV2`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PilotChunk {
    pub text: String,
    pub start_s: f64,
    pub end_s: f64,
}

/// Pilot output. Field shape mirrors `WhisperChunkResultV2`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PilotResult {
    pub lang: String,
    pub text: String,
    pub chunks: Vec<PilotChunk>,
}

/// Run candle-Whisper inference end-to-end on an audio file.
pub fn transcribe(cfg: &PilotConfig) -> Result<PilotResult> {
    log::info!("loading audio: {}", cfg.audio_path.display());
    let (mut pcm_data, sample_rate) = audio::pcm_decode(&cfg.audio_path)
        .map_err(|e| anyhow::anyhow!("audio decode failed: {e}"))?;
    if sample_rate != m::SAMPLE_RATE as u32 {
        log::info!(
            "resampling {} Hz → {} Hz ({} samples)",
            sample_rate,
            m::SAMPLE_RATE,
            pcm_data.len()
        );
        pcm_data = audio::resample(&pcm_data, sample_rate, m::SAMPLE_RATE as u32)
            .map_err(|e| anyhow::anyhow!("resample failed: {e}"))?;
    }
    log::info!("PCM samples: {}", pcm_data.len());

    let (model_id, revision) = cfg.model.hf_repo();
    log::info!("fetching model: {} @ {}", model_id, revision);
    let fetch = hf_fetcher(cfg.model)?;
    let config_path = fetch("config.json")?;
    let tokenizer_path = fetch("tokenizer.json")?;
    let weights_path = fetch("model.safetensors")?;

    let config: Config = serde_json::from_str(&std::fs::read_to_string(config_path)?)?;
    let tokenizer = Tokenizer::from_file(tokenizer_path).map_err(E::msg)?;

    let mel_filters = load_mel_filters(config.num_mel_bins)?;
    let mel = m::audio::pcm_to_mel(&config, &pcm_data, &mel_filters);
    let mel_len = mel.len();
    let mel = Tensor::from_vec(
        mel,
        (1, config.num_mel_bins, mel_len / config.num_mel_bins),
        &cfg.device,
    )?;
    log::info!("mel shape: {:?}", mel.dims());

    let model = {
        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], m::DTYPE, &cfg.device)? };
        Model::Normal(m::model::Whisper::load(&vb, config)?)
    };

    let language_token = match token_id(&tokenizer, &format!("<|{}|>", cfg.language)) {
        Ok(id) => Some(id),
        Err(_) => anyhow::bail!("language {} is not supported by this model", cfg.language),
    };

    let mut decoder = Decoder::new(
        model,
        tokenizer,
        cfg.seed,
        &cfg.device,
        language_token,
        Some(Task::Transcribe),
        true, // timestamps mode: we need them to extract chunks
        None, // max_initial_timestamp_index
    )?;

    log::info!("running decoder");
    let segments = decoder.run(&mel)?;
    log::info!("decoded {} segments", segments.len());
    for (i, seg) in segments.iter().enumerate() {
        log::info!(
            "  seg[{i}]: start={:.2}s dur={:.2}s tokens={} no_speech={:.3} avg_logprob={:.3} text={:?}",
            seg.start,
            seg.duration,
            seg.dr.tokens.len(),
            seg.dr.no_speech_prob,
            seg.dr.avg_logprob,
            seg.dr.text,
        );
    }

    let chunks = extract_chunks(&decoder, &segments)?;
    let text = chunks
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    Ok(PilotResult {
        lang: cfg.language.clone(),
        text,
        chunks,
    })
}

/// Decode the embedded mel-filter table for the given number of mel bins.
/// Build a fetcher over the model's hf-hub repo: `fetch("file")` returns
/// the cached-or-downloaded local path. One implementation for every
/// artifact fetch in this crate (the hf-hub 1.0 rewrite forced a sweep
/// of exactly this pattern; next time it is one edit).
///
/// hf-hub 1.0 replaced `Api`/`Repo` with the `HFClientSync` builder API,
/// and `client.model` takes owner + name separately rather than an
/// "owner/name" id string.
pub(crate) fn hf_fetcher(
    model: WhisperModel,
) -> Result<impl Fn(&str) -> Result<std::path::PathBuf>> {
    let (model_id, revision) = model.hf_repo();
    let (owner, name) = model_id
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("model id {model_id} is not in owner/name form"))?;
    let client =
        HFClientSync::new().map_err(|e| anyhow::anyhow!("hf-hub client init failed: {e}"))?;
    let repo = client.model(owner, name);
    Ok(move |file: &str| -> Result<std::path::PathBuf> {
        repo.download_file()
            .filename(file.to_owned())
            .revision(revision.to_owned())
            .send()
            .map_err(|e| anyhow::anyhow!("hf-hub download of {file} failed: {e}"))
    })
}

pub(crate) fn load_mel_filters(num_mel_bins: usize) -> Result<Vec<f32>> {
    let mel_bytes: &[u8] = match num_mel_bins {
        80 => include_bytes!("melfilters.bytes"),
        128 => include_bytes!("melfilters128.bytes"),
        n => anyhow::bail!("unexpected num_mel_bins {n}"),
    };
    let mut filters = vec![0f32; mel_bytes.len() / 4];
    <byteorder::LittleEndian as byteorder::ByteOrder>::read_f32_into(mel_bytes, &mut filters);
    Ok(filters)
}

/// Walk the per-segment token streams, splitting on Whisper's timestamp
/// tokens (everything above `no_timestamps_token`) to produce one
/// [`PilotChunk`] per inferred utterance.
fn extract_chunks(decoder: &Decoder, segments: &[Segment]) -> Result<Vec<PilotChunk>> {
    let mut out = Vec::new();
    let no_ts = decoder.no_timestamps_token();
    let sot = decoder.sot_token();
    let eot = decoder.eot_token();

    for segment in segments {
        let mut tokens_to_decode: Vec<u32> = Vec::new();
        let mut prev_timestamp_s = 0f32;
        let mut have_prev = false;

        for &token in &segment.dr.tokens {
            if token == sot || token == eot {
                continue;
            }
            if token > no_ts {
                let timestamp_s = (token - no_ts + 1) as f32 / 50.0;
                if !tokens_to_decode.is_empty() && have_prev {
                    let text = decoder
                        .tokenizer()
                        .decode(&tokens_to_decode, true)
                        .map_err(E::msg)?;
                    if !text.trim().is_empty() {
                        out.push(PilotChunk {
                            text: text.trim().to_owned(),
                            start_s: segment.start + prev_timestamp_s as f64,
                            end_s: segment.start + timestamp_s as f64,
                        });
                    }
                    tokens_to_decode.clear();
                }
                prev_timestamp_s = timestamp_s;
                have_prev = true;
            } else {
                tokens_to_decode.push(token);
            }
        }
        if !tokens_to_decode.is_empty() && have_prev {
            let text = decoder
                .tokenizer()
                .decode(&tokens_to_decode, true)
                .map_err(E::msg)?;
            if !text.trim().is_empty() {
                out.push(PilotChunk {
                    text: text.trim().to_owned(),
                    start_s: segment.start + prev_timestamp_s as f64,
                    end_s: segment.start + segment.duration,
                });
            }
        }
    }
    Ok(out)
}
