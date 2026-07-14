//! Concrete whisper-rs backend implementation. Compiled only when
//! `whisper-rs-backend` is enabled.

#![cfg(feature = "whisper-rs-backend")]

use std::path::Path;
use std::sync::Arc;

use batchalign_types::api::{DurationSeconds, LanguageCode3};
use batchalign_types::worker_v2::requests::WhisperChunkSpanV2;
use batchalign_types::worker_v2::responses::WhisperChunkResultV2;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::audio::{TARGET_SAMPLE_RATE, pcm_decode, resample};
use super::cache::PathKeyedCache;
use super::config::WhisperNativeConfig;
use super::error::WhisperNativeError;

/// Process-level cache for the loaded `WhisperContext`. First call
/// pays the ~3 s + 3 GB load; every subsequent call clones an `Arc`
/// and creates a per-job `WhisperState`.
static CONTEXT_CACHE: PathKeyedCache<WhisperContext> = PathKeyedCache::new();

/// Load `WhisperContext` from disk. Cold-load only; routed through
/// `CONTEXT_CACHE` so it runs once per process (per model path).
fn load_whisper_context(model_path: &Path) -> Result<Arc<WhisperContext>, WhisperNativeError> {
    let path_str = model_path
        .to_str()
        .ok_or_else(|| WhisperNativeError::ModelPathNotUtf8(model_path.to_path_buf()))?;
    let load_started = std::time::Instant::now();
    let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())?;
    tracing::info!(
        model = %model_path.display(),
        elapsed_ms = load_started.elapsed().as_millis() as u64,
        "whisper-native: cold-loaded WhisperContext"
    );
    Ok(Arc::new(ctx))
}

/// End-to-end transcribe via whisper.cpp's standalone decoder.
///
/// `WhisperContext` is shared across calls via `CONTEXT_CACHE`: cold-load
/// once per process, every subsequent call clones an `Arc` and creates a
/// fresh `WhisperState` (cheap: just decoder bookkeeping). See
/// `super::cache` for the cache policy and race-window behavior.
///
/// **Async caveat**: this function is sync. Callers in async context
/// must dispatch via `tokio::task::spawn_blocking` (or the existing
/// worker-pool `execute_v2` machinery, which already isolates
/// blocking work); otherwise inference will stall the executor for
/// minutes per call.
pub(super) fn transcribe_impl(
    audio_path: &Path,
    lang: LanguageCode3,
    cfg: &WhisperNativeConfig,
) -> Result<WhisperChunkResultV2, WhisperNativeError> {
    tracing::info!(
        model = %cfg.model_path.display(),
        audio = %audio_path.display(),
        lang = %lang,
        "whisper-native: starting"
    );

    let (mut pcm, native_sr) = pcm_decode(audio_path)?;
    if native_sr != TARGET_SAMPLE_RATE {
        tracing::debug!(
            from = native_sr,
            to = TARGET_SAMPLE_RATE,
            samples_in = pcm.len(),
            "whisper-native: resampling"
        );
        pcm = resample(&pcm, native_sr, TARGET_SAMPLE_RATE)?;
    }

    let ctx = CONTEXT_CACHE.get_or_load(&cfg.model_path, load_whisper_context)?;
    let mut state = ctx.create_state()?;

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    let lang_iso2 = lang_to_iso2(&lang)?;
    params.set_language(Some(lang_iso2));
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    if let Some(n) = cfg.n_threads {
        params.set_n_threads(n);
    }
    if let Some(mc) = cfg.max_context {
        // Pilot finding: setting this to 0 resets decoder context per
        // 30 s chunk, suppressing tail-of-audio token-loop hallucinations
        // (and running ~45% faster: cross-chunk attention is expensive).
        params.set_n_max_text_ctx(mc);
    }
    params.set_translate(cfg.translate);

    state.full(params, &pcm)?;

    let n_segments = state.full_n_segments();
    let mut chunks: Vec<WhisperChunkSpanV2> = Vec::with_capacity(n_segments as usize);
    for segment in state.as_iter() {
        let text = segment
            .to_str_lossy()
            .map_err(|e| WhisperNativeError::SegmentTextNotUtf8 {
                reason: e.to_string(),
            })?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        // whisper.cpp returns timestamps in centiseconds (10 ms units).
        chunks.push(WhisperChunkSpanV2 {
            text: trimmed.to_owned(),
            start_s: DurationSeconds(segment.start_timestamp() as f64 / 100.0),
            end_s: DurationSeconds(segment.end_timestamp() as f64 / 100.0),
        });
    }

    let text = chunks
        .iter()
        .map(|c| c.text.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    tracing::info!(
        chunks = chunks.len(),
        chars = text.len(),
        "whisper-native: done"
    );

    Ok(WhisperChunkResultV2 { lang, text, chunks })
}

/// Map ISO 639-3 (TalkBank's internal canonical) to ISO 639-1 (whisper.cpp's
/// expected language token).
///
/// Returns `UnsupportedLanguage` for codes outside the table; silent
/// default-to-English would mask real misconfiguration. The Rev.AI
/// preflight code at `crates/batchalign-app/src/revai/preflight.rs`
/// keeps a fuller table; this list covers the languages whisper.cpp's
/// large-v3 model genuinely supports well. Lifting both into a shared
/// `LanguageCode3::to_iso_639_1()` on `batchalign-types` is the right
/// follow-up; keeping the duplication in-place for this commit so the
/// integration foundation lands without dragging in a cross-crate
/// refactor.
fn lang_to_iso2(lang: &LanguageCode3) -> Result<&'static str, WhisperNativeError> {
    let iso = match &**lang {
        "eng" => "en",
        "yue" => "yue",
        "cmn" | "zho" => "zh",
        "fra" | "fre" => "fr",
        "deu" | "ger" => "de",
        "spa" => "es",
        "ita" => "it",
        "jpn" => "ja",
        "kor" => "ko",
        "nld" | "dut" => "nl",
        "por" => "pt",
        "rus" => "ru",
        "tur" => "tr",
        "swe" => "sv",
        "nor" => "no",
        "dan" => "da",
        "fin" => "fi",
        "pol" => "pl",
        "ell" | "gre" => "el",
        "hun" => "hu",
        "heb" => "he",
        "ara" => "ar",
        other => return Err(WhisperNativeError::UnsupportedLanguage(other.to_owned())),
    };
    Ok(iso)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn lang_map_known_codes() {
        assert_eq!(lang_to_iso2(&LanguageCode3::eng()).unwrap(), "en");
    }

    #[test]
    fn lang_map_unknown_returns_error() {
        let bogus = LanguageCode3("xxx".to_owned());
        let err = lang_to_iso2(&bogus).unwrap_err();
        assert!(matches!(err, WhisperNativeError::UnsupportedLanguage(_)));
    }

    #[test]
    fn config_defaults_are_pilot_recommended() {
        let cfg = WhisperNativeConfig::for_model(std::path::PathBuf::from("dummy.bin"));
        assert_eq!(cfg.n_threads, Some(8));
        assert_eq!(cfg.max_context, Some(0));
        assert!(!cfg.translate);
    }
}
