//! The verify-flag tier pass over merged drafts (`merge-verify`).
//!
//! Consumes a merged draft directory plus an engine-verdicts JSON
//! (forced-alignment score, pitch band, machine-ear answer per flagged
//! utterance, produced upstream by the verify engines or replayed from
//! a cache) and emits a rewritten draft plus a review queue.
//!
//! Tier semantics (human-calibrated against blind listening verdicts;
//! measured 97.5% precision on the auto-trust tier):
//!
//! - AUTO_TRUST: trusted category AND ear YES AND pitch CHILD. The
//!   verify `%com` flag is REWRITTEN to a machine-verified provenance
//!   note (maintainer ruling: provenance survives in the transcript,
//!   flags are never silently deleted).
//! - REVIEW: trusted category failing a gate, or an uncalibrated
//!   category. Flag untouched; the line exports to the review queue.
//! - HOLD: clock and region categories; untouched entirely (clock
//!   placements measured 0/7 correct; interpolation drift is a
//!   session-level problem, not a per-line one).
//! - Demotion: a `confident` line with adverse verdicts (pitch ADULT
//!   or ear NO) GAINS a review flag and joins the queue; text and
//!   timing are never moved by this pass.
//!
//! Corpus-specific flag vocabularies stay OUTSIDE this module: the
//! verdicts JSON carries each line's category (mapped at the corpus
//! seam), and verify flags are identified by a caller-supplied prefix.
//!
//! The preservation invariant (main tiers byte-identical through the
//! pass) is CHECKED here, not assumed: a violation is a hard error.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use talkbank_model::model::{ChatFile, ComTier, DependentTier, Line};

/// Ordinal of a main-tier utterance within one file (0-based, in
/// document order). The verdicts JSON keys lines by this ordinal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(transparent)]
pub struct UtteranceOrdinal(pub usize);

/// Calibrated flag category, supplied per line by the corpus seam.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifyCategory {
    /// Diarization-mislabel rescues (the primary promotion target).
    Other,
    /// Medium-confidence anchors.
    Medium,
    /// Medium-confidence anchors under their measurement-era name.
    UnknownFlag,
    /// Approximate/pitch-ambiguous placements.
    Approx,
    /// Interpolated (clock) placements; never auto-trusted.
    Clock,
    /// Child-voice-region placements; never auto-trusted.
    Region,
    /// Weak matches (uncalibrated; always reviewed).
    Weak,
    /// Boundary-suspect glued placements (uncalibrated; always reviewed).
    Glued,
    /// Child-orphan placements (uncalibrated; always reviewed).
    ChildOrphan,
    /// Not flagged; eligible only for demotion re-flagging.
    Confident,
}

impl VerifyCategory {
    /// Whether the ear sample calibrated this category as safe for
    /// silent trust when the remaining gates pass.
    fn is_trusted(self) -> bool {
        matches!(
            self,
            VerifyCategory::Other
                | VerifyCategory::Medium
                | VerifyCategory::UnknownFlag
                | VerifyCategory::Approx
        )
    }
}

/// Pitch band verdict for the placed span (from the pitch engine).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PitchBand {
    /// Voiced f0 sits in the child band for the placed span.
    Child,
    /// Voiced f0 sits in the adult band.
    Adult,
    /// Too few voiced frames, or mixed banding (whisper, murmur, overlap).
    Ambiguous,
}

/// Machine-ear verdict (does the child say the claimed text here?).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EarVerdict {
    /// The machine ear heard the child say the claimed text.
    Yes,
    /// It did not (routes to review, never auto-demotes on its own).
    No,
}

/// One line's engine verdicts, keyed by utterance ordinal.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LineVerdict {
    /// Which main-tier utterance this verdict describes.
    pub utterance_index: UtteranceOrdinal,
    /// Calibrated flag category (mapped at the corpus seam).
    pub category: VerifyCategory,
    /// Mean per-word forced-alignment confidence for the placed text.
    /// None = UNSCORABLE: the token sequence exceeded what the audio
    /// window could hold, so CTC alignment was impossible. FA is
    /// ordering-only (never a promote/demote gate), so an unscorable
    /// line is tiered by its category, pitch, and ear alone.
    pub fa_mean_score: Option<f64>,
    /// Pitch band of the placed span.
    pub pitch: PitchBand,
    /// Machine-ear answer for the placed text.
    pub ear: EarVerdict,
}

