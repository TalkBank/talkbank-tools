//! Shared recording [`RunnerEventSink`] for tests.
//!
//! One mock, not one per test module: the sink has ~20 methods, and a second
//! hand-written copy is a second thing to forget to update when the trait grows.
//! `set_batch_progress` was added to the trait on 2026-07-29 and the duplicate
//! that existed then had to be patched twice.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::api::{DisplayPath, JobId, JobStatus, UnixTimestamp};
use crate::scheduling::{AttemptOutcome, FailureCategory, RetryDisposition, WorkUnitKind};
use crate::store::CompletedFileOutput;

use super::FileStage;
use super::event_sink::RunnerEventSink;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RecordedProgress {
    pub(crate) job_id: JobId,
    pub(crate) filename: String,
    pub(crate) stage: FileStage,
    pub(crate) current: Option<i64>,
    pub(crate) total: Option<i64>,
}

#[derive(Default)]
pub(crate) struct RecordingSink {
    progress: Mutex<Vec<RecordedProgress>>,
    batch_snapshots: Mutex<Vec<crate::runner::util::batch_progress::BatchInferProgress>>,
}

#[async_trait]
impl RunnerEventSink for RecordingSink {
    async fn mark_file_processing(
        &self,
        _job_id: &JobId,
        _filename: &str,
        _started_at: UnixTimestamp,
    ) {
    }

    async fn mark_file_done(
        &self,
        _job_id: &JobId,
        _filename: &str,
        _finished_at: UnixTimestamp,
        _result: Option<CompletedFileOutput>,
    ) {
    }

    async fn mark_file_error(
        &self,
        _job_id: &JobId,
        _filename: &str,
        _error: &str,
        _category: FailureCategory,
        _finished_at: UnixTimestamp,
    ) {
    }

    async fn start_file_attempt(
        &self,
        _job_id: &JobId,
        _filename: &str,
        _work_unit_kind: WorkUnitKind,
        _started_at: UnixTimestamp,
    ) {
    }

    async fn finish_file_attempt(
        &self,
        _job_id: &JobId,
        _filename: &str,
        _outcome: AttemptOutcome,
        _failure_category: Option<FailureCategory>,
        _disposition: RetryDisposition,
        _finished_at: UnixTimestamp,
    ) {
    }

    async fn mark_file_retry_pending(
        &self,
        _job_id: &JobId,
        _filename: &str,
        _retry_at: UnixTimestamp,
        _category: FailureCategory,
        _message: &str,
        _finished_at: UnixTimestamp,
    ) {
    }

    async fn clear_file_retry_state(&self, _job_id: &JobId, _filename: &str) {}

    async fn set_file_progress(
        &self,
        job_id: &JobId,
        filename: &str,
        stage: FileStage,
        current: Option<i64>,
        total: Option<i64>,
    ) {
        self.progress
            .lock()
            .expect("progress lock")
            .push(RecordedProgress {
                job_id: job_id.clone(),
                filename: filename.to_string(),
                stage,
                current,
                total,
            });
    }

    async fn set_batch_progress(
        &self,
        _job_id: &JobId,
        progress: crate::runner::util::batch_progress::BatchInferProgress,
    ) {
        self.batch_snapshots
            .lock()
            .expect("batch snapshots lock")
            .push(progress);
    }

    async fn unfinished_files(&self, _job_id: &JobId) -> Vec<DisplayPath> {
        Vec::new()
    }

    async fn file_status_label(&self, _job_id: &JobId, _filename: &str) -> Option<String> {
        None
    }

    async fn bump_forced_terminal_errors(&self, _count: usize) {}

    async fn fail_job(&self, _job_id: &JobId, _error: &str, _failed_at: UnixTimestamp) {}

    async fn mark_job_running(&self, _job_id: &JobId) {}

    async fn record_job_worker_count(&self, _job_id: &JobId, _worker_count: usize) {}

    async fn requeue_job_after_memory_gate(&self, _job_id: &JobId, _retry_at: UnixTimestamp) {}

    async fn bump_deferred_work_units(&self) {}

    async fn bump_memory_gate_aborts(&self) {}

    async fn finalize_job(
        &self,
        _job_id: &JobId,
        _expected_generation: crate::store::RunGeneration,
        _final_status: JobStatus,
        _completed_at: UnixTimestamp,
    ) -> Option<String> {
        None
    }
}

impl RecordingSink {
    /// Every file-progress write the sink received, in order.
    pub(crate) fn progress(&self) -> Vec<RecordedProgress> {
        self.progress.lock().expect("progress lock").clone()
    }

    /// Every batch-progress snapshot the sink received, in order.
    pub(crate) fn batch_snapshots(
        &self,
    ) -> Vec<crate::runner::util::batch_progress::BatchInferProgress> {
        self.batch_snapshots
            .lock()
            .expect("batch snapshots lock")
            .clone()
    }
}
