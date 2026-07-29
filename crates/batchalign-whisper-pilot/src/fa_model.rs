//! Vendored candle-transformers Whisper model WITH cross-attention capture,
//! for teacher-forced forced alignment.
//!
//! candle-transformers 0.11's `models::whisper::model` does not expose
//! attention weights, and the HF forced-alignment algorithm needs the
//! post-softmax CROSS-attention of specific alignment heads. This module
//! is that model file (same weight names, same math) with three additions,
//! each marked `FA-CAPTURE`:
//!
//! 1. `MultiHeadAttention` records its post-softmax weights when running
//!    as cross-attention.
//! 2. `ResidualAttentionBlock` lets the recorded tensor be taken.
//! 3. `TextDecoder::take_cross_attentions` harvests one tensor per layer
//!    after a forward pass.
//!
//! Everything else is byte-faithful to upstream so the numerics match the
//! reference implementation. Upstream:
//! <https://docs.rs/candle-transformers/0.11.0/src/candle_transformers/models/whisper/model.rs.html>

#![allow(clippy::unwrap_used)]

use candle_core::{Device, IndexOp, Result, Tensor, D};
use candle_nn::{embedding, Conv1d, Conv1dConfig, Embedding, LayerNorm, Module, VarBuilder};
use candle_transformers::models::whisper::Config;
use candle_transformers::models::with_tracing::{linear, linear_no_bias, Linear};

fn conv1d(
    in_channels: usize,
    out_channels: usize,
    kernel_size: usize,
    config: Conv1dConfig,
    vb: VarBuilder,
) -> Result<Conv1d> {
    let weight = vb.get((out_channels, in_channels, kernel_size), "weight")?;
    let bias = vb.get(out_channels, "bias")?;
    Ok(Conv1d::new(weight, Some(bias), config))
}

fn layer_norm(size: usize, vb: VarBuilder) -> Result<LayerNorm> {
    let weight = vb.get(size, "weight")?;
    let bias = vb.get(size, "bias")?;
    Ok(LayerNorm::new(weight, bias, 1e-5))
}

#[derive(Debug, Clone)]
struct MultiHeadAttention {
    query: Linear,
    key: Linear,
    value: Linear,
    out: Linear,
    n_head: usize,
    kv_cache: Option<(Tensor, Tensor)>,
    // FA-CAPTURE: post-softmax weights of the most recent CROSS-attention
    // forward ([batch, heads, q_ctx, k_ctx]); None for self-attention.
    last_cross_attn: Option<Tensor>,
}

impl MultiHeadAttention {
    fn load(n_state: usize, n_head: usize, vb: VarBuilder) -> Result<Self> {
        let query = linear(n_state, n_state, vb.pp("q_proj"))?;
        let value = linear(n_state, n_state, vb.pp("v_proj"))?;
        let key = linear_no_bias(n_state, n_state, vb.pp("k_proj"))?;
        let out = linear(n_state, n_state, vb.pp("out_proj"))?;
        Ok(Self {
            query,
            key,
            value,
            out,
            n_head,
            kv_cache: None,
            last_cross_attn: None,
        })
    }

    fn forward(
        &mut self,
        x: &Tensor,
        xa: Option<&Tensor>,
        mask: Option<&Tensor>,
        flush_cache: bool,
    ) -> Result<Tensor> {
        let is_cross = xa.is_some();
        let q = self.query.forward(x)?;
        let (k, v) = match xa {
            None => {
                let k = self.key.forward(x)?;
                let v = self.value.forward(x)?;
                (k, v)
            }
            Some(x) => {
                if flush_cache {
                    self.kv_cache = None;
                }
                if let Some((k, v)) = &self.kv_cache {
                    (k.clone(), v.clone())
                } else {
                    let k = self.key.forward(x)?;
                    let v = self.value.forward(x)?;
                    self.kv_cache = Some((k.clone(), v.clone()));
                    (k, v)
                }
            }
        };
        let wv = self.qkv_attention(&q, &k, &v, mask, is_cross)?;
        let out = self.out.forward(&wv)?;
        Ok(out)
    }

    fn reshape_head(&self, x: &Tensor) -> Result<Tensor> {
        let (n_batch, n_ctx, n_state) = x.dims3()?;
        let target_dims = &[n_batch, n_ctx, self.n_head, n_state / self.n_head];
        x.reshape(target_dims)?.transpose(1, 2)
    }

