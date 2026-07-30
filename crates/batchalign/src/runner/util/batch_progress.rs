//! Batch-level progress for the batched-text commands.
//!
//! # Why a job-level progress surface exists at all
//!
//! Per-file progress (`set_file_progress`) answers "which file, what stage".
//! It cannot answer "how far into this file's inference are we", because a
//! file's utterances are handed to a text backend as one batch and nothing
//! comes back until the batch does. So during `FileStage::Analyzing` a file
//! has a stage and no numbers, which is the largest remaining violation of
//! the crate's time-transparency rule (see
//! `book/src/batchalign/architecture/time-transparency.md`): every audio
//! command publishes counts, and the batched-text path did not.
//!
//! The backend already produces the missing numbers.
//! `batchalign/worker/_text_v2.py` installs an `_on_progress` callback that
//! emits a `progress_v2` event at most once per second per request, carrying
//! `completed` / `total` utterances. This module is the accounting that turns
//! that stream into the two things a UI can render.
//!
//! # Two projections, one ledger
//!
//! ```text
//! Python backend ──progress_v2──▶ worker handle ──▶ BackendProgress
//!                                                        │
//!                                            BatchProgressLedger
//!                                              │            │
//!                    per-group projection ◀────┘            └────▶ per-source projection
//!                    BatchInferProgress                            SourceProgress
//!                    (job-level: the dashboard                     (per-file counts on the
//!                     BatchProgressPanel, the CLI                   existing file-progress
//!                     summary line, the API field)                  channel)
//! ```
//!
//! Both come from the same events, so they cannot disagree. That matters:
//! the previous design published only the job-level view, and the per-file
//! view it did not feed is the one an operator watching a single long file
//! actually needs.
//!
//! # Why the key is (source, group, chunk)
//!
//! This is the correction that makes the feature trustworthy, and it is worth
//! reading before changing anything here.
//!
//! `morphosyntax::worker::infer_batch_homogeneous` splits one language group
//! into up to `max_workers_per_key` chunks whenever it holds `2 *
//! MIN_CHUNK_SIZE` items or more, which on any multi-worker host is always.
//! Each chunk is a separate backend request reporting its OWN `completed` /
//! `total`. The 2026-04 ledger keyed on language alone, took the first
//! event's `total` as the denominator, and let each later event overwrite
//! `completed`. With four chunks of 274 items that displayed `453/274`, i.e.
//! 165%, which is exactly the overflow recorded in this module's history and
//! observed live before the feature was removed on 2026-05-03.
//!
//! Keying on the CHUNK rather than the request id is deliberate:
//! `dispatch_execute_v2_with_retry_and_progress` retries a chunk under a
//! fresh request id, so a request-keyed ledger would count a retried chunk's
//! utterances twice and keep the abandoned attempt's stale count forever. The
//! chunk is the stable unit of work; the request is one attempt at it.
//!
//! Totals are therefore SUMMED across chunks and the latest report per chunk
//! wins, which is correct under both chunking and retry.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::api::{DisplayPath, LanguageCode3, UtteranceCount};
use crate::types::worker_v2::ProgressEventV2;

/// Index of one chunk within a single (source, language-group) batch.
///
/// Not a request id: a chunk retried after a worker failure keeps its index
/// and gets a new request id, and the ledger must treat that as the same unit
/// of work. See the module docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct BatchChunkIndex(pub(crate) u32);

/// Progress reported by one backend for one chunk of work.
///
/// Named `BackendProgress` after the same concept in the upstream fork whose
/// mechanisms this crate is adopting: where that fork already names a concept
/// being introduced here, its name is taken, so the two codebases stay
/// comparable at a glance. `source_id` likewise borrows its vocabulary for
/// input provenance, typed with our own `DisplayPath` rather than a bare
/// string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackendProgress {
    /// Which input file this work came from.
    pub(crate) source_id: DisplayPath,
    /// The homogeneous batch group, i.e. the language the backend is running.
    pub(crate) group: LanguageCode3,
    /// Which chunk of that group's items this report covers.
    pub(crate) chunk: BatchChunkIndex,
    /// Utterances finished in this chunk so far.
    pub(crate) completed: UtteranceCount,
    /// Utterances in this chunk in total.
    pub(crate) total: UtteranceCount,
}