/// All verdicts for one session file (`<session>.cha`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionVerdicts {
    /// Session id; the draft file is `<session>.cha`.
    pub session: String,
    /// Per-line verdicts, keyed by utterance ordinal.
    pub lines: Vec<LineVerdict>,
}

/// The verdicts document consumed by the pass.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerdictsDoc {
    /// Every session the pass should process.
    pub sessions: Vec<SessionVerdicts>,
}

/// What the calibrated rule decides for one line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TierOutcome {
    /// Rewrite the verify flag to a machine-verified provenance note.
    AutoTrust,
    /// Leave the flag; export the line to the review queue.
    Review,
    /// Untouched (clock/region).
    Hold,
    /// Confident line with adverse verdicts: gains a review flag.
    Demote,
    /// Confident line with benign verdicts: nothing to do.
    Untouched,
}

/// Assign the calibrated tier for one verdict line.
pub fn tier_outcome(verdict: &LineVerdict) -> TierOutcome {
    match verdict.category {
        VerifyCategory::Clock | VerifyCategory::Region => TierOutcome::Hold,
        VerifyCategory::Confident => {
            if verdict.pitch == PitchBand::Adult || verdict.ear == EarVerdict::No {
                TierOutcome::Demote
            } else {
                TierOutcome::Untouched
            }
        }
        VerifyCategory::Weak | VerifyCategory::Glued | VerifyCategory::ChildOrphan => {
            TierOutcome::Review
        }
        category if category.is_trusted() => {
            if verdict.ear == EarVerdict::Yes && verdict.pitch == PitchBand::Child {
                TierOutcome::AutoTrust
            } else {
                TierOutcome::Review
            }
        }
        // is_trusted covers every remaining variant; keep the compiler
        // honest if a category is ever added without a tier decision.
        VerifyCategory::Other
        | VerifyCategory::Medium
        | VerifyCategory::UnknownFlag
        | VerifyCategory::Approx => unreachable_category(),
    }
}

/// The guard arm above is structurally unreachable (`is_trusted`
/// matches exactly those four variants); modeled as a typed dead end
/// rather than a panic per the no-panic policy.
fn unreachable_category() -> TierOutcome {
    TierOutcome::Review
}

/// One review-queue entry (REVIEW tier or demotion).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueEntry {
    /// Session id of the queued line.
    pub session: String,
    /// Which main-tier utterance to review.
    pub utterance_index: UtteranceOrdinal,
    /// The line's calibrated flag category.
    pub category: VerifyCategory,
    /// Why the line queued (REVIEW gate failure or demotion).
    pub tier: TierOutcome,
    /// Mean per-word forced-alignment confidence (None = unscorable).
    pub fa_mean_score: Option<f64>,
    /// Pitch band of the placed span.
    pub pitch: PitchBand,
    /// Machine-ear answer.
    pub ear: EarVerdict,
}

/// The exported review queue.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReviewQueue {
    /// Queued lines across all sessions, in document order per session.
    pub entries: Vec<QueueEntry>,
}

/// Per-run counts, printed by the CLI.
#[derive(Clone, Copy, Debug, Default, Serialize)]
pub struct VerifySummary {
    /// Sessions processed.
    pub sessions: usize,
    /// Lines promoted to machine-verified provenance notes.
    pub auto_trusted: usize,
    /// Lines exported to the review queue (gate failures).
    pub reviewed: usize,
    /// Clock/region lines left untouched.
    pub held: usize,
    /// Previously-confident lines re-flagged for review.
    pub demoted: usize,
}