    fn qkv_attention(
        &mut self,
        q: &Tensor,
        k: &Tensor,
        v: &Tensor,
        mask: Option<&Tensor>,
        is_cross: bool,
    ) -> Result<Tensor> {
        let (_, n_ctx, n_state) = q.dims3()?;
        let scale = ((n_state / self.n_head) as f64).powf(-0.25);
        let q = (self.reshape_head(q)? * scale)?;
        let k = (self.reshape_head(k)?.transpose(2, 3)? * scale)?;
        let v = self.reshape_head(v)?.contiguous()?;
        let mut qk = q.matmul(&k)?;
        if let Some(mask) = mask {
            let mask = mask.i((0..n_ctx, 0..n_ctx))?;
            qk = qk.broadcast_add(&mask)?
        }
        let w = candle_nn::ops::softmax_last_dim(&qk)?;
        // FA-CAPTURE: keep the cross-attention weights for alignment.
        if is_cross {
            self.last_cross_attn = Some(w.clone());
        }
        let wv = w.matmul(&v)?.transpose(1, 2)?.flatten_from(2)?;
        Ok(wv)
    }
}

#[derive(Debug, Clone)]
struct ResidualAttentionBlock {
    attn: MultiHeadAttention,
    attn_ln: LayerNorm,
    cross_attn: Option<(MultiHeadAttention, LayerNorm)>,
    mlp_linear1: Linear,
    mlp_linear2: Linear,
    mlp_ln: LayerNorm,
}

impl ResidualAttentionBlock {
    fn load(n_state: usize, n_head: usize, ca: bool, vb: VarBuilder) -> Result<Self> {
        let attn = MultiHeadAttention::load(n_state, n_head, vb.pp("self_attn"))?;
        let attn_ln = layer_norm(n_state, vb.pp("self_attn_layer_norm"))?;
        let cross_attn = if ca {
            let cross_attn = MultiHeadAttention::load(n_state, n_head, vb.pp("encoder_attn"))?;
            let cross_attn_ln = layer_norm(n_state, vb.pp("encoder_attn_layer_norm"))?;
            Some((cross_attn, cross_attn_ln))
        } else {
            None
        };
        let n_mlp = n_state * 4;
        let mlp_linear1 = linear(n_state, n_mlp, vb.pp("fc1"))?;
        let mlp_linear2 = linear(n_mlp, n_state, vb.pp("fc2"))?;
        let mlp_ln = layer_norm(n_state, vb.pp("final_layer_norm"))?;
        Ok(Self {
            attn,
            attn_ln,
            cross_attn,
            mlp_linear1,
            mlp_linear2,
            mlp_ln,
        })
    }

    fn forward(
        &mut self,
        x: &Tensor,
        xa: Option<&Tensor>,
        mask: Option<&Tensor>,
        flush_kv_cache: bool,
    ) -> Result<Tensor> {
        let attn = self
            .attn
            .forward(&self.attn_ln.forward(x)?, None, mask, flush_kv_cache)?;
        let mut x = (x + attn)?;
        if let Some((attn, ln)) = &mut self.cross_attn {
            x = (&x + attn.forward(&ln.forward(&x)?, xa, None, flush_kv_cache)?)?;
        }
        let mlp = self.mlp_linear2.forward(
            &self
                .mlp_linear1
                .forward(&self.mlp_ln.forward(&x)?)?
                .gelu()?,
        )?;
        x + mlp
    }

    // FA-CAPTURE: take (and clear) this block's recorded cross-attention.
    fn take_cross_attn(&mut self) -> Option<Tensor> {
        self.cross_attn
            .as_mut()
            .and_then(|(attn, _)| attn.last_cross_attn.take())
    }
}

fn sinusoids(length: usize, channels: usize, device: &Device) -> Result<Tensor> {
    let max_timescale = 10000f32;
    let log_timescale_increment = max_timescale.ln() / (channels / 2 - 1) as f32;
    let inv_timescales: Vec<_> = (0..channels / 2)
        .map(|i| (i as f32 * (-log_timescale_increment)).exp())
        .collect();
    let inv_timescales = Tensor::new(inv_timescales.as_slice(), device)?.unsqueeze(0)?;
    let arange = Tensor::arange(0, length as u32, device)?
        .to_dtype(candle_core::DType::F32)?
        .unsqueeze(1)?;
    let sh = (length, channels / 2);
    let scaled_time = (arange.broadcast_as(sh)? * inv_timescales.broadcast_as(sh)?)?;
    let sincos = Tensor::cat(&[scaled_time.sin()?, scaled_time.cos()?], 1)?;
    Ok(sincos)
}

/// Audio encoder, identical to upstream.
#[derive(Debug, Clone)]
pub struct AudioEncoder {
    conv1: Conv1d,
    conv2: Conv1d,
    positional_embedding: Tensor,
    blocks: Vec<ResidualAttentionBlock>,
    ln_post: LayerNorm,
}

