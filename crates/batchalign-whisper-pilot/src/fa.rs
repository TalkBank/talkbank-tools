//! Teacher-forced Whisper forced alignment on candle (the Rust FA port).
//!
//! Faithful reimplementation of the production Python path
//! (`batchalign/inference/fa.py::infer_whisper_fa`), which is the HF
//! `find_alignment` algorithm:
//!
//! 1. one forward pass with the KNOWN transcript as decoder input
//!    (teacher forcing: the causal-masked full-sequence forward),
//! 2. post-softmax cross-attention from the model's `alignment_heads`,
//! 3. per-(head, frame) standardization over the token axis,
//! 4. median filter along frames,
//! 5. head-mean cost matrix, row 0 flattened to the matrix mean,
//! 6. DTW; each token's time is the frame where the path enters its row,
//!    at 20 ms per frame.
//!
//! Steps 4-6 reuse `batchalign::whisper_native::fa_dtw`, the shared
//! dependency-free numeric core, so the pilot and the eventual
//! production dispatch cannot drift numerically.
//!
//! V1 constraint: one 30-second window (audio is padded or truncated to
//! 30 s), exactly matching how the production Python path is invoked by
//! the FA pipeline (which windows audio upstream per utterance batch).

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use candle_core::{Device, IndexOp, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, Config};
use hf_hub::HFClientSync;
use serde::Deserialize;
use tokenizers::Tokenizer;

use batchalign::whisper_native::fa_dtw::{
    CostMatrix, dynamic_time_warping, median_filter_rows, token_jump_times_s,
};

use crate::WhisperModel;
use crate::audio;

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
    /// computed mel (bins x frames, row-major). Used by the parity
    /// harness to localize divergence between the mel implementations
    /// and the transformer stack. `None` in real use.
    pub mel_override: Option<MelOverride>,
}

/// An externally supplied mel spectrogram for the diagnostic seam.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct MelOverride {
    /// Mel bin count (must match the model config).
    pub bins: usize,
    /// Frame count.
    pub frames: usize,
    /// Row-major values, `bins * frames` long.
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