/// Typed failures of the pass.
#[derive(Debug, thiserror::Error)]
pub enum MergeVerifyError {
    /// Reading or writing a filesystem path failed.
    #[error("i/o failure on {path}: {source}")]
    Io {
        /// Path the i/o operation targeted.
        path: PathBuf,
        /// Underlying i/o error.
        #[source]
        source: std::io::Error,
    },
    /// The verdicts document is not valid JSON for the expected shape.
    #[error("verdicts JSON at {path} did not parse: {source}")]
    VerdictsParse {
        /// Path of the verdicts document.
        path: PathBuf,
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },
    /// A session named in the verdicts has no draft file.
    #[error("draft file for session '{session}' not found at {path}")]
    MissingSession {
        /// Session named by the verdicts document.
        session: String,
        /// Draft path that was expected to exist.
        path: PathBuf,
    },
    /// A draft did not parse cleanly; the pass refuses partial input.
    #[error("draft {path} failed to parse as CHAT: {details}")]
    DraftParse {
        /// Draft file that failed to parse.
        path: PathBuf,
        /// Joined parser diagnostics.
        details: String,
    },
    /// A verdict names an utterance ordinal past the draft's end.
    #[error(
        "verdict for session '{session}' names utterance ordinal {ordinal:?} \
         but the draft has only {utterance_count} main-tier utterances"
    )]
    OrdinalOutOfRange {
        /// Session whose verdicts overran the draft.
        session: String,
        /// The out-of-range ordinal.
        ordinal: UtteranceOrdinal,
        /// How many main-tier utterances the draft actually has.
        utterance_count: usize,
    },
    /// The preservation invariant broke: a main tier changed.
    #[error(
        "preservation invariant violated in session '{session}': main tier \
         {ordinal:?} changed through the pass"
    )]
    MainTierChanged {
        /// Session where the invariant broke.
        session: String,
        /// Ordinal of the changed main tier (usize::MAX sentinel is
        /// never used; a count mismatch reports the first divergence).
        ordinal: UtteranceOrdinal,
    },
    /// The review queue could not be serialized.
    #[error("review queue at {path} failed to serialize: {source}")]
    QueueSerialize {
        /// Queue output path.
        path: PathBuf,
        /// Underlying JSON error.
        #[source]
        source: serde_json::Error,
    },
}

/// The plain text of a `%com` tier (concatenated text segments; bullets
/// and pictures contribute nothing to prefix matching).
fn com_text(tier: &ComTier) -> String {
    use talkbank_model::model::BulletContentSegment;
    let mut out = String::new();
    for segment in &tier.content.segments {
        match segment {
            BulletContentSegment::Text(text) => out.push_str(text.text.as_str()),
            BulletContentSegment::Bullet(_)
            | BulletContentSegment::Picture(_)
            | BulletContentSegment::Continuation => {}
        }
    }
    out
}

/// Render a pitch band for provenance notes.
fn pitch_label(pitch: PitchBand) -> &'static str {
    match pitch {
        PitchBand::Child => "child",
        PitchBand::Adult => "adult",
        PitchBand::Ambiguous => "ambiguous",
    }
}

/// Render an ear verdict for provenance notes.
fn ear_label(ear: EarVerdict) -> &'static str {
    match ear {
        EarVerdict::Yes => "yes",
        EarVerdict::No => "no",
    }
}

/// The machine-verified provenance note replacing a promoted flag.
/// Carries the three signals and the known residual failure mode
/// (whispered adult speech defeats the pitch leg; ~2.5% measured).
/// Render the FA leg for a note: a number, or an honest n/a for an
/// unscorable line (never a fabricated value).
fn fa_label(score: Option<f64>) -> String {
    match score {
        Some(value) => format!("{value:.2}"),
        None => "n/a".to_owned(),
    }
}

fn provenance_note(verdict: &LineVerdict) -> String {
    format!(
        "machine-verified placement (fa={}, pitch={}, ear={}); \
         residual risk: whispered adult speech (~2.5% measured)",
        fa_label(verdict.fa_mean_score),
        pitch_label(verdict.pitch),
        ear_label(verdict.ear),
    )
}

