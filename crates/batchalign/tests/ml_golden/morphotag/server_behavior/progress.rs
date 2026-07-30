use super::super::helpers::repeated_chat;
use crate::common::{LiveServerJobClient, require_live_server};
use batchalign::api::{FilePayload, FileProgressStage, JobStatus, LanguageSpec, ReleasedCommand};
use batchalign::options::{CommandOptions, CommonOptions, MorphotagOptions};
use batchalign::worker::InferTask;

/// Verify the new morphotag architecture still surfaces per-language batch
/// progress for mixed-language jobs rather than collapsing everything onto one
/// generic group.
///
/// **This test was dark from 2026-05-06 to 2026-07-29.** It submitted
/// `LanguageSpec::Resolved(eng)`, and `c2236ab3` (2026-05-06) made the server
/// reject a job-level language for morphotag with HTTP 400, so every run died
/// at submission without ever reaching the behaviour under test. `PerFile` is
/// also the honest spec for a multilingual job: each fixture file declares its
/// own `@Languages`, and per-file resolution is the whole point of the test.
/// Note the dates: `e8235c13` (2026-05-03) removed the progress producer and
/// this ban landed three days later, so the feature and its only test died in
/// the same restructure.
#[tokio::test]
async fn morphotag_multilingual_job_reports_separate_batch_progress_groups() {
    let Some(server) = require_live_server(
        InferTask::Morphosyntax,
        "Server does not support morphosyntax infer",
    )
    .await
    else {
        return;
    };
    let jobs = LiveServerJobClient::from_session(&server);

    let initial = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            LanguageSpec::PerFile,
            vec![
                FilePayload {
                    filename: "english.cha".into(),
                    content: repeated_chat("eng", "ENG", "the dog runs", 220),
                },
                FilePayload {
                    filename: "spanish.cha".into(),
                    content: repeated_chat("spa", "SPA", "el perro corre", 220),
                },
            ],
            CommandOptions::Morphotag(MorphotagOptions {
                common: CommonOptions {
                    override_media_cache: true,
                    batch_window: 0,
                    ..CommonOptions::default()
                },

                ..Default::default()
            }),
        )
        .await;

    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(180);
    let mut observed_groups = std::collections::BTreeSet::new();

    loop {
        let info = jobs.job_info(&initial.job_id).await;
        if let Some(progress) = &info.batch_progress {
            observed_groups.extend(progress.language_groups.keys().cloned());
        }
        if info.status.is_terminal() || tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    let final_info = jobs.poll_done(&initial.job_id).await;
    assert_eq!(
        final_info.status,
        JobStatus::Completed,
        "multilingual morphotag job should complete"
    );
    assert!(
        observed_groups.contains("eng"),
        "expected eng batch progress group, observed {observed_groups:?}"
    );
    assert!(
        observed_groups.contains("spa"),
        "expected spa batch progress group, observed {observed_groups:?}"
    );
}

/// Verify an operator can see a long morphotag file ADVANCING, not just
/// "Analyzing" with no numbers.
///
/// **Rewritten 2026-07-29, and the reason matters.** This test used to assert
/// that a `--batch-window` job reported progress on `FileProgressStage::Parsing`.
/// Two things it depended on do not exist:
///
/// 1. `FileProgressStage::Parsing` has NO producer anywhere in the crate. It is
///    declared in the API enum and referenced only by the TUI's colour map, so
///    the assertion could never have passed.
/// 2. `batch_window` is parsed from `--batch-window`, stored in options and
///    serialized, and then read by NOTHING in the execution path. The windowing
///    it names is not part of the per-file-fanout architecture that replaced the
///    pooled one. That is an operator-visible flag doing nothing, recorded for
///    adjudication rather than quietly deleted here.
///
/// What now exists, and what this asserts: per-file utterance counts published
/// from the batch-progress ledger's per-source projection, on the ordinary
/// file-progress channel every other command uses. That is the signal an
/// operator watching one long file actually needs.
#[tokio::test]
async fn morphotag_job_reports_per_file_utterance_progress() {
    let Some(server) = require_live_server(
        InferTask::Morphosyntax,
        "Server does not support morphosyntax infer",
    )
    .await
    else {
        return;
    };
    let jobs = LiveServerJobClient::from_session(&server);

    // Long enough that inference spans several progress events (the backend
    // throttles to one per second per request).
    let files: Vec<FilePayload> = (0..2)
        .map(|idx| FilePayload {
            filename: format!("window_{idx}.cha").into(),
            content: repeated_chat("eng", "PAR", "the window test runs", 400),
        })
        .collect();

    let initial = jobs
        .submit_content_job(
            ReleasedCommand::Morphotag,
            LanguageSpec::PerFile,
            files,
            CommandOptions::Morphotag(MorphotagOptions {
                common: CommonOptions {
                    override_media_cache: true,
                    ..CommonOptions::default()
                },

                ..Default::default()
            }),
        )
        .await;

    let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_secs(180);
    let mut max_current = 0;
    let mut max_total = 0;

    loop {
        let info = jobs.job_info(&initial.job_id).await;
        for file in &info.file_statuses {
            if file.progress_stage == Some(FileProgressStage::Analyzing) {
                max_current = max_current.max(file.progress_current.unwrap_or(0));
                max_total = max_total.max(file.progress_total.unwrap_or(0));
            }
        }
        if info.status.is_terminal() || tokio::time::Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    let final_info = jobs.poll_done(&initial.job_id).await;
    assert_eq!(
        final_info.status,
        JobStatus::Completed,
        "morphotag job should complete"
    );
    // Deliberately does NOT assert that counts were observed. Whether an
    // intermediate report exists depends on the file taking more than a second
    // of inference (the backend throttles to one event per second per request,
    // and always emits the final one), and on the poll loop sampling while it is
    // live. Asserting presence here would be a race dressed up as coverage.
    // Presence is pinned deterministically instead, in
    // `execution::morphotag::progress`'s own tests. What IS invariant is that
    // anything observed must be coherent.
    assert!(
        max_current <= max_total,
        "completed utterances must never exceed the total (the 453/274 defect); \
         observed current={max_current} total={max_total}"
    );
    if max_total > 0 {
        assert!(
            max_current > 0,
            "a reported denominator with a zero numerator means the projection \
             published an empty row; observed current={max_current} total={max_total}"
        );
    }
}