/// Run teacher-forced alignment. Returns one timing per decoder token,
/// specials included, mirroring the Python path's output shape.
pub fn forced_align(req: &FaRequest) -> Result<Vec<FaTokenTiming>> {
    // ---- audio -> padded 30 s pcm -> mel --------------------------------
    let (mut pcm, sample_rate) = audio::pcm_decode(&req.audio_path)
        .map_err(|e| anyhow!("audio decode failed: {e}"))?;
    if sample_rate != m::SAMPLE_RATE as u32 {
        pcm = audio::resample(&pcm, sample_rate, m::SAMPLE_RATE as u32)
            .map_err(|e| anyhow!("resample failed: {e}"))?;
    }
    let n_samples = m::N_SAMPLES; // 30 s * 16 kHz
    pcm.resize(n_samples, 0.0);

    // ---- model artifacts -------------------------------------------------
    let (model_id, revision) = req.model.hf_repo();
    let (owner, name) = model_id
        .split_once('/')
        .ok_or_else(|| anyhow!("model id {model_id} is not in owner/name form"))?;
    let client = HFClientSync::new().map_err(|e| anyhow!("hf-hub client init failed: {e}"))?;
    let repo = client.model(owner, name);
    let fetch = |file: &str| -> Result<PathBuf> {
        repo.download_file()
            .filename(file.to_owned())
            .revision(revision.to_owned())
            .send()
            .map_err(|e| anyhow!("hf-hub download of {file} failed: {e}"))
    };
    let config: Config = serde_json::from_str(&std::fs::read_to_string(fetch("config.json")?)?)?;
    let generation: GenerationAlignmentConfig =
        serde_json::from_str(&std::fs::read_to_string(fetch("generation_config.json")?)?)
            .context("generation_config.json lacks alignment_heads")?;
    let tokenizer = Tokenizer::from_file(fetch("tokenizer.json")?).map_err(anyhow::Error::msg)?;
    let weights_path = fetch("model.safetensors")?;

    // ---- tokens: [sot, notimestamps] + text + [eot] ----------------------
    // Mirrors WhisperProcessor(text=...) labels as observed on the golden
    // (no language/task tokens when none are configured).
    let special = |tok: &str| -> Result<u32> {
        tokenizer
            .token_to_id(tok)
            .ok_or_else(|| anyhow!("tokenizer lacks special token {tok}"))
    };
    let sot = special("<|startoftranscript|>")?;
    let no_ts = special("<|notimestamps|>")?;
    let eot = special("<|endoftext|>")?;
    let encoding = tokenizer
        .encode(req.text.as_str(), false)
        .map_err(anyhow::Error::msg)?;
    let mut tokens: Vec<u32> = vec![sot, no_ts];
    tokens.extend_from_slice(encoding.get_ids());
    tokens.push(eot);
    // HF's model(labels=...) SHIFTS the labels right before the decoder
    // (shift_tokens_right: prepend decoder_start_token = sot, drop the
    // last), so attention row k is produced while PREDICTING label k
    // from input token k-1. Reproduce that exactly; timings still zip
    // with the UNSHIFTED labels, matching the Python path's output.
    let mut decoder_input: Vec<u32> = Vec::with_capacity(tokens.len());
    decoder_input.push(sot);
    decoder_input.extend_from_slice(&tokens[..tokens.len() - 1]);

    // ---- forward pass with capture --------------------------------------
    let mel = match &req.mel_override {
        Some(o) => {
            if o.bins != config.num_mel_bins || o.values.len() != o.bins * o.frames {
                return Err(anyhow!(
                    "mel override shape mismatch: {}x{} with {} values (model wants {} bins)",
                    o.bins, o.frames, o.values.len(), config.num_mel_bins
                ));
            }
            Tensor::from_vec(o.values.clone(), (1, o.bins, o.frames), &req.device)?
        }
        None => {
            let mel_filters = crate::load_mel_filters(config.num_mel_bins)?;
            let mel = m::audio::pcm_to_mel(&config, &pcm, &mel_filters);
            // candle's pcm_to_mel pads beyond the 30 s window; Whisper's
            // encoder consumes exactly N_FRAMES (3000) mel frames, same
            // as the HF processor's fixed-size input_features.
            let n_frames_total = mel.len() / config.num_mel_bins;
            let n_frames_mel = n_frames_total.min(m::N_FRAMES);
            Tensor::from_vec(mel, (1, config.num_mel_bins, n_frames_total), &req.device)?
                .narrow(2, 0, n_frames_mel)?
        }
    };

    let vb = unsafe {
        VarBuilder::from_mmaped_safetensors(&[weights_path], m::DTYPE, &req.device)?
    };
    let mut model = crate::fa_model::FaWhisper::load(&vb, config)?;
    let audio_features = model.encoder.forward(&mel, true)?;
    let token_t = Tensor::new(decoder_input.as_slice(), &req.device)?.unsqueeze(0)?;
    let _hidden = model.decoder.forward(&token_t, &audio_features, true)?;
    let cross = model.decoder.take_cross_attentions();
    let n_layers = model.config.decoder_layers;
    if cross.len() != n_layers {
        return Err(anyhow!(
            "cross-attention capture incomplete: {} of {n_layers} layers",
            cross.len()
        ));
    }

    // ---- alignment-head weight stack [heads][tokens][frames] -------------
    let n_tokens = tokens.len();
    let mut head_mats: Vec<Vec<f32>> = Vec::with_capacity(generation.alignment_heads.len());
    let mut n_frames = 0usize;
    for &(layer, head) in &generation.alignment_heads {
        let layer_t = cross
            .get(layer)
            .ok_or_else(|| anyhow!("alignment head references layer {layer} out of range"))?;
        // [batch, heads, tokens, frames] -> [tokens, frames]
        let w = layer_t.i((0, head))?.to_dtype(candle_core::DType::F32)?;
        let (t, f) = w.dims2()?;
        if t != n_tokens {
            return Err(anyhow!("attention token axis {t} != token count {n_tokens}"));
        }
        n_frames = f;
        head_mats.push(w.flatten_all()?.to_vec1::<f32>()?);
    }

    // ---- standardize over the token axis, per (head, frame) --------------
    // Python: std, mean = torch.std_mean(weights, dim=-2, keepdim=True,
    // unbiased=False); weights = (weights - mean) / std
    for mat in &mut head_mats {
        for f in 0..n_frames {
            let mut mean = 0.0f64;
            for t in 0..n_tokens {
                mean += f64::from(mat[t * n_frames + f]);
            }
            mean /= n_tokens as f64;
            let mut var = 0.0f64;
            for t in 0..n_tokens {
                let d = f64::from(mat[t * n_frames + f]) - mean;
                var += d * d;
            }
            var /= n_tokens as f64;
            let std = var.sqrt().max(f64::EPSILON);
            for t in 0..n_tokens {
                let idx = t * n_frames + f;
                mat[idx] = ((f64::from(mat[idx]) - mean) / std) as f32;
            }
        }
    }

    // ---- median filter along frames, then head-mean ----------------------
    let mut mean_matrix = vec![0.0f32; n_tokens * n_frames];
    let n_heads = head_mats.len();
    for mat in head_mats {
        let mut cm = CostMatrix::new(n_tokens, n_frames, mat)?;
        median_filter_rows(&mut cm, generation.median_filter_width)?;
        for (acc, v) in mean_matrix.iter_mut().zip(cm.values.iter()) {
            *acc += *v / n_heads as f32;
        }
    }
    // Python: matrix[0] = matrix.mean()
    let global_mean =
        mean_matrix.iter().map(|v| f64::from(*v)).sum::<f64>() / mean_matrix.len() as f64;
    for v in mean_matrix.iter_mut().take(n_frames) {
        *v = global_mean as f32;
    }

    // ---- DTW over the NEGATED matrix ------------------------------------
    let neg: Vec<f32> = mean_matrix.iter().map(|v| -v).collect();
    let cost = CostMatrix::new(n_tokens, n_frames, neg)?;
    let path = dynamic_time_warping(&cost)?;
    let times = token_jump_times_s(&path);
    if times.len() != n_tokens {
        return Err(anyhow!(
            "DTW produced {} token times for {n_tokens} tokens",
            times.len()
        ));
    }

    Ok(tokens
        .iter()
        .zip(times)
        .map(|(id, time_s)| FaTokenTiming {
            token: tokenizer.decode(&[*id], false).unwrap_or_default(),
            time_s,
        })
        .collect())
}