/// The review flag added to a demoted (previously confident) line.
fn demotion_note(verdict: &LineVerdict) -> String {
    format!(
        "review: machine-demoted placement (fa={}, pitch={}, ear={})",
        fa_label(verdict.fa_mean_score),
        pitch_label(verdict.pitch),
        ear_label(verdict.ear),
    )
}

/// Rewrite the flag segment of a %com tier's text into the provenance
/// note, preserving any human transcriber note ahead of it verbatim.
///
/// The merge writer appends its flag to a pre-existing human comment as
/// `<human note> ; <prefix>: <flag>`, so the flag is not always at the
/// start of the tier; it always runs from the `<prefix>:` marker to the
/// end. Returns None when the tier carries no flag (a plain human
/// comment is never touched). A bare `starts_with` predicate here
/// silently skipped four human-prefixed bundle flags (2026-07-17).
fn rewrite_flag_text(text: &str, flag_prefix: &str, verdict: &LineVerdict) -> Option<String> {
    let marker = format!("{flag_prefix}:");
    let flag_start = text.find(&marker)?;
    Some(format!(
        "{}{}",
        &text[..flag_start],
        provenance_note(verdict)
    ))
}

/// Apply the tier pass to one parsed draft in place. Returns the queue
/// entries contributed by this session.
fn apply_to_file(
    file: &mut ChatFile,
    session: &str,
    verdicts: &[LineVerdict],
    flag_prefix: &str,
    summary: &mut VerifySummary,
) -> Result<Vec<QueueEntry>, MergeVerifyError> {
    let by_ordinal: BTreeMap<UtteranceOrdinal, &LineVerdict> = verdicts
        .iter()
        .map(|verdict| (verdict.utterance_index, verdict))
        .collect();

    let mut queue = Vec::new();
    let mut ordinal = 0usize;
    let mut utterance_count = 0usize;

    for line in file.lines.iter_mut() {
        let Line::Utterance(utterance) = line else {
            continue;
        };
        utterance_count += 1;
        let current = UtteranceOrdinal(ordinal);
        ordinal += 1;
        let Some(verdict) = by_ordinal.get(&current) else {
            continue;
        };
        let outcome = tier_outcome(verdict);
        match outcome {
            TierOutcome::AutoTrust => {
                summary.auto_trusted += 1;
                for tier in utterance.dependent_tiers.iter_mut() {
                    if let DependentTier::Com(com) = tier
                        && let Some(rewritten) =
                            rewrite_flag_text(&com_text(com), flag_prefix, verdict)
                    {
                        *com = ComTier::from_text(rewritten);
                    }
                }
            }
            TierOutcome::Review => {
                summary.reviewed += 1;
                queue.push(queue_entry(session, verdict, outcome));
            }
            TierOutcome::Hold => {
                summary.held += 1;
            }
            TierOutcome::Demote => {
                summary.demoted += 1;
                utterance
                    .dependent_tiers
                    .push(DependentTier::Com(ComTier::from_text(demotion_note(
                        verdict,
                    ))));
                queue.push(queue_entry(session, verdict, outcome));
            }
            TierOutcome::Untouched => {}
        }
    }

    if let Some(out_of_range) = by_ordinal.keys().find(|key| key.0 >= utterance_count) {
        return Err(MergeVerifyError::OrdinalOutOfRange {
            session: session.to_owned(),
            ordinal: *out_of_range,
            utterance_count,
        });
    }

    Ok(queue)
}

fn queue_entry(session: &str, verdict: &LineVerdict, tier: TierOutcome) -> QueueEntry {
    QueueEntry {
        session: session.to_owned(),
        utterance_index: verdict.utterance_index,
        category: verdict.category,
        tier,
        fa_mean_score: verdict.fa_mean_score,
        pitch: verdict.pitch,
        ear: verdict.ear,
    }
}

