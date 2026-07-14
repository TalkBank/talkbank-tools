//! Standalone whisper.cpp (via `whisper-rs`) CLI for the inference pilot.
//!
//! Same shape and output schema as `candle-pilot`, so the two arms can be
//! compared directly on accuracy, wall time, memory, and binary size.
//!
//! Usage:
//!     whisper-rs-pilot <audio.{wav,mp3,…}> --model /path/to/ggml-small.bin --lang en
//!     whisper-rs-pilot <audio> --model <ggml> --lang en --json out.json

use anyhow::{Context, Result};
use batchalign_whisper_pilot::{PilotChunk, PilotResult, audio};
use clap::Parser;
use std::path::PathBuf;
use std::time::Instant;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Whisper expects 16 kHz mono PCM. Used for both decode-quality assertion
/// and resample target.
const SAMPLE_RATE_HZ: u32 = 16_000;

#[derive(Parser, Debug)]
#[command(
    name = "whisper-rs-pilot",
    about = "whisper.cpp inference pilot driver"
)]
struct Args {
    /// Path to an audio file (any symphonia-supported format).
    audio: PathBuf,

    /// Path to a ggml-format model file (`.bin`). Pre-downloaded from
    /// https://huggingface.co/ggerganov/whisper.cpp.
    #[arg(long)]
    model: PathBuf,

    /// 2-letter ISO language code.
    #[arg(long, default_value = "en")]
    lang: String,

    /// Optional JSON output path. If omitted, results are printed to stdout.
    #[arg(long)]
    json: Option<PathBuf>,

    /// Number of CPU threads for inference. Default: whisper.cpp's heuristic.
    #[arg(long)]
    threads: Option<i32>,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let args = Args::parse();

    log::info!("loading audio: {}", args.audio.display());
    let (mut pcm_data, sample_rate) =
        audio::pcm_decode(&args.audio).map_err(|e| anyhow::anyhow!("audio decode failed: {e}"))?;
    if sample_rate != SAMPLE_RATE_HZ {
        log::info!(
            "resampling {} Hz → {} Hz ({} samples)",
            sample_rate,
            SAMPLE_RATE_HZ,
            pcm_data.len()
        );
        pcm_data = audio::resample(&pcm_data, sample_rate, SAMPLE_RATE_HZ)
            .map_err(|e| anyhow::anyhow!("resample failed: {e}"))?;
    }
    log::info!("PCM samples: {}", pcm_data.len());

    log::info!("loading ggml model: {}", args.model.display());
    let ctx = WhisperContext::new_with_params(
        args.model
            .to_str()
            .context("model path must be valid UTF-8")?,
        WhisperContextParameters::default(),
    )
    .map_err(|e| anyhow::anyhow!("whisper-rs context init failed: {e}"))?;
    let mut state = ctx
        .create_state()
        .map_err(|e| anyhow::anyhow!("create_state failed: {e}"))?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(args.lang.as_str()));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    if let Some(n) = args.threads {
        params.set_n_threads(n);
    }

    log::info!("running whisper.cpp inference");
    let start = Instant::now();
    state
        .full(params, &pcm_data)
        .map_err(|e| anyhow::anyhow!("whisper.cpp full() failed: {e}"))?;
    let elapsed = start.elapsed();

    let n_segments = state.full_n_segments();
    log::info!("decoded {} segments", n_segments);

    let mut chunks: Vec<PilotChunk> = Vec::with_capacity(n_segments as usize);
    for segment in state.as_iter() {
        let text = segment
            .to_str_lossy()
            .map_err(|e| anyhow::anyhow!("segment text decode failed: {e}"))?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        // whisper.cpp returns timestamps in centiseconds (10 ms units).
        let t0_cs = segment.start_timestamp();
        let t1_cs = segment.end_timestamp();
        chunks.push(PilotChunk {
            text: trimmed.to_owned(),
            start_s: t0_cs as f64 / 100.0,
            end_s: t1_cs as f64 / 100.0,
        });
    }

    let text = chunks
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let result = PilotResult {
        lang: args.lang.clone(),
        text,
        chunks,
    };

    eprintln!();
    eprintln!("=== summary ===");
    eprintln!("audio:     {}", args.audio.display());
    eprintln!("model:     {}", args.model.display());
    eprintln!("lang:      {}", args.lang);
    eprintln!("chunks:    {}", result.chunks.len());
    eprintln!("wall time: {:.2}s", elapsed.as_secs_f64());

    let json = serde_json::to_string_pretty(&result)?;
    match args.json {
        Some(path) => {
            std::fs::write(&path, &json)?;
            eprintln!("wrote: {}", path.display());
        }
        None => println!("{json}"),
    }

    Ok(())
}
