//! Runner-owned job execution mutations on [`JobStore`].

use crate::api::{JobId, JobStatus, UnixTimestamp};

use super::super::registry::JobCompletionSnapshot;
use super::super::{JobStore, PersistedJobUpdate};

impl JobStore {
    /// Re-queue a job after memory-gate rejection and persist the retry deadline.
    pub(crate) async fn requeue_job_after_memory_gate(
        &self,
        job_id: &JobId,
        retry_at: UnixTimestamp,
    ) {
        let Some(job_update) = self
            .registry
            .requeue_after_memory_gate(job_id, retry_at)
            .await
        else {
            return;
        };
        self.notify_job_item(job_update);

        self.db_update_job(
            job_id,
            PersistedJobUpdate {
                status: JobStatus::Queued,
                error: None,
                completed_at: None,
                num_workers: None,
                next_eligible_at: Some(retry_at),
            },
        )
        .await;
    }

    /// Mark a job as running and clear any deferred retry deadline.
    pub(crate) async fn mark_job_running(&self, job_id: &JobId) {
        let Some(job_update) = self.registry.mark_job_running(job_id).await else {
            return;
        };
        self.notify_job_item(job_update);

        self.db_update_job(
            job_id,
            PersistedJobUpdate {
                status: JobStatus::Running,
                error: None,
                completed_at: None,
                num_workers: None,
                next_eligible_at: None,
            },
        )
        .await;
    }

    /// Record the per-job worker count chosen for this run.
    pub(crate) async fn record_job_worker_count(&self, job_id: &JobId, num_workers: usize) {
        if !self
            .registry
            .record_job_worker_count(job_id, num_workers)
            .await
        {
            return;
        }

        self.db_update_job(
            job_id,
            PersistedJobUpdate {
                status: JobStatus::Running,
                error: None,
                completed_at: None,
                num_workers: Some(num_workers as i32),
                next_eligible_at: None,
            },
        )
        .await;
    }

    /// Fail a job immediately with a job-level error message.
    pub(crate) async fn fail_job(&self, job_id: &JobId, error: &str, completed_at: UnixTimestamp) {
        let Some(job_update) = self.registry.fail_job(job_id, error, completed_at).await else {
            return;
        };
        self.notify_job_item(job_update);

        self.db_update_job(
            job_id,
            PersistedJobUpdate {
                status: JobStatus::Failed,
                error: Some(error),
                completed_at: Some(completed_at),
                num_workers: None,
                next_eligible_at: None,
            },
        )
        .await;
    }

    /// Return the cancellation and aggregate file-outcome facts for one job.
    pub(crate) async fn completion_snapshot(
        &self,
        job_id: &JobId,
    ) -> Option<JobCompletionSnapshot> {
        self.registry.completion_snapshot(job_id).await
    }

    /// Finalize a job after all file tasks have stopped mutating its state.
    ///
    /// Returns the job-level failure reason (the per-file errors `Job::finalize`
    /// aggregated) so the runner can log it at the right level; `None` for
    /// non-failed jobs. The reason is also broadcast on the `JobListItem` and
    /// persisted to the `jobs.db` `error` column; that column was hardcoded
    /// `None` before the 2026-06 silent-failure fix, so jobs.db / the dashboard
    /// showed "failed" with no cause.
    pub(crate) async fn finalize_job(
        &self,
        job_id: &JobId,
        final_status: JobStatus,
        completed_at: UnixTimestamp,
    ) -> Option<String> {
        let job_update = self
            .registry
            .finalize_job(job_id, final_status, completed_at)
            .await?;
        let job_error = job_update.error.clone();
        self.notify_job_item(job_update);

        self.db_update_job(
            job_id,
            PersistedJobUpdate {
                status: final_status,
                error: job_error.as_deref(),
                completed_at: Some(completed_at),
                num_workers: None,
                next_eligible_at: None,
            },
        )
        .await;

        job_error
    }
}

#[cfg(test)]
mod tests {
    use tokio::sync::broadcast;

    use crate::api::{FileStatusKind, JobId, JobStatus, ReleasedCommand, UnixTimestamp};
    use crate::store::queries::tests::{make_job, test_config};
    use crate::ws::BROADCAST_CAPACITY;

    use super::*;

