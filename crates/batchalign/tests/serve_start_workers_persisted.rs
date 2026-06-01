//! Subprocess regression test: `serve start --workers N` must propagate
//! into the per-job worker count recorded for submitted batches.
//!
//! Scenario:
//!
//! ```text
//! batchalign3 serve stop
//! batchalign3 serve start --workers 4
//! batchalign3 transcribe <multi-file batch> ...
//! ```
//!
//! On hosts where `gpu.is_functional_for_batchalign()` returns false
//! (Apple Silicon with MPS excluded, hosts without CUDA, hosts that
//! set `--force-cpu` explicitly), the recommended
//! `gpu_thread_pool_size` collapses to 1. Without this regression
//! guard the planner clamps every GPU-tagged command (transcribe,
//! align, benchmark) to a single worker per job, silently shadowing
//! the operator's explicit `--workers N`.
//!
//! See [`compute_workers_force_cpu_treats_gpu_commands_as_cpu_bound`]
//! in `runner::util` for the same invariant pinned at the planner
//! level.
//!
//! Run: `cargo nextest run -p batchalign --test serve_start_workers_persisted`
//! (skips gracefully if Python with batchalign deps is unavailable).

mod cli_common;

use batchalign::api::{FilePayload, JobInfo, JobSubmission, NumSpeakers, ReleasedCommand};
use batchalign::api::{LanguageCode3, LanguageSpec};
use batchalign::options::{
    AsrEngineName, CommandOptions, CommonOptions, TranscribeOptions,
};

use cli_common::{CliHarness, poll_job_done, resolve_python};

/// Pin the observable Spencer saw broken: `serve start --workers 4`
/// followed by a 6-file batch must result in `JobInfo.num_workers >= 2`
/// (ideally `Some(4)`).
///
/// A 6-file batch is deliberately above the single-file `num_workers=1`
/// floor: `granted_workers` cannot exceed file count, so `<= 4 files`
/// would be a confound. With 6 files and `--workers 4`, the planner has
/// no excuse to grant fewer than 4.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serve_start_workers_propagates_to_submitted_job_num_workers() {
    let Some(python) = resolve_python() else {
        eprintln!("SKIP: Python 3 with batchalign deps not available");
        return;
    };

    let harness = CliHarness::new();

    // Pick a non-default port to avoid colliding with any local daemon
    // running on the operator's machine. Range chosen to be high and
    // unlikely to overlap other tests.
    let port: u16 = 19500 + (std::process::id() as u16 % 500);
    let config = format!(
        "host: 127.0.0.1\nport: {port}\nwarmup_commands: []\nauto_daemon: false\n"
    );
    std::fs::write(harness.server_config_path(), &config)
        .expect("write server.yaml");

    // Explicit background-mode `serve start --workers 4`. NOT
    // `transcribe`, which would go through the separate
    // `ensure_daemon_locked` auto-start path and miss this seam.
    let start_output = harness
        .cmd()
        .args([
            "serve",
            "start",
            "--test-echo",
            "--python",
            &python,
            "--port",
            &port.to_string(),
            "--workers",
            "4",
            "--config",
            harness.server_config_path().to_str().unwrap(),
        ])
        .timeout(std::time::Duration::from_secs(15))
        .output();

    match &start_output {
        Ok(o) if !o.status.success() => {
            eprintln!(
                "SKIP: serve start failed (port {port} conflict?): stderr={:?}",
                String::from_utf8_lossy(&o.stderr)
            );
            return;
        }
        Err(e) => {
            eprintln!("SKIP: serve start errored: {e}");
            return;
        }
        _ => {}
    }

    let base_url = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::new();

    // Poll for daemon health (test-echo warm-up can take several seconds
    // on first launch).
    let ready = wait_for_health(&client, &base_url, 30).await;
    if !ready {
        stop_daemon(&harness);
        panic!("daemon never became healthy on port {port}");
    }

    // Submit a 6-file batch (paths_mode=false, embedded content) so the
    // job planner has 6 work units to distribute across the daemon's
    // workers.
    let submission = JobSubmission {
        command: ReleasedCommand::Transcribe,
        lang: LanguageSpec::Resolved(LanguageCode3::eng()),
        num_speakers: NumSpeakers(1),
        files: (1..=6)
            .map(|i| FilePayload {
                filename: format!("f{i}.cha").into(),
                content: "@UTF8\n@Begin\n*CHI:\thello .\n@End\n".into(),
            })
            .collect(),
        media_files: vec![],
        media_mapping: Default::default(),
        media_subdir: Default::default(),
        source_dir: Default::default(),
        // test-echo workers bypass real ASR; engine name is required by the
        // submission schema but not actually invoked.
        options: CommandOptions::Transcribe(TranscribeOptions {
            common: CommonOptions::default(),
            asr_engine: AsrEngineName::RevAi,
            diarize: false,
            wor: false.into(),
            merge_abbrev: false.into(),
            batch_size: 8,
            utseg_fallback: false.into(),
        }),
        paths_mode: false,
        source_paths: vec![],
        output_paths: vec![],
        display_names: vec![],
        debug_traces: false,
        before_paths: vec![],
    };

    let resp = client
        .post(format!("{base_url}/jobs"))
        .json(&submission)
        .send()
        .await
        .expect("POST /jobs");
    assert!(
        resp.status().is_success(),
        "job submission failed: {}",
        resp.status()
    );
    let info: JobInfo = resp.json().await.expect("parse JobInfo");

    let final_info = poll_job_done(&client, &base_url, &info.job_id).await;

    // Cleanup: stop daemon before assertion so a failure doesn't leak a
    // background process across test runs.
    stop_daemon(&harness);

    // The intent of `--workers 4` on the explicit serve-start path is
    // that the daemon grants up to 4 parallel workers per job. With 6
    // files in the batch, `granted_workers` should be exactly 4 (the
    // requested cap, not file-count-limited).
    assert_eq!(
        final_info.num_workers,
        Some(4),
        "`serve start --workers 4` did not propagate to job execution. \
         Expected num_workers=4 on a 6-file batch, got {:?}. \
         Job status: {:?}.",
        final_info.num_workers,
        final_info.status,
    );
}

async fn wait_for_health(
    client: &reqwest::Client,
    base_url: &str,
    timeout_s: u64,
) -> bool {
    let deadline =
        std::time::Instant::now() + std::time::Duration::from_secs(timeout_s);
    while std::time::Instant::now() < deadline {
        if let Ok(resp) = client.get(format!("{base_url}/health")).send().await
            && resp.status().is_success()
        {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    false
}

fn stop_daemon(harness: &CliHarness) {
    let _ = harness
        .cmd()
        .args(["serve", "stop"])
        .timeout(std::time::Duration::from_secs(10))
        .output();
}
