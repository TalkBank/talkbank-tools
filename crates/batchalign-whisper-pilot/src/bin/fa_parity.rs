//! Parity harness: Rust candle forced alignment vs the Python golden.
//!
//! Usage: fa_parity <audio> <golden.json> [--metal]
//!
//! The golden file is produced by the production Python FA path (see
//! the scratch `golden_fa.py`); this binary runs the Rust port on the
//! same audio+text and reports per-token deltas. Exit codes: 0 = token
//! sequences identical and max |delta| <= 0.10 s; 1 = mismatch.

use anyhow::{Result, anyhow};
use batchalign_whisper_pilot::WhisperModel;
use batchalign_whisper_pilot::fa::{FaRequest, forced_align};


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
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        return Err(anyhow!("usage: fa_parity <audio> <golden.json> [--metal]"));
    }
    let device = if args.iter().any(|a| a == "--metal") {
        candle_core::Device::new_metal(0)?
    } else {
        candle_core::Device::Cpu
    };
    let golden: Golden = serde_json::from_str(&std::fs::read_to_string(&args[2])?)?;
    let mel_override = args
        .iter()
        .position(|a| a == "--mel-json")
        .map(|i| -> Result<_> {
            let path = args
                .get(i + 1)
                .ok_or_else(|| anyhow!("--mel-json needs a path"))?;
            Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
        })
        .transpose()?;
    let req = FaRequest {
        model: WhisperModel::LargeV2,
        audio_path: std::path::PathBuf::from(&args[1]),
        text: golden.text.clone(),
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
        println!("{:>24}  golden {:7.3}s  rust {:7.3}s  d {:6.3}s", g.text, g.time_s, r.time_s, d);
    }
    let mean = sum_delta / ours.len() as f64;
    println!("tokens {}  max|d| {:.3}s  mean|d| {:.3}s", ours.len(), max_delta, mean);
    if max_delta > 0.10 {
        return Err(anyhow!("PARITY FAIL: max delta {max_delta:.3}s > 0.10s"));
    }
    println!("PARITY OK");
    Ok(())
}
