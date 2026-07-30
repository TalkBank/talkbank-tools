//! Batch-progress reporting for a morphotag job.
//!
//! One reporter per job owns the accounting and the publishing cadence; each
//! per-file task holds a cheap [`BackendProgressPort`] that stamps its own
//! provenance onto every report. Nothing here is shared mutable state: the
//! ledger lives in the drain task and reaches it only over a channel.
//!
//! ```text
//! dispatch_morphotag_job
//!   ├── BatchProgressReporter::spawn(sink, job_id)      ── owns the ledger
//!   ├── per file: reporter.port(filename) ──▶ MorphosyntaxParams.progress
//!   │                                            └─▶ infer_batch ─▶ per chunk
//!   └── reporter.finish().await                    (drains, publishes, exits)
//! ```
//!
//! # History
//!
//! `15c88de2` (2026-04-28) had a working drain loop in the legacy batched-text
//! dispatch, and a 61-line `BatchProgressReporter` written to replace it that
//! nothing ever constructed. `e8235c13` (2026-05-03) deleted the working loop
//! and left the unused successor behind, so the feature died with a designed
//! replacement sitting dead on arrival: the "frozen mid-sentence" state the
//! rearchitecture charter describes. This is that successor, built for the
//! per-file-fanout architecture that replaced the pooled one, with the
//! aggregation defect fixed (see [`crate::runner::util::batch_progress`]).

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::api::{DisplayPath, JobId, LanguageCode3};
use crate::runner::util::batch_progress::{
    BackendProgress, BatchChunkIndex, BatchProgressLedger, SourceProgress,
};
use crate::runner::util::{FileStage, RunnerEventSink};
use crate::types::worker_v2::ProgressEventV2;

/// How often a changed snapshot is published.
///
/// The Python backend already throttles to at most one event per second per
/// request, so this bounds the store writes when many files report at once
/// rather than the wire traffic.
const PUBLISH_INTERVAL: Duration = Duration::from_secs(2);

/// How long without any new report before the last snapshot is republished.
///
/// A republish is how an operator tells "stalled" from "finished": the numbers
/// stay put but keep arriving. This is the heartbeat the 2026-04 code called
/// its stall detector, kept at the same 120s.
const STALL_REPUBLISH_INTERVAL: Duration = Duration::from_secs(120);

/// Channel depth for backend reports.
///
/// Reports are throttled per request and a job's chunk count is bounded by the
/// worker pool, so this is generous; a full channel drops the least useful
/// event (see [`BackendProgressPort::report`]) rather than blocking inference.
const REPORT_CHANNEL_DEPTH: usize = 256;

/// Owns one job's batch-progress accounting and publishing.
pub(crate) struct BatchProgressReporter {
    /// Kept so [`Self::port`] can hand out clones.
    tx: Option<mpsc::Sender<BackendProgress>>,
    /// Explicit shutdown signal for the drain task.
    ///
    /// Shutdown is NOT inferred from the channel closing. Every port holds a
    /// sender clone, so "the channel closed" means "no port survives anywhere",
    /// and a caller holding one port one line too long would make `finish()`
    /// wait forever. A hang is the worst failure mode available here, so the
    /// drain gets told to stop rather than left to deduce it. Caught by
    /// `finish_does_not_wait_for_outstanding_ports`.
    shutdown: CancellationToken,
    drain: tokio::task::JoinHandle<()>,
}

impl BatchProgressReporter {
    /// Start a reporter for one job.
    pub(crate) fn spawn(sink: Arc<dyn RunnerEventSink>, job_id: JobId) -> Self {
        let (tx, rx) = mpsc::channel::<BackendProgress>(REPORT_CHANNEL_DEPTH);
        let shutdown = CancellationToken::new();
        let drain = tokio::spawn(drain_reports(sink, job_id, rx, shutdown.clone()));
        Self {
            tx: Some(tx),
            shutdown,
            drain,
        }
    }

    /// A port for one input file, or `None` once [`Self::finish`] has run.
    pub(crate) fn port(&self, source_id: DisplayPath) -> Option<BackendProgressPort> {
        self.tx.as_ref().map(|tx| BackendProgressPort {
            source_id,
            tx: tx.clone(),
        })
    }

    /// Close the channel and wait for the drain task to publish what is left.
    ///
    /// Awaited rather than detached: the final snapshot is the one showing every
    /// group complete, and a dropped task would race the job's own completion so
    /// the last thing an operator saw could be a partial count.
    pub(crate) async fn finish(mut self) {
        drop(self.tx.take());
        self.shutdown.cancel();
        if let Err(error) = self.drain.await {
            tracing::warn!(error = %error, "batch progress drain task panicked");
        }
    }
}

/// One input file's handle for reporting backend progress.
///
/// Cloneable and cheap: this is what threads down through
/// `MorphosyntaxParams` to the chunk dispatch, so it deliberately knows only
/// its own file identity and nothing about jobs, stores or sinks.
#[derive(Clone)]
pub(crate) struct BackendProgressPort {
    source_id: DisplayPath,
    tx: mpsc::Sender<BackendProgress>,
}

