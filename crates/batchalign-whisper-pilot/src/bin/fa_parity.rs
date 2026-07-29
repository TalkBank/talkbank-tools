//! Parity harness: Rust candle forced alignment vs the Python golden.
//!
//! The golden file is produced by the production Python FA path (see
//! the scratch `golden_fa.py`); this binary runs the Rust port on the
//! same audio+text and reports per-token deltas.

use anyhow::{Result, anyhow};
use batchalign_whisper_pilot::WhisperModel;
use batchalign_whisper_pilot::fa::{FaRequest, forced_align};
use clap::Parser;

/// Maximum per-token |delta| accepted as parity. Measured parity on
/// large-v2/JFK is 0.040 s (two DTW frames); 0.10 s (five frames) gives
/// headroom for device/dtype jitter across machines without letting a
/// systematic one-token shift (~0.2 s+) pass.
const MAX_TOKEN_DELTA_S: f64 = 0.10;

/// Compare Rust forced alignment against a Python-produced golden.
#[derive(Parser)]
struct Args {
    /// Audio file the golden was produced from.
    audio: std::path::PathBuf,
    /// Golden JSON (text + per-token timings) from golden_fa.py.
    golden: std::path::PathBuf,
    /// Bypass the Rust mel front-end with a Python-dumped mel
    /// (diagnostic: localizes divergence to mel vs transformer stack).
    #[arg(long)]
    mel_json: Option<std::path::PathBuf>,
    /// Run on Metal instead of CPU.
    #[arg(long)]
    metal: bool,
}

#[derive(serde::Deserialize)]
struct Golden {
    text: String,
    tokens: Vec<GoldenToken>,
}

#[derive(serde::Deserialize)]
struct GoldenToken {
    text: String,
    time_s: f64,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let device = if args.metal {
        candle_core::Device::new_metal(0)?
    } else {
        candle_core::Device::Cpu
    };
    let golden: Golden = serde_json::from_str(&std::fs::read_to_string(&args.golden)?)?;
    let mel_override = args
        .mel_json
        .as_ref()
        .map(|p| -> Result<_> { Ok(serde_json::from_str(&std::fs::read_to_string(p)?)?) })
        .transpose()?;
    let req = FaRequest {
        model: WhisperModel::LargeV2,
        audio_path: args.audio,
        text: golden.text,
        device,
        mel_override,
    };
    let ours = forced_align(&req)?;

    if ours.len() != golden.tokens.len() {
        return Err(anyhow!(
            "token count mismatch: rust {} vs golden {}",
            ours.len(),
            golden.tokens.len()
        ));
    }
    let mut max_delta = 0.0f64;
    let mut sum_delta = 0.0f64;
    for (r, g) in ours.iter().zip(&golden.tokens) {
        if r.token.trim() != g.text.trim() {
            return Err(anyhow!(
                "token text mismatch: rust {:?} vs golden {:?}",
                r.token,
                g.text
            ));
        }
        let d = (r.time_s - g.time_s).abs();
        max_delta = max_delta.max(d);
        sum_delta += d;
        println!(
            "{:>24}  golden {:7.3}s  rust {:7.3}s  d {:6.3}s",
            g.text, g.time_s, r.time_s, d
        );
    }
    let mean = sum_delta / ours.len() as f64;
    println!(
        "tokens {}  max|d| {:.3}s  mean|d| {:.3}s",
        ours.len(),
        max_delta,
        mean
    );
    if max_delta > MAX_TOKEN_DELTA_S {
        return Err(anyhow!(
            "PARITY FAIL: max delta {max_delta:.3}s > {MAX_TOKEN_DELTA_S}s"
        ));
    }
    println!("PARITY OK");
    Ok(())
}