/// LOGICAL main-tier lines of a CHAT text, for the preservation check.
///
/// CHAT wraps long tiers onto tab-indented continuation lines and the
/// serializer may re-wrap differently (typically unwrapping), so the
/// invariant compares logical content: each `*` line with its
/// continuations joined by single spaces. Caught on real merged drafts
/// where wrapped utterances round-tripped to single lines.
fn main_tier_lines(chat: &str) -> Vec<String> {
    let mut logical: Vec<String> = Vec::new();
    let mut in_main = false;
    for line in chat.lines() {
        if line.starts_with('*') {
            logical.push(line.to_owned());
            in_main = true;
        } else if in_main && line.starts_with('\t') {
            if let Some(current) = logical.last_mut() {
                current.push(' ');
                current.push_str(line.trim_start_matches('\t'));
            }
        } else {
            in_main = false;
        }
    }
    logical
}

/// Run the pass: read every session named in the verdicts document from
/// `draft_dir`, apply tiers, write rewritten drafts and the review
/// queue into `out_dir`.
pub fn run(
    draft_dir: &Path,
    verdicts_path: &Path,
    out_dir: &Path,
    flag_prefix: &str,
) -> Result<VerifySummary, MergeVerifyError> {
    let verdicts_text =
        std::fs::read_to_string(verdicts_path).map_err(|source| MergeVerifyError::Io {
            path: verdicts_path.to_path_buf(),
            source,
        })?;
    let doc: VerdictsDoc =
        serde_json::from_str(&verdicts_text).map_err(|source| MergeVerifyError::VerdictsParse {
            path: verdicts_path.to_path_buf(),
            source,
        })?;

    std::fs::create_dir_all(out_dir).map_err(|source| MergeVerifyError::Io {
        path: out_dir.to_path_buf(),
        source,
    })?;

    let parser = crate::chat_parser();
    let mut summary = VerifySummary::default();
    let mut queue = ReviewQueue {
        entries: Vec::new(),
    };

    for session in &doc.sessions {
        let in_path = draft_dir.join(format!("{}.cha", session.session));
        if !in_path.is_file() {
            return Err(MergeVerifyError::MissingSession {
                session: session.session.clone(),
                path: in_path,
            });
        }
        let before = std::fs::read_to_string(&in_path).map_err(|source| MergeVerifyError::Io {
            path: in_path.clone(),
            source,
        })?;
        let (mut file, parse_errors) = batchalign_transform::parse::parse_lenient(&parser, &before);
        if !parse_errors.is_empty() {
            return Err(MergeVerifyError::DraftParse {
                path: in_path,
                details: parse_errors
                    .iter()
                    .map(|error| error.to_string())
                    .collect::<Vec<_>>()
                    .join("; "),
            });
        }

        queue.entries.extend(apply_to_file(
            &mut file,
            &session.session,
            &session.lines,
            flag_prefix,
            &mut summary,
        )?);
        summary.sessions += 1;

        let after = batchalign_transform::serialize::to_chat_string(&file);
        let mains_before = main_tier_lines(&before);
        let mains_after = main_tier_lines(&after);
        if let Some(changed) = mains_before
            .iter()
            .zip(mains_after.iter())
            .position(|(before_line, after_line)| before_line != after_line)
            .or_else(|| (mains_before.len() != mains_after.len()).then_some(usize::MAX))
        {
            return Err(MergeVerifyError::MainTierChanged {
                session: session.session.clone(),
                ordinal: UtteranceOrdinal(changed),
            });
        }

        let out_path = out_dir.join(format!("{}.cha", session.session));
        std::fs::write(&out_path, after).map_err(|source| MergeVerifyError::Io {
            path: out_path,
            source,
        })?;
    }

    let queue_path = out_dir.join("review-queue.json");
    let queue_json = serde_json::to_string_pretty(&queue).map_err(|source| {
        MergeVerifyError::QueueSerialize {
            path: queue_path.clone(),
            source,
        }
    })?;
    std::fs::write(&queue_path, queue_json).map_err(|source| MergeVerifyError::Io {
        path: queue_path,
        source,
    })?;

    Ok(summary)
}
