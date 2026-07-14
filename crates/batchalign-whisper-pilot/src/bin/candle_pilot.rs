//! Standalone candle-Whisper CLI for the inference pilot.
//!
//! Run a single audio file through candle-Whisper and print the result
//! as JSON, alongside wall-time timing.
//!
//! Usage:
//!     candle-pilot <audio.mp3> --model small --lang en
//!     candle-pilot <audio.mp3> --model small --lang en --json out.json

use anyhow::Result;
use batchalign_whisper_pilot::{PilotConfig, WhisperModel, transcribe};
use candle_core::Device;
use clap::Parser;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "candle-pilot", about = "candle-Whisper inference pilot driver")]
struct Args {
    /// Path to an audio file (any symphonia-supported format).
    audio: PathBuf,

    /// Whisper checkpoint size.
    #[arg(long, value_enum, default_value = "small")]
    model: ModelArg,

    /// 2-letter ISO language code.
    #[arg(long, default_value = "en")]
    lang: String,

    /// Optional JSON output path. If omitted, results are printed to stdout.
    #[arg(long)]
    json: Option<PathBuf>,

    /// RNG seed for the temperature-fallback sampler.
    #[arg(long, default_value_t = 299_792_458)]
    seed: u64,
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
enum ModelArg {
    Tiny,
    Base,
    Small,
    Medium,
    LargeV3,
}

impl From<ModelArg> for WhisperModel {
    fn from(m: ModelArg) -> Self {
        match m {
            ModelArg::Tiny => Self::Tiny,
            ModelArg::Base => Self::Base,
            ModelArg::Small => Self::Small,
            ModelArg::Medium => Self::Medium,
            ModelArg::LargeV3 => Self::LargeV3,
        }
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let cfg = PilotConfig {
        audio_path: args.audio.clone(),
        model: args.model.into(),
        language: args.lang.clone(),
        device: Device::Cpu,
        seed: args.seed,
    };

    let start = Instant::now();
    let result = transcribe(&cfg)?;
    let elapsed = start.elapsed();

    eprintln!();
    eprintln!("=== summary ===");
    eprintln!("audio:     {}", args.audio.display());
    eprintln!("model:     {:?}", cfg.model);
    eprintln!("lang:      {}", cfg.language);
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
