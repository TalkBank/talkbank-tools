//! Parallel Rev.AI preflight submission owned by the Rust server.
//!
//! Preflight exists to upload many audio files to Rev.AI ahead of the normal
//! per-file processing loop. That is control-plane work: it is about queueing,
//! concurrency, and job bookkeeping, not model inference. Keeping it here
//! avoids widening the Python worker protocol with a generic HTTP sidecar API.

use crate::types::revai_language::RevAiLanguageHint;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use crate::revai::{RevAiClient, SubmitOptions};
use tokio::sync::Semaphore;

use crate::api::{NumSpeakers, RevAiJobId};

use super::{RevAiApiKey, RevAiCredentialError, load_revai_api_key};

/// Typed preflight submission plan built by the runner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RevAiPreflightPlan {
    /// Audio file paths to upload.
    pub(crate) audio_paths: Vec<PathBuf>,
    /// Batchalign job language: may be `Auto` for ASR auto-detection.
    pub(crate) lang: crate::api::LanguageSpec,
    /// Speaker-count hint forwarded to Rev.AI where supported.
    pub(crate) num_speakers: NumSpeakers,
    /// Maximum concurrent uploads.
    pub(crate) max_concurrent: usize,
}

/// Partial-success result for one preflight batch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct RevAiPreflightResult {
    /// Successfully submitted path → Rev.AI job ID mappings.
    pub(crate) job_ids: HashMap<PathBuf, RevAiJobId>,
    /// Path → error mappings for failed submissions.
    pub(crate) errors: BTreeMap<String, String>,
}