impl BackendProgress {
    /// Lift one wire-protocol progress event into a domain message.
    ///
    /// The event's `stage` field is deliberately IGNORED. The 2026-04 producer
    /// smuggled the language code through it by rewriting `event.stage` in
    /// flight, so a field named "stage" carried a language by a convention no
    /// type enforced. Provenance now arrives as typed fields supplied by the
    /// dispatch site, which knows them for certain.
    pub(crate) fn from_event(
        source_id: DisplayPath,
        group: LanguageCode3,
        chunk: BatchChunkIndex,
        event: &ProgressEventV2,
    ) -> Self {
        Self {
            source_id,
            group,
            chunk,
            completed: UtteranceCount(u64::from(event.completed)),
            total: UtteranceCount(u64::from(event.total)),
        }
    }
}

/// Identity of one unit of batched work within a job.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ChunkKey {
    source_id: DisplayPath,
    group: LanguageCode3,
    chunk: BatchChunkIndex,
}

/// The latest report for one chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChunkProgress {
    completed: UtteranceCount,
    total: UtteranceCount,
}

/// Running account of a job's batched inference.
///
/// Owned by the reporter task (`execution::morphotag::progress`), never
/// shared: every mutation arrives as a `BackendProgress` on a channel, so
/// there is no lock here by construction.
#[derive(Debug, Default)]
pub(crate) struct BatchProgressLedger {
    chunks: BTreeMap<ChunkKey, ChunkProgress>,
}

impl BatchProgressLedger {
    /// Create an empty ledger.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Record one backend report.
    ///
    /// Returns whether this changed the ledger, so the caller can skip
    /// republishing an identical snapshot. A chunk's later report replaces its
    /// earlier one; a retried chunk replaces its own abandoned attempt.
    pub(crate) fn record(&mut self, progress: BackendProgress) -> bool {
        let key = ChunkKey {
            source_id: progress.source_id,
            group: progress.group,
            chunk: progress.chunk,
        };
        let next = ChunkProgress {
            completed: progress.completed,
            total: progress.total,
        };
        match self.chunks.insert(key, next) {
            Some(previous) => previous != next,
            None => true,
        }
    }

    /// Project the ledger onto the job-level, per-language API shape.
    pub(crate) fn snapshot(&self) -> BatchInferProgress {
        let mut language_groups: BTreeMap<LanguageCode3, LanguageGroupProgress> = BTreeMap::new();
        for (key, chunk) in &self.chunks {
            let entry = language_groups
                .entry(key.group.clone())
                .or_insert_with(|| LanguageGroupProgress::empty(key.group.clone()));
            entry.add(*chunk);
        }
        BatchInferProgress { language_groups }
    }

    /// Project the ledger onto per-source totals, for the file-progress
    /// channel every other command already publishes on.
    pub(crate) fn source_progress(&self) -> Vec<SourceProgress> {
        let mut per_source: BTreeMap<&DisplayPath, SourceProgress> = BTreeMap::new();
        for (key, chunk) in &self.chunks {
            let entry = per_source
                .entry(&key.source_id)
                .or_insert_with(|| SourceProgress {
                    source_id: key.source_id.clone(),
                    completed: UtteranceCount(0),
                    total: UtteranceCount(0),
                });
            entry.completed = UtteranceCount(entry.completed.0 + chunk.completed.0);
            entry.total = UtteranceCount(entry.total.0 + chunk.total.0);
        }
        per_source.into_values().collect()
    }
}

/// Aggregate utterance counts for one input file.
///
/// A named struct rather than a `(DisplayPath, u64, u64)` tuple: two counts of
/// the same primitive type next to each other is exactly the seam where
/// completed and total get swapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceProgress {
    /// The input file these counts belong to.
    pub(crate) source_id: DisplayPath,
    /// Utterances finished across all of this file's chunks.
    pub(crate) completed: UtteranceCount,
    /// Utterances in total across all of this file's chunks.
    pub(crate) total: UtteranceCount,
}

/// Progress for one language group within a batched infer job.
///
/// Wire-compatible with the pre-2026-07 shape: `LanguageCode3` and
/// `UtteranceCount` are both `#[serde(transparent)]` newtypes, so the JSON is
/// byte-identical to the `String` / `u64` version the dashboard and the
/// generated TypeScript already consume.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct LanguageGroupProgress {
    /// ISO 639-3 language code (e.g. `"fra"`, `"eng"`).
    pub lang: LanguageCode3,
    /// Number of utterances completed so far.
    pub completed_utterances: UtteranceCount,
    /// Total utterances in this language group.
    pub total_utterances: UtteranceCount,
}

