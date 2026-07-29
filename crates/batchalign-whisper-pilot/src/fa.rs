//! Teacher-forced Whisper forced alignment on candle (the Rust FA port).
//!
//! Faithful reimplementation of the production Python path
//! (`batchalign/inference/fa.py::infer_whisper_fa`), which is the HF
//! `find_alignment` algorithm:
//!
//! 1. one forward pass with the KNOWN transcript as decoder input
//!    (teacher forcing; NB HF's `model(labels=...)` shifts the labels
//!    right first, so the decoder input is `[sot] + labels[..n-1]`
//!    while timings map to the UNSHIFTED labels),
//! 2. post-softmax cross-attention from the model's `alignment_heads`,
//! 3. per-(head, frame) standardization over the token axis,
//! 4. median filter along frames,
//! 5. head-mean cost matrix, row 0 flattened to the matrix mean,
//! 6. DTW; each token's time is the frame where the path enters its row,
//!    at 20 ms per frame.
//!
//! Steps 3-6 reuse `batchalign::whisper_native::fa_dtw`, the shared
//! dependency-free numeric core, so the pilot and the eventual
//! production dispatch cannot drift numerically.
//!
//! Split for promotion: [`FaAssets::load`] acquires artifacts and builds
//! the model ONCE; [`FaAssets::align`] runs per call. The production
//! dispatch caches `FaAssets` (per model+device) and calls `align` per
//! utterance batch; [`forced_align`] composes the two for one-shot use
//! (the parity harness). When this promotes, the `anyhow` errors here
//! become a typed `FaError` beside `FaDtwError`; `anyhow` stays in the
//! harness bin.
//!
//! V1 constraint: one 30-second window (audio is padded or truncated to
//! 30 s), matching how the production Python path is invoked by the FA
//! pipeline (which windows audio upstream per utterance batch).

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use candle_core::{Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, Config};
use serde::Deserialize;
use tokenizers::Tokenizer;

use batchalign::whisper_native::fa_dtw::{
    CostMatrix, dynamic_time_warping, median_filter_rows, standardize_columns,
    token_jump_times_s,
};

use crate::WhisperModel;
use crate::audio;
use crate::decoder::token_id;

/// One aligned token: its decoded text and jump time in seconds.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FaTokenTiming {
    /// Decoded token text (HF tokenizer `decode` of the single id).
    pub token: String,
    /// Jump time: seconds at which the DTW path enters this token's row.
    pub time_s: f64,
}

/// A forced-alignment request.
#[derive(Debug, Clone)]
pub struct FaRequest {
    /// Which Whisper checkpoint to align with.
    pub model: WhisperModel,
    /// Audio file (any format symphonia can decode).
    pub audio_path: PathBuf,
    /// The known transcript to force through the decoder.
    pub text: String,
    /// Inference device.
    pub device: Device,
    /// DIAGNOSTIC seam: bypass the pcm->mel front-end with an externally
    /// computed mel. Used by the parity harness to localize divergence
    /// between the mel implementations and the transformer stack.
    /// `None` in real use.
    pub mel_override: Option<MelOverride>,
}

/// An externally supplied mel spectrogram for the diagnostic seam.
/// Frame count is derived (`values.len() / bins`).
#[derive(Debug, Clone, Deserialize)]
pub struct MelOverride {
    /// Mel bin count (must match the model config).
    pub bins: usize,
    /// Row-major values; length must be a multiple of `bins`.
    pub values: Vec<f32>,
}

/// The subset of HF `generation_config.json` forced alignment needs.
#[derive(Debug, Deserialize)]
struct GenerationAlignmentConfig {
    alignment_heads: Vec<(usize, usize)>,
    #[serde(default = "default_median_filter_width")]
    median_filter_width: usize,
}

fn default_median_filter_width() -> usize {
    7
}

/// Everything loaded once per (model, device): config, alignment
/// parameters, tokenizer, weights-backed model.
pub struct FaAssets {
    config: Config,
    generation: GenerationAlignmentConfig,
    tokenizer: Tokenizer,
    model: crate::fa_model::FaWhisper,
    device: Device,
}