/// Run a production preflight batch using the configured Rev.AI credentials.
pub(crate) async fn preflight_submit_audio_paths(
    plan: &RevAiPreflightPlan,
) -> Result<RevAiPreflightResult, RevAiCredentialError> {
    let api_key = load_revai_api_key()?;
    Ok(submit_with(
        plan,
        Arc::new(move |request| submit_one_with_client(&api_key, request)),
    )
    .await)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RevAiSubmitRequest {
    audio_path: PathBuf,
    language: RevAiLanguageHint,
    speakers_count: Option<u32>,
    metadata: String,
}

type RevAiSubmitFn =
    Arc<dyn Fn(RevAiSubmitRequest) -> Result<String, String> + Send + Sync + 'static>;

async fn submit_with(plan: &RevAiPreflightPlan, submitter: RevAiSubmitFn) -> RevAiPreflightResult {
    let mut tasks = tokio::task::JoinSet::new();
    let concurrency = plan.max_concurrent.max(1);
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let language = match &plan.lang {
        // Auto and PerFile both reach Rev.AI as "auto", `PerFile` should
        // not happen on the transcribe path (CLI surface does not produce
        // it), but if a regression introduces it we ask Rev.AI to detect
        // rather than panic.
        crate::api::LanguageSpec::Auto | crate::api::LanguageSpec::PerFile => {
            RevAiLanguageHint::auto()
        }
        crate::api::LanguageSpec::Resolved(code) => RevAiLanguageHint::from(code),
    };
    let speakers_count = speakers_count_hint(language.as_str(), plan.num_speakers);

    for audio_path in &plan.audio_paths {
        let submit_request = RevAiSubmitRequest {
            audio_path: audio_path.clone(),
            language: language.clone(),
            speakers_count,
            metadata: format!(
                "batchalign3_{}",
                audio_path.file_stem().unwrap_or_default().to_string_lossy()
            ),
        };
        let submitter = submitter.clone();
        let semaphore = semaphore.clone();
        tasks.spawn(async move {
            // Lifetime invariant: `semaphore` is an `Arc<Semaphore>`
            // owned by `preflight_revai_jobs`'s caller for the
            // duration of this `tasks.join_all().await` below. The
            // semaphore can only close when its last owner drops it,
            // which cannot happen while these spawned tasks still
            // hold an `Arc` clone.
            #[allow(clippy::expect_used)]
            let _permit = semaphore
                .acquire_owned()
                .await
                .expect("preflight semaphore closed");
            let path = submit_request.audio_path.clone();
            let error_path = path.clone();
            let join = tokio::task::spawn_blocking(move || {
                let result = submitter(submit_request);
                (path, result)
            })
            .await;
            match join {
                Ok(pair) => pair,
                Err(err) => (
                    error_path,
                    Err(format!("preflight worker thread failed: {err}")),
                ),
            }
        });
    }

    let mut result = RevAiPreflightResult::default();
    while let Some(joined) = tasks.join_next().await {
        match joined {
            Ok((path, Ok(job_id))) => {
                result.job_ids.insert(path, RevAiJobId::from(job_id));
            }
            Ok((path, Err(error))) => {
                result
                    .errors
                    .insert(path.to_string_lossy().into_owned(), error);
            }
            Err(err) => {
                result.errors.insert(
                    "<internal>".to_string(),
                    format!("preflight task join failed: {err}"),
                );
            }
        }
    }

    result
}

fn submit_one_with_client(
    api_key: &RevAiApiKey,
    request: RevAiSubmitRequest,
) -> Result<String, String> {
    let client = RevAiClient::new(api_key.as_str());
    let options = SubmitOptions {
        language: request.language.as_str().to_string(),
        speakers_count: request.speakers_count,
        skip_postprocessing: skip_postprocessing_hint(request.language.as_str()),
        metadata: Some(request.metadata),
    };
    client
        .submit_local_file(&request.audio_path, &options)
        .map(|job| job.id)
        .map_err(|err| err.to_string())
}

fn speakers_count_hint(language: &str, num_speakers: NumSpeakers) -> Option<u32> {
    match language {
        "en" | "es" => Some(num_speakers.0),
        _ => None,
    }
}

/// Rev.AI `skip_postprocessing` policy.
///
/// When `true`, Rev.AI skips "inverse text normalization (ITN), casing
/// and punctuation" post-processing: returning the spoken-form text
/// (what the speaker literally said: `"eighty percent"`, `"seventeen year
/// old"`, `"May nineteenth"`) rather than the written-convenience form
/// produced by ITN (`"80%"`, `"17-year-old"`, `"May 19th"`). CHAT records
/// spoken form, so we want the flag `true` wherever it's available.
///
/// Rev.AI docs: "Only available for English and Spanish languages." For
/// other languages the parameter has no effect; we return `None` so the
/// request body doesn't carry a noop field.
///
/// Why not `Some(false)` elsewhere: sending `false` explicitly asks Rev.AI
/// to apply ITN, which is the historical default and produces main-tier-
/// illegal CHAT content for languages with E220 (digits forbidden in
/// word text). Prior to 2026-04-22 BA3's preflight hardcoded
/// `Some(false)` which caused the Rev.AI response to contain tokens like
/// `"80%"` / `"17-year-old"`; that, combined with a gap in the downstream
/// normalizer, produced the transcribe failure reported on that date.
/// See the provenance assessment report in the private workspace.
pub(super) fn skip_postprocessing_hint(language: &str) -> Option<bool> {
    match language {
        "en" | "es" => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::LanguageCode3;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn preflight_collects_successes_and_failures() {
        let plan = RevAiPreflightPlan {
            audio_paths: vec![PathBuf::from("/tmp/a.wav"), PathBuf::from("/tmp/b.wav")],
            lang: crate::api::LanguageSpec::Resolved(LanguageCode3::eng()),
            num_speakers: NumSpeakers(2),
            max_concurrent: 2,
        };

        let result = submit_with(
            &plan,
            Arc::new(|request| {
                if request.audio_path.ends_with("a.wav") {
                    // PathBuf ends_with works for last component
                    Ok("job-a".to_string())
                } else {
                    Err("boom".to_string())
                }
            }),
        )
        .await;

        assert_eq!(
            result
                .job_ids
                .get(&PathBuf::from("/tmp/a.wav"))
                .map(|id| &**id),
            Some("job-a")
        );
        assert_eq!(
            result.errors.get("/tmp/b.wav").map(|s| s.as_str()),
            Some("boom")
        );
    }

    #[tokio::test]
    async fn preflight_honors_max_concurrency_guard() {
        let plan = RevAiPreflightPlan {
            audio_paths: vec![
                PathBuf::from("/tmp/a.wav"),
                PathBuf::from("/tmp/b.wav"),
                PathBuf::from("/tmp/c.wav"),
            ],
            lang: crate::api::LanguageSpec::Resolved(LanguageCode3::eng()),
            num_speakers: NumSpeakers(1),
            max_concurrent: 1,
        };

        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let result = submit_with(
            &plan,
            Arc::new({
                let in_flight = in_flight.clone();
                let peak = peak.clone();
                move |request| {
                    let now = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    Ok(format!("job-for-{}", request.audio_path.display()))
                }
            }),
        )
        .await;

        assert_eq!(peak.load(Ordering::SeqCst), 1);
        assert_eq!(result.job_ids.len(), 3);
    }

    #[test]
    fn language_hint_maps_common_codes() {
        assert_eq!(
            RevAiLanguageHint::from(&LanguageCode3::eng()).as_str(),
            "en"
        );
        assert_eq!(
            RevAiLanguageHint::from(&LanguageCode3::spa()).as_str(),
            "es"
        );
        assert_eq!(
            RevAiLanguageHint::from(&LanguageCode3::zho()).as_str(),
            "cmn"
        );
    }

    // RED: skip_postprocessing semantics per Rev.AI docs
    //
    // Rev.AI's `skip_postprocessing` parameter skips "inverse text
    // normalization (ITN), casing and punctuation" and is "Only available
    // for English and Spanish languages" (verbatim from
    // docs.rev.ai/api/asynchronous/reference/jobs/submittranscriptionjob).
    //
    // ITN turns spoken form into written form: "eighty percent" → "80%",
    // "seventeen year old" → "17-year-old", "one hundred" → "100".
    // CHAT records spoken form, so we want ITN *skipped* for the languages
    // where the flag is available. For unsupported languages we don't
    // send the flag (it has no effect and makes the request shape noisier).
    #[test]
    fn skip_postprocessing_hint_is_some_true_for_en_and_es_only() {
        assert_eq!(skip_postprocessing_hint("en"), Some(true));
        assert_eq!(skip_postprocessing_hint("es"), Some(true));
        assert_eq!(skip_postprocessing_hint("cmn"), None);
        assert_eq!(skip_postprocessing_hint("fr"), None);
        assert_eq!(skip_postprocessing_hint("auto"), None);
    }
}