    /// Re-queued jobs retain queued status and gain the next retry timestamp.
    #[tokio::test]
    async fn requeue_job_after_memory_gate_sets_retry_deadline() {
        let (tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
        let store = JobStore::new(test_config(), None, tx);
        store
            .submit(make_job(
                "job-1",
                ReleasedCommand::Morphotag,
                vec!["a.cha".into()],
            ))
            .await
            .unwrap();

        let retry_at = UnixTimestamp(321.0);
        store
            .requeue_job_after_memory_gate(&JobId::from("job-1"), retry_at)
            .await;

        let info = store.get(&JobId::from("job-1")).await.unwrap();
        assert_eq!(info.status, JobStatus::Queued);
        assert_eq!(info.next_eligible_at, Some(retry_at));
    }

    /// Finalization recomputes completed file counts from terminal file state.
    #[tokio::test]
    async fn finalize_job_recounts_terminal_files() {
        let (tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
        let store = JobStore::new(test_config(), None, tx);
        let mut job = make_job(
            "job-1",
            ReleasedCommand::Morphotag,
            vec!["a.cha".into(), "b.cha".into()],
        );
        job.execution.file_statuses.get_mut("a.cha").unwrap().status = FileStatusKind::Done;
        store.submit(job).await.unwrap();

        store
            .finalize_job(
                &JobId::from("job-1"),
                JobStatus::Completed,
                UnixTimestamp(500.0),
            )
            .await;

        let info = store.get(&JobId::from("job-1")).await.unwrap();
        assert_eq!(info.status, JobStatus::Completed);
        assert_eq!(info.completed_files, 1);
    }

    /// A job that fails because its files failed must surface WHY: the per-file
    /// error messages are aggregated into the job-level error, which is shown
    /// in the in-memory projection (the `/jobs` + dashboard source) AND
    /// persisted to the `jobs.db` `error` column so it survives a reload.
    ///
    /// Regression for the 2026-06 silent-failure bug: a job that failed because
    /// its (Stanza-skew-affected) files failed was recorded with an EMPTY
    /// `error` column and no job-level reason, so the dashboard/jobs.db showed
    /// "failed" with no cause; the only place the real error appeared was the
    /// CLI client response.
    #[tokio::test]
    async fn finalize_failed_job_records_reason_from_file_failures() {
        use std::sync::Arc;

        use crate::db::JobDB;
        use crate::scheduling::FailureCategory;
        use crate::store::FileFailureRecord;

        let dir = tempfile::tempdir().unwrap();
        let db = Arc::new(JobDB::open(Some(dir.path())).await.unwrap());
        let (tx, _rx) = broadcast::channel(BROADCAST_CAPACITY);
        let store = JobStore::new(test_config(), Some(db.clone()), tx);

        let job_id = JobId::from("job-fail");
        store
            .submit(make_job(
                "job-fail",
                ReleasedCommand::Morphotag,
                vec!["a.cha".into(), "b.cha".into()],
            ))
            .await
            .unwrap();

        // Both files fail with the verbatim worker message, the shape of the
        // Stanza manifest-skew failure that exposed this bug.
        let failure = FileFailureRecord {
            message: "worker bootstrap error: ensure_task failed: md5 mismatch".into(),
            category: FailureCategory::WorkerBootstrap,
            finished_at: UnixTimestamp(400.0),
        };
        store.mark_file_error(&job_id, "a.cha", &failure).await;
        store.mark_file_error(&job_id, "b.cha", &failure).await;

        let failure_reason = store
            .finalize_job(&job_id, JobStatus::Failed, UnixTimestamp(500.0))
            .await;

        // (0) finalize_job returns the aggregated reason so the runner can log
        //     it (and to confirm the JobListItem projection carries the error,
        //     which is where this return value is read from).
        assert!(
            failure_reason
                .as_deref()
                .is_some_and(|reason| reason.contains("md5 mismatch")),
            "finalize_job should return the failure reason, got: {failure_reason:?}"
        );

        // (1) In-memory projection (the `/jobs` endpoint + dashboard source)
        //     must carry the reason, not a bare "failed".
        let info = store.get(&job_id).await.unwrap();
        assert_eq!(info.status, JobStatus::Failed);
        let in_memory = info
            .error
            .expect("a failed job must record a job-level error reason (dashboard/jobs)");
        assert!(
            in_memory.contains("md5 mismatch"),
            "in-memory job error should name the file failure, got: {in_memory}"
        );

        // (2) The `jobs.db` `error` column must persist it: a fresh store
        //     reloaded purely from the DB still sees the reason.
        let (tx2, _rx2) = broadcast::channel(BROADCAST_CAPACITY);
        let store2 = JobStore::new(test_config(), Some(db.clone()), tx2);
        store2.load_from_db().await.unwrap();
        let reloaded = store2.get(&job_id).await.expect("reloaded job present");
        let persisted = reloaded
            .error
            .expect("jobs.db `error` column must be populated for a failed job");
        assert!(
            persisted.contains("md5 mismatch"),
            "persisted job error should name the file failure, got: {persisted}"
        );
    }
}