impl LanguageGroupProgress {
    /// A group with no work recorded yet.
    fn empty(lang: LanguageCode3) -> Self {
        Self {
            lang,
            completed_utterances: UtteranceCount(0),
            total_utterances: UtteranceCount(0),
        }
    }

    /// Fold one chunk's counts into this group.
    fn add(&mut self, chunk: ChunkProgress) {
        self.completed_utterances = UtteranceCount(self.completed_utterances.0 + chunk.completed.0);
        self.total_utterances = UtteranceCount(self.total_utterances.0 + chunk.total.0);
    }

    /// Whether this language group has finished processing.
    pub fn is_complete(&self) -> bool {
        self.completed_utterances.0 >= self.total_utterances.0
    }

    /// Progress as a fraction in `[0.0, 1.0]`.
    ///
    /// An empty group reads as complete rather than as 0%: a group with no
    /// utterances has nothing left to do.
    pub fn fraction(&self) -> f64 {
        if self.total_utterances.0 == 0 {
            1.0
        } else {
            self.completed_utterances.0 as f64 / self.total_utterances.0 as f64
        }
    }
}

/// Aggregate progress for a batched infer job across all language groups.
///
/// This is a PROJECTION of [`BatchProgressLedger`], not a thing callers
/// mutate. The 2026-04 version exposed `register_group` / `update_group` /
/// `complete_group`, and those mutators are what made the double-counting bug
/// expressible: they invited last-write-wins updates keyed on language, which
/// is the wrong granularity. Build one of these with
/// `BatchProgressLedger::snapshot` instead.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[cfg_attr(feature = "server", derive(utoipa::ToSchema))]
pub struct BatchInferProgress {
    /// Per-language-group progress, keyed by language code.
    /// `BTreeMap` for deterministic JSON.
    pub language_groups: BTreeMap<LanguageCode3, LanguageGroupProgress>,
}

impl BatchInferProgress {
    /// Total utterances across all language groups.
    pub fn total_utterances(&self) -> UtteranceCount {
        UtteranceCount(
            self.language_groups
                .values()
                .map(|g| g.total_utterances.0)
                .sum(),
        )
    }

    /// Total completed utterances across all language groups.
    pub fn completed_utterances(&self) -> UtteranceCount {
        UtteranceCount(
            self.language_groups
                .values()
                .map(|g| g.completed_utterances.0)
                .sum(),
        )
    }

    /// Overall progress as a fraction in `[0.0, 1.0]`.
    pub fn overall_fraction(&self) -> f64 {
        let total = self.total_utterances().0;
        if total == 0 {
            1.0
        } else {
            self.completed_utterances().0 as f64 / total as f64
        }
    }

    /// Whether all language groups have finished.
    pub fn is_complete(&self) -> bool {
        self.language_groups.values().all(|g| g.is_complete())
    }

    /// Language codes for groups that have not yet completed.
    ///
    /// This is what a stall report names: with the ledger keyed per chunk, an
    /// incomplete group here means real outstanding utterances, not an
    /// artefact of one chunk's numbers overwriting another's.
    pub fn incomplete_groups(&self) -> Vec<&LanguageCode3> {
        self.language_groups
            .values()
            .filter(|g| !g.is_complete())
            .map(|g| &g.lang)
            .collect()
    }

    /// Number of language groups still in progress.
    pub fn active_groups(&self) -> usize {
        self.language_groups
            .values()
            .filter(|g| !g.is_complete())
            .count()
    }