impl BackendProgressPort {
    /// Report one wire progress event for one chunk of one language group.
    ///
    /// Uses `try_send`: a progress report must never slow inference or, worse,
    /// deadlock it if the drain task is gone. A dropped report costs at most one
    /// stale second on a display, and the next report supersedes it, because the
    /// ledger stores the latest value per chunk rather than accumulating deltas.
    /// That is a property of the aggregation, not luck.
    pub(crate) fn report(
        &self,
        group: &LanguageCode3,
        chunk: BatchChunkIndex,
        event: &ProgressEventV2,
    ) {
        let progress =
            BackendProgress::from_event(self.source_id.clone(), group.clone(), chunk, event);
        match self.tx.try_send(progress) {
            Ok(()) => {}
            Err(mpsc::error::TrySendError::Full(_)) => {
                tracing::debug!(
                    source_id = %self.source_id,
                    group = %group,
                    "dropped a batch progress report: channel full"
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                tracing::debug!(
                    source_id = %self.source_id,
                    "dropped a batch progress report: reporter already finished"
                );
            }
        }
    }
}

/// Accumulate reports and publish snapshots until the channel closes.
async fn drain_reports(
    sink: Arc<dyn RunnerEventSink>,
    job_id: JobId,
    mut rx: mpsc::Receiver<BackendProgress>,
    shutdown: CancellationToken,
) {
    let mut ledger = BatchProgressLedger::new();
    let mut unpublished_change = false;
    let mut last_publish = Instant::now();

    loop {
        let received = tokio::select! {
            // Bias toward draining queued reports, so a shutdown arriving with
            // reports still in the channel does not discard the newest numbers.
            biased;
            received = tokio::time::timeout(PUBLISH_INTERVAL, rx.recv()) => received,
            () = shutdown.cancelled() => {
                // Take whatever is already queued, then publish and stop.
                while let Ok(progress) = rx.try_recv() {
                    unpublished_change |= ledger.record(progress);
                }
                if unpublished_change {
                    publish(sink.as_ref(), &job_id, &ledger).await;
                }
                return;
            }
        };

        match received {
            Ok(Some(progress)) => {
                unpublished_change |= ledger.record(progress);
            }
            Ok(None) => {
                // Every port is gone: publish the final state and stop.
                if unpublished_change {
                    publish(sink.as_ref(), &job_id, &ledger).await;
                }
                return;
            }
            Err(_timed_out) => {}
        }

        if unpublished_change && last_publish.elapsed() >= PUBLISH_INTERVAL {
            publish(sink.as_ref(), &job_id, &ledger).await;
            unpublished_change = false;
            last_publish = Instant::now();
        } else if !unpublished_change && last_publish.elapsed() >= STALL_REPUBLISH_INTERVAL {
            publish(sink.as_ref(), &job_id, &ledger).await;
            last_publish = Instant::now();
        }
    }
}

/// Publish both projections of the ledger.
async fn publish(sink: &dyn RunnerEventSink, job_id: &JobId, ledger: &BatchProgressLedger) {
    sink.set_batch_progress(job_id, ledger.snapshot()).await;

    for source in ledger.source_progress() {
        // Skip a file whose utterances are all accounted for. Its row is about
        // to go terminal (or already has), and the store refuses progress on a
        // terminal file anyway; not sending is clearer than being refused.
        if source.completed.0 >= source.total.0 {
            continue;
        }
        publish_source(sink, job_id, &source).await;
    }
}

/// Publish one file's utterance counts on the ordinary file-progress channel.
///
/// This is the projection that gives an operator watching one long file a
/// denominator, and it deliberately reuses `set_file_progress` rather than
/// inventing a second per-file surface: `FileStage::Analyzing` is already what
/// the CLI spinner, the TUI and the dashboard row render for this stage.
async fn publish_source(sink: &dyn RunnerEventSink, job_id: &JobId, source: &SourceProgress) {
    sink.set_file_progress(
        job_id,
        source.source_id.as_ref(),
        FileStage::Analyzing,
        Some(i64::try_from(source.completed.0).unwrap_or(i64::MAX)),
        Some(i64::try_from(source.total.0).unwrap_or(i64::MAX)),
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::util::batch_progress::BatchChunkIndex;
    use crate::runner::util::test_sink::RecordingSink;
    use batchalign_types::worker_v2::WorkerRequestIdV2;

    fn event(completed: u32, total: u32) -> ProgressEventV2 {
        ProgressEventV2 {
            request_id: WorkerRequestIdV2::from("req"),
            completed,
            total,
            stage: "stanza_processing".to_string(),
        }
    }

    fn lang(code: &str) -> LanguageCode3 {
        LanguageCode3::try_new(code).expect("valid test language")
    }

    /// The reporter must publish BOTH projections: the job-level per-language
    /// snapshot and the per-file utterance counts.
    ///
    /// The per-file half is the one that had no coverage anywhere: it is what
    /// gives an operator watching a single long file a denominator, and a live
    /// ML test cannot assert it reliably because a fast file emits only its
    /// final event.
    #[tokio::test]
    async fn reporter_publishes_both_projections() {
        let sink = Arc::new(RecordingSink::default());
        let job_id = JobId::from("job-1");
        let reporter = BatchProgressReporter::spawn(sink.clone(), job_id.clone());

        let port = reporter
            .port(DisplayPath::from("a.cha"))
            .expect("a fresh reporter must hand out ports");
        // Mid-flight: 40 of 100 utterances done in this file's only chunk.
        port.report(&lang("eng"), BatchChunkIndex(0), &event(40, 100));
        reporter.finish().await;

        let snapshots = sink.batch_snapshots();
        let last = snapshots
            .last()
            .expect("the reporter must publish at least once");
        let group = last
            .language_groups
            .get(&lang("eng"))
            .expect("eng group must be present");
        assert_eq!(group.completed_utterances.0, 40);
        assert_eq!(group.total_utterances.0, 100);

        let file_writes = sink.progress();
        let write = file_writes
            .iter()
            .find(|w| w.filename == "a.cha")
            .expect("an incomplete file must get per-file utterance counts");
        assert_eq!(write.current, Some(40));
        assert_eq!(write.total, Some(100));
        assert_eq!(write.stage, FileStage::Analyzing);
    }

    /// A file whose chunks are all finished must not get another per-file write.
    ///
    /// Its row is about to go terminal and the store refuses progress on a
    /// terminal file, so sending would be noise that only shows up as a refused
    /// write. The job-level snapshot still counts it.
    #[tokio::test]
    async fn a_finished_file_gets_no_further_per_file_write() {
        let sink = Arc::new(RecordingSink::default());
        let reporter = BatchProgressReporter::spawn(sink.clone(), JobId::from("job-2"));

        let port = reporter
            .port(DisplayPath::from("done.cha"))
            .expect("port must exist");
        port.report(&lang("eng"), BatchChunkIndex(0), &event(100, 100));
        reporter.finish().await;

        assert!(
            sink.progress().is_empty(),
            "a complete file must not receive a per-file progress write"
        );
        let snapshots = sink.batch_snapshots();
        let last = snapshots.last().expect("job-level snapshot must publish");
        assert_eq!(last.completed_utterances().0, 100);
    }

    /// Chunk keying survives the whole path, not just the ledger: two chunks of
    /// one file sum instead of overwriting.
    #[tokio::test]
    async fn two_chunks_of_one_file_sum_through_the_reporter() {
        let sink = Arc::new(RecordingSink::default());
        let reporter = BatchProgressReporter::spawn(sink.clone(), JobId::from("job-3"));

        let port = reporter
            .port(DisplayPath::from("big.cha"))
            .expect("port must exist");
        port.report(&lang("eng"), BatchChunkIndex(0), &event(150, 274));
        port.report(&lang("eng"), BatchChunkIndex(1), &event(120, 274));
        reporter.finish().await;

        let snapshots = sink.batch_snapshots();
        let last = snapshots.last().expect("job-level snapshot must publish");
        let group = last
            .language_groups
            .get(&lang("eng"))
            .expect("eng group must be present");
        assert_eq!(group.total_utterances.0, 548, "chunk totals must sum");
        assert_eq!(group.completed_utterances.0, 270);
    }

    /// `finish()` must not wait for outstanding ports.
    ///
    /// This is the test that caught the original design: `finish()` dropped its
    /// own sender and awaited the drain, but a live port holds a sender clone, so
    /// the channel never closed and `finish()` hung forever. Production happened
    /// to be safe (the fanout is joined before `finish`), which is the worst kind
    /// of safe.
    #[tokio::test]
    async fn finish_does_not_wait_for_outstanding_ports() {
        let sink = Arc::new(RecordingSink::default());
        let reporter = BatchProgressReporter::spawn(sink.clone(), JobId::from("job-5"));
        let port = reporter
            .port(DisplayPath::from("held.cha"))
            .expect("port must exist");
        port.report(&lang("eng"), BatchChunkIndex(0), &event(5, 50));

        // The port is deliberately still alive here.
        tokio::time::timeout(Duration::from_secs(5), reporter.finish())
            .await
            .expect("finish must not wait on an outstanding port");

        assert_eq!(
            sink.batch_snapshots().len(),
            1,
            "the queued report must still be published on shutdown"
        );
        drop(port);
    }

    /// Ports are refused after `finish`, so a late report cannot resurrect a
    /// finished job's progress.
    #[tokio::test]
    async fn a_dropped_port_reports_nothing() {
        let sink = Arc::new(RecordingSink::default());
        let reporter = BatchProgressReporter::spawn(sink.clone(), JobId::from("job-4"));
        let port = reporter.port(DisplayPath::from("a.cha")).expect("port");
        reporter.finish().await;

        // The drain is gone; this must be dropped, not panic or block.
        port.report(&lang("eng"), BatchChunkIndex(0), &event(10, 100));
        assert!(sink.progress().is_empty());
    }
}