impl AudioEncoder {
    fn load(vb: VarBuilder, cfg: &Config) -> Result<Self> {
        let n_state = cfg.d_model;
        let n_head = cfg.encoder_attention_heads;
        let n_ctx = cfg.max_source_positions;
        let cfg1 = Conv1dConfig {
            padding: 1,
            stride: 1,
            groups: 1,
            dilation: 1,
            cudnn_fwd_algo: None,
        };
        let cfg2 = Conv1dConfig {
            padding: 1,
            stride: 2,
            groups: 1,
            dilation: 1,
            cudnn_fwd_algo: None,
        };
        let conv1 = conv1d(cfg.num_mel_bins, n_state, 3, cfg1, vb.pp("conv1"))?;
        let conv2 = conv1d(n_state, n_state, 3, cfg2, vb.pp("conv2"))?;
        let positional_embedding = sinusoids(n_ctx, n_state, vb.device())?;
        let blocks = (0..cfg.encoder_layers)
            .map(|i| {
                ResidualAttentionBlock::load(n_state, n_head, false, vb.pp(format!("layers.{i}")))
            })
            .collect::<Result<Vec<_>>>()?;
        let ln_post = layer_norm(n_state, vb.pp("layer_norm"))?;
        Ok(Self {
            conv1,
            conv2,
            positional_embedding,
            blocks,
            ln_post,
        })
    }

    /// Encode a mel spectrogram to hidden states.
    pub fn forward(&mut self, x: &Tensor, flush_kv_cache: bool) -> Result<Tensor> {
        let x = self.conv1.forward(x)?.gelu()?;
        let x = self.conv2.forward(&x)?.gelu()?;
        let x = x.transpose(1, 2)?;
        let (_bsize, seq_len, _hidden) = x.dims3()?;
        let positional_embedding = self.positional_embedding.narrow(0, 0, seq_len)?;
        let mut x = x.broadcast_add(&positional_embedding)?;
        for block in self.blocks.iter_mut() {
            x = block.forward(&x, None, None, flush_kv_cache)?
        }
        let x = self.ln_post.forward(&x)?;
        Ok(x)
    }
}

/// Text decoder with FA-CAPTURE harvesting.
#[derive(Debug, Clone)]
pub struct TextDecoder {
    token_embedding: Embedding,
    positional_embedding: Tensor,
    blocks: Vec<ResidualAttentionBlock>,
    ln: LayerNorm,
    mask: Tensor,
}

impl TextDecoder {
    fn load(vb: VarBuilder, cfg: &Config) -> Result<Self> {
        let n_state = cfg.d_model;
        let n_head = cfg.decoder_attention_heads;
        let n_ctx = cfg.max_target_positions;
        let token_embedding = embedding(cfg.vocab_size, n_state, vb.pp("embed_tokens"))?;
        let positional_embedding = vb.get((n_ctx, n_state), "embed_positions.weight")?;
        let blocks = (0..cfg.decoder_layers)
            .map(|i| {
                ResidualAttentionBlock::load(n_state, n_head, true, vb.pp(format!("layers.{i}")))
            })
            .collect::<Result<Vec<_>>>()?;
        let ln = layer_norm(n_state, vb.pp("layer_norm"))?;
        let mask: Vec<_> = (0..n_ctx)
            .flat_map(|i| (0..n_ctx).map(move |j| if j > i { f32::NEG_INFINITY } else { 0f32 }))
            .collect();
        let mask = Tensor::from_vec(mask, (n_ctx, n_ctx), vb.device())?;
        Ok(Self {
            token_embedding,
            positional_embedding,
            blocks,
            ln,
            mask,
        })
    }

    /// One (teacher-forced when `x` is the full known token sequence)
    /// forward pass.
    pub fn forward(&mut self, x: &Tensor, xa: &Tensor, flush_kv_cache: bool) -> Result<Tensor> {
        let last = x.dim(D::Minus1)?;
        let token_embedding = self.token_embedding.forward(x)?;
        let positional_embedding = self.positional_embedding.narrow(0, 0, last)?;
        let mut x = token_embedding.broadcast_add(&positional_embedding)?;
        for block in self.blocks.iter_mut() {
            x = block.forward(&x, Some(xa), Some(&self.mask), flush_kv_cache)?;
        }
        self.ln.forward(&x)
    }

    /// FA-CAPTURE: harvest each layer's recorded cross-attention weights
    /// (in layer order) after a forward pass. Layers whose weights were
    /// somehow not recorded are an error at the caller's level; `None`
    /// entries are represented by absence, so callers must check length.
    pub fn take_cross_attentions(&mut self) -> Vec<Tensor> {
        self.blocks
            .iter_mut()
            .filter_map(|b| b.take_cross_attn())
            .collect()
    }
}

/// Whisper with FA capture, mirroring upstream's `Whisper` wrapper.
#[derive(Debug, Clone)]
pub struct FaWhisper {
    /// Audio encoder half.
    pub encoder: AudioEncoder,
    /// Text decoder half (with capture).
    pub decoder: TextDecoder,
    /// Model configuration.
    pub config: Config,
}

impl FaWhisper {
    /// Load from the same safetensors weight layout as upstream.
    pub fn load(vb: &VarBuilder, config: Config) -> Result<Self> {
        let encoder = AudioEncoder::load(vb.pp("model.encoder"), &config)?;
        let decoder = TextDecoder::load(vb.pp("model.decoder"), &config)?;
        Ok(Self {
            encoder,
            decoder,
            config,
        })
    }
}