    /// Human-readable summary for CLI display.
    ///
    /// Example: `"3/5 languages done, 1200/1800 utterances (67%)"`.
    pub fn summary(&self) -> String {
        let total_groups = self.language_groups.len();
        let complete_groups = total_groups - self.active_groups();
        let completed = self.completed_utterances().0;
        let total = self.total_utterances().0;
        let pct = (100u64.saturating_mul(completed))
            .checked_div(total)
            .map(|v| v as u32)
            .unwrap_or(100);
        format!(
            "{complete_groups}/{total_groups} languages done, \
             {completed}/{total} utterances ({pct}%)"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::worker_v2::ProgressEventV2;
    use batchalign_types::worker_v2::WorkerRequestIdV2;

    fn event(request: &str, completed: u32, total: u32) -> ProgressEventV2 {
        ProgressEventV2 {
            request_id: WorkerRequestIdV2::from(request),
            completed,
            total,
            // The real Python producer hard-codes this label for every event;
            // the ledger must never depend on it.
            stage: "stanza_processing".to_string(),
        }
    }

    fn lang(code: &str) -> LanguageCode3 {
        LanguageCode3::try_new(code).expect("test language code must be a valid ISO 639-3 code")
    }

    fn report(
        ledger: &mut BatchProgressLedger,
        source: &str,
        group: &str,
        chunk: u32,
        completed: u32,
        total: u32,
    ) -> bool {
        ledger.record(BackendProgress::from_event(
            DisplayPath::from(source),
            lang(group),
            BatchChunkIndex(chunk),
            &event("req", completed, total),
        ))
    }

    /// THE REGRESSION THIS MODULE EXISTS FOR.
    ///
    /// One language group split across four chunks of 274 items each. The
    /// 2026-04 ledger keyed on language alone, so the denominator was the
    /// first chunk's 274 and the numerator summed every chunk's completions:
    /// `453/274`, 165%, observed live. Summing per chunk gives 1,096 as the
    /// denominator, and completed can never exceed it.
    #[test]
    fn chunked_group_totals_sum_instead_of_overwriting() {
        let mut ledger = BatchProgressLedger::new();
        for chunk in 0..4 {
            report(&mut ledger, "a.cha", "eng", chunk, 0, 274);
        }
        report(&mut ledger, "a.cha", "eng", 0, 274, 274);
        report(&mut ledger, "a.cha", "eng", 1, 179, 274);

        let snapshot = ledger.snapshot();
        let group = &snapshot.language_groups[&lang("eng")];
        assert_eq!(group.total_utterances, UtteranceCount(1_096));
        assert_eq!(group.completed_utterances, UtteranceCount(453));
        assert!(
            group.completed_utterances.0 <= group.total_utterances.0,
            "completed must never exceed total: {group:?}"
        );
    }

    /// A retried chunk must replace its own abandoned attempt, not add to it.
    ///
    /// `dispatch_execute_v2_with_retry_and_progress` reissues a failed chunk
    /// under a fresh request id. Keying the ledger on the request id would
    /// double the denominator and strand the dead attempt's count; keying on
    /// the chunk index makes the retry overwrite.
    #[test]
    fn retried_chunk_replaces_its_abandoned_attempt() {
        let mut ledger = BatchProgressLedger::new();
        report(&mut ledger, "a.cha", "eng", 0, 90, 300);
        // Same chunk, new attempt, counts restart from zero.
        report(&mut ledger, "a.cha", "eng", 0, 10, 300);

        let snapshot = ledger.snapshot();
        let group = &snapshot.language_groups[&lang("eng")];
        assert_eq!(group.total_utterances, UtteranceCount(300));
        assert_eq!(group.completed_utterances, UtteranceCount(10));
    }

    /// Distinct languages stay distinct, and the wire-protocol `stage` label
    /// they all share cannot merge them.
    #[test]
    fn languages_do_not_collapse_on_the_shared_stage_label() {
        let mut ledger = BatchProgressLedger::new();
        report(&mut ledger, "eng.cha", "eng", 0, 100, 100);
        report(&mut ledger, "hrv.cha", "hrv", 0, 100, 209);
        report(&mut ledger, "cat.cha", "cat", 0, 130, 274);

        let snapshot = ledger.snapshot();
        assert_eq!(
            snapshot.language_groups.len(),
            3,
            "expected eng + hrv + cat, got {:?}",
            snapshot.language_groups.keys().collect::<Vec<_>>()
        );
        // The old collapse is now UNREPRESENTABLE rather than merely absent:
        // the shared stage label cannot even be built into a group key, because
        // `LanguageCode3` validates its input.
        assert!(
            LanguageCode3::try_new("stanza_processing").is_err(),
            "the wire stage label must not be constructible as a language group"
        );
        assert_eq!(snapshot.total_utterances(), UtteranceCount(583));

        let incomplete = snapshot.incomplete_groups();
        assert_eq!(
            incomplete.len(),
            2,
            "expected hrv + cat, got {incomplete:?}"
        );
    }

    /// The same events must also answer "how far into THIS file are we",
    /// which is the projection the 2026-04 design never fed.
    #[test]
    fn per_source_projection_sums_that_file_only() {
        let mut ledger = BatchProgressLedger::new();
        report(&mut ledger, "a.cha", "eng", 0, 50, 100);
        report(&mut ledger, "a.cha", "eng", 1, 25, 100);
        report(&mut ledger, "b.cha", "spa", 0, 10, 40);

        let sources = ledger.source_progress();
        assert_eq!(sources.len(), 2);
        let a = sources
            .iter()
            .find(|s| s.source_id == DisplayPath::from("a.cha"))
            .expect("a.cha must be present");
        assert_eq!(a.completed, UtteranceCount(75));
        assert_eq!(a.total, UtteranceCount(200));
        let b = sources
            .iter()
            .find(|s| s.source_id == DisplayPath::from("b.cha"))
            .expect("b.cha must be present");
        assert_eq!(b.completed, UtteranceCount(10));
        assert_eq!(b.total, UtteranceCount(40));
    }

    /// Two files in the same language belong to one job-level group but stay
    /// separate per-source rows. This is the multilingual-job shape the
    /// dashboard panel renders, and the single-language case that used to be
    /// dismissed as "one bar, not worth it".
    #[test]
    fn one_group_can_span_several_sources() {
        let mut ledger = BatchProgressLedger::new();
        report(&mut ledger, "a.cha", "eng", 0, 10, 100);
        report(&mut ledger, "b.cha", "eng", 0, 40, 100);

        let snapshot = ledger.snapshot();
        assert_eq!(snapshot.language_groups.len(), 1);
        let group = &snapshot.language_groups[&lang("eng")];
        assert_eq!(group.completed_utterances, UtteranceCount(50));
        assert_eq!(group.total_utterances, UtteranceCount(200));
        assert_eq!(ledger.source_progress().len(), 2);
    }

    /// Republishing an unchanged snapshot is wasted work, so `record` reports
    /// whether it changed anything.
    #[test]
    fn record_reports_whether_the_ledger_changed() {
        let mut ledger = BatchProgressLedger::new();
        assert!(report(&mut ledger, "a.cha", "eng", 0, 10, 100));
        assert!(!report(&mut ledger, "a.cha", "eng", 0, 10, 100));
        assert!(report(&mut ledger, "a.cha", "eng", 0, 11, 100));
    }

    #[test]
    fn empty_ledger_is_complete_and_empty() {
        let snapshot = BatchProgressLedger::new().snapshot();
        assert!(snapshot.is_complete());
        assert_eq!(snapshot.total_utterances(), UtteranceCount(0));
        assert_eq!(snapshot.active_groups(), 0);
        assert_eq!(snapshot.overall_fraction(), 1.0);
    }

    #[test]
    fn summary_reads_as_a_sentence() {
        let mut ledger = BatchProgressLedger::new();
        report(&mut ledger, "a.cha", "eng", 0, 1000, 1000);
        report(&mut ledger, "b.cha", "fra", 0, 250, 500);
        report(&mut ledger, "c.cha", "deu", 0, 0, 300);

        let summary = ledger.snapshot().summary();
        assert!(summary.contains("1/3 languages done"), "got: {summary}");
        assert!(summary.contains("1250/1800"), "got: {summary}");
        assert!(summary.contains("69%"), "got: {summary}");
    }

    /// The JSON contract the dashboard and the generated TypeScript consume
    /// must not change: transparent newtypes, alphabetical keys.
    #[test]
    fn json_shape_is_unchanged_and_deterministic() {
        let mut ledger = BatchProgressLedger::new();
        report(&mut ledger, "c.cha", "fra", 0, 5, 50);
        report(&mut ledger, "a.cha", "eng", 0, 10, 100);
        report(&mut ledger, "b.cha", "deu", 0, 0, 30);

        let json = serde_json::to_string(&ledger.snapshot()).expect("snapshot must serialize");
        assert_eq!(
            json,
            r#"{"language_groups":{"deu":{"lang":"deu","completed_utterances":0,"total_utterances":30},"eng":{"lang":"eng","completed_utterances":10,"total_utterances":100},"fra":{"lang":"fra","completed_utterances":5,"total_utterances":50}}}"#
        );
        let back: BatchInferProgress =
            serde_json::from_str(&json).expect("snapshot must round-trip");
        assert_eq!(back, ledger.snapshot());
    }
}