impl FaAssets {
    /// Fetch artifacts (config, generation config, tokenizer, weights)
    /// via hf-hub and build the capture-enabled model on `device`.
    pub fn load(model: WhisperModel, device: &Device) -> Result<Self> {
        let fetch = crate::hf_fetcher(model)?;
        let config: Config =
            serde_json::from_str(&std::fs::read_to_string(fetch("config.json")?)?)?;
        let generation: GenerationAlignmentConfig =
            serde_json::from_str(&std::fs::read_to_string(fetch("generation_config.json")?)?)
                .context("generation_config.json lacks alignment_heads")?;
        let tokenizer =
            Tokenizer::from_file(fetch("tokenizer.json")?).map_err(anyhow::Error::msg)?;
        let weights_path = fetch("model.safetensors")?;
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[weights_path], m::DTYPE, device)?
        };
        let model = crate::fa_model::FaWhisper::load(&vb, config.clone())?;
        Ok(Self {
            config,
            generation,
            tokenizer,
            model,
            device: device.clone(),
        })
    }

    /// Run teacher-forced alignment of `text` against a mel spectrogram.
    /// Returns one timing per label token, specials included, mirroring
    /// the Python path's output shape.
    pub fn align(&mut self, mel: &Tensor, text: &str) -> Result<Vec<FaTokenTiming>> {
        // ---- labels: [sot, notimestamps] + text + [eot] ------------------
        // Mirrors WhisperProcessor(text=...) labels as observed on the
        // golden (no language/task tokens when none are configured).
        let sot = token_id(&self.tokenizer, m::SOT_TOKEN).map_err(anyhow::Error::msg)?;
        let no_ts =
            token_id(&self.tokenizer, m::NO_TIMESTAMPS_TOKEN).map_err(anyhow::Error::msg)?;
        let eot = token_id(&self.tokenizer, m::EOT_TOKEN).map_err(anyhow::Error::msg)?;
        let encoding = self
            .tokenizer
            .encode(text, false)
            .map_err(anyhow::Error::msg)?;
        let mut labels: Vec<u32> = vec![sot, no_ts];
        labels.extend_from_slice(encoding.get_ids());
        labels.push(eot);
        // HF's model(labels=...) SHIFTS the labels right before the
        // decoder (shift_tokens_right: prepend decoder_start_token = sot,
        // drop the last), so attention row k is produced while PREDICTING
        // label k from input token k-1. Reproduce that exactly; timings
        // still zip with the UNSHIFTED labels.
        let mut decoder_input: Vec<u32> = Vec::with_capacity(labels.len());
        decoder_input.push(sot);
        decoder_input.extend_from_slice(&labels[..labels.len() - 1]);

        // ---- forward pass with capture -----------------------------------
        let audio_features = self.model.encoder.forward(mel, true)?;
        let token_t = Tensor::new(decoder_input.as_slice(), &self.device)?.unsqueeze(0)?;
        let _hidden = self.model.decoder.forward(&token_t, &audio_features, true)?;
        let cross = self.model.decoder.take_cross_attentions();
        if let Some(missing) = cross.iter().position(Option::is_none) {
            return Err(anyhow!(
                "cross-attention capture missing for decoder layer {missing}"
            ));
        }

        // ---- alignment-head matrices, standardized + filtered ------------
        let n_tokens = labels.len();
        let n_heads = self.generation.alignment_heads.len();
        let mut sum_matrix: Vec<f32> = Vec::new();
        let mut n_frames = 0usize;
        for &(layer, head) in &self.generation.alignment_heads {
            let layer_t = cross
                .get(layer)
                .and_then(|t| t.as_ref())
                .ok_or_else(|| anyhow!("alignment head references layer {layer} out of range"))?;
            // [batch, heads, tokens, frames] -> [tokens, frames]
            let w = layer_t.i((0, head))?.to_dtype(candle_core::DType::F32)?;
            let (t, f) = w.dims2()?;
            if t != n_tokens {
                return Err(anyhow!("attention token axis {t} != token count {n_tokens}"));
            }
            n_frames = f;
            let mut cm =
                CostMatrix::new(n_tokens, n_frames, w.flatten_all()?.to_vec1::<f32>()?)?;
            // Python: torch.std_mean(weights, dim=-2, unbiased=False)
            // per head, then median filter along frames.
            standardize_columns(&mut cm);
            median_filter_rows(&mut cm, self.generation.median_filter_width)?;
            let values = cm.into_values();
            if sum_matrix.is_empty() {
                sum_matrix = values;
            } else {
                for (acc, v) in sum_matrix.iter_mut().zip(values) {
                    *acc += v;
                }
            }
        }
        // Head mean; then Python's `matrix[0] = matrix.mean()`; then the
        // DTW runs over the NEGATED matrix.
        let inv_heads = 1.0f32 / n_heads as f32;
        for v in sum_matrix.iter_mut() {
            *v *= inv_heads;
        }
        let global_mean =
            sum_matrix.iter().map(|v| f64::from(*v)).sum::<f64>() / sum_matrix.len() as f64;
        for v in sum_matrix.iter_mut().take(n_frames) {
            *v = global_mean as f32;
        }
        for v in sum_matrix.iter_mut() {
            *v = -*v;
        }
        let cost = CostMatrix::new(n_tokens, n_frames, sum_matrix)?;
        let path = dynamic_time_warping(&cost)?;
        let times = token_jump_times_s(&path);
        if times.len() != n_tokens {
            return Err(anyhow!(
                "DTW produced {} token times for {n_tokens} tokens",
                times.len()
            ));
        }

        labels
            .iter()
            .zip(times)
            .map(|(id, time_s)| {
                let token = self
                    .tokenizer
                    .decode(&[*id], false)
                    .map_err(anyhow::Error::msg)?;
                Ok(FaTokenTiming { token, time_s })
            })
            .collect()
    }

    /// Build the model-shaped mel tensor for an audio file: decode,
    /// resample to 16 kHz, pad/truncate to the 30 s window, log-mel.
    pub fn mel_for_audio(&self, audio_path: &std::path::Path) -> Result<Tensor> {
        let (mut pcm, sample_rate) = audio::pcm_decode(audio_path)
            .map_err(|e| anyhow!("audio decode failed: {e}"))?;
        if sample_rate != m::SAMPLE_RATE as u32 {
            pcm = audio::resample(&pcm, sample_rate, m::SAMPLE_RATE as u32)
                .map_err(|e| anyhow!("resample failed: {e}"))?;
        }
        pcm.resize(m::N_SAMPLES, 0.0);
        let mel_filters = crate::load_mel_filters(self.config.num_mel_bins)?;
        let mel = m::audio::pcm_to_mel(&self.config, &pcm, &mel_filters);
        // candle's pcm_to_mel pads beyond the 30 s window; Whisper's
        // encoder consumes exactly N_FRAMES (3000) mel frames, same as
        // the HF processor's fixed-size input_features. The buffer is
        // BIN-major (each bin's full frame series contiguous), so the
        // truncation must happen on the tensor's frame axis, never on
        // the flat buffer (a flat truncate keeps the first bins whole
        // and drops the rest: measured as a 13.7 s mean timing error).
        let bins = self.config.num_mel_bins;
        let total_frames = mel.len() / bins;
        let frames = total_frames.min(m::N_FRAMES);
        Ok(Tensor::from_vec(mel, (1, bins, total_frames), &self.device)?
            .narrow(2, 0, frames)?)
    }

    fn mel_from_override(&self, o: &MelOverride) -> Result<Tensor> {
        if o.bins != self.config.num_mel_bins || !o.values.len().is_multiple_of(o.bins) {
            return Err(anyhow!(
                "mel override shape mismatch: {} bins with {} values (model wants {} bins)",
                o.bins,
                o.values.len(),
                self.config.num_mel_bins
            ));
        }
        let frames = o.values.len() / o.bins;
        Ok(Tensor::from_vec(
            o.values.clone(),
            (1, o.bins, frames),
            &self.device,
        )?)
    }
}

/// One-shot forced alignment: load assets, build the mel, align. The
/// parity harness's entry point; production callers hold `FaAssets` and
/// call `align` repeatedly instead.
pub fn forced_align(req: &FaRequest) -> Result<Vec<FaTokenTiming>> {
    let mut assets = FaAssets::load(req.model, &req.device)?;
    let mel = match &req.mel_override {
        Some(o) => assets.mel_from_override(o)?,
        None => assets.mel_for_audio(&req.audio_path)?,
    };
    assets.align(&mel, &req.text)
}
