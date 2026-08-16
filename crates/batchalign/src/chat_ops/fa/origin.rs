//! Where a timing number came from, carried with the number.
//!
//! # Why provenance is a type and not a comment
//!
//! A CHAT bullet is a pair of integers. Nothing about `1266565_1286865`
//! distinguishes a moment an engine measured against audio from one this
//! program computed by dividing a gap by a word count, from one a repair pass
//! moved to restore ordering, from one that was cut down to fit the recording.
//! All four are written identically and all four read downstream as
//! measurements.
//!
//! That is not a hypothetical loss. On one merged corpus roughly 37 percent of
//! the timings were interpolated rather than measured, and the two were
//! indistinguishable in the output, so a comparison against a reference
//! transcript reported 37.2 percent agreement when aligning by time and 76.4
//! percent aligning by text: the gap was almost entirely our own invented
//! timings being scored as though they were observations. There was no way to
//! ask the data which numbers were real, because the data had never been asked
//! to remember.
//!
//! # The rule this module enforces
//!
//! Every timing that reaches a transcript is accompanied by an [`Origin`]
//! saying how it was produced, and the constructors that produce timings
//! require one. A caller cannot mint a number without stating where it came
//! from, because there is no constructor that omits the argument.
//!
//! # The distinction that matters most
//!
//! [`Origin::is_observation`] separates numbers that came from OUTSIDE this
//! program (an engine measuring audio, or a human writing a bullet) from
//! numbers this program DERIVED. Only the first kind may be treated as
//! evidence. Everything else is our own arithmetic, and scoring our arithmetic
//! against a reference measures the arithmetic, not the transcript.

use std::borrow::Cow;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::coordinates::{FileMs, Ms};

/// Which alignment engine produced a measurement.
///
/// A newtype rather than a bare string so an engine name cannot be swapped with
/// a language code, a model path, or any of the other short strings that travel
/// beside it through the worker protocol.
///
/// # Why `Cow<'static, str>`
///
/// Every engine name PRODUCED in this workspace is a compile-time literal:
/// `EngineBackend::wire_name` returns `&'static str` and every implementation
/// returns one. So construction borrows and never allocates, which matters
/// because an `Origin` is built per word (twice per word on the interval path,
/// once per token on the onset path) and a `String` here cost a heap
/// allocation at every one of them.
///
/// It is a `Cow` rather than a plain `&'static str` because provenance is
/// SERIALIZED: a timing read back from the FA cache carries an owned name that
/// no literal in this binary corresponds to. Cloning stays free on the live
/// path (a borrowed `Cow` clone copies a pointer), and only cache reads own
/// their string. A type that is expensive to carry is a type people stop
/// carrying, and provenance nobody carries is the defect this module exists to
/// prevent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EngineId(Cow<'static, str>);

impl EngineId {
    /// Name an engine, borrowing a literal.
    pub const fn new(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }
}

impl fmt::Display for EngineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a clamped value was cut down to fit.
///
/// The three bounds this pipeline clamps against are different claims about
/// the world, and collapsing them loses exactly the fact a reader needs. A
/// word cut to the RECORDING means the transcript described more speech than
/// the audio holds, which is a data problem. A word cut to its own UTTERANCE
/// means our word-level arithmetic disagreed with an utterance bullet we were
/// given, which is our problem. A word cut to the NEXT ONSET means an assumed
/// duration ran into the following word, which is neither: it is the assumption
/// working as intended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClampBound {
    /// The end of the audio file.
    RecordingEnd,
    /// The enclosing utterance's bullet.
    UtteranceBullet,
    /// The onset of the following word.
    NextOnset,
}

impl fmt::Display for ClampBound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RecordingEnd => f.write_str("the end of the recording"),
            Self::UtteranceBullet => f.write_str("the utterance bullet"),
            Self::NextOnset => f.write_str("the next word's onset"),
        }
    }
}

/// How a timing number was produced.
///
/// Ordered roughly from strongest evidence to weakest. Each derived variant
/// carries the inputs of its own computation, so a reader can reconstruct WHY
/// the number is what it is rather than only that it was derived.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    /// An alignment engine measured this position against the audio.
    ///
    /// The strongest thing we have. Still not ground truth: an engine that
    /// fails to find its words reports positions that are arithmetic rather
    /// than measurement, which is why a measured timing can still be refused
    /// by the containment check.
    EngineMeasured {
        /// The engine that reported it.
        engine: EngineId,
    },

    /// Read from a bullet that was already in the transcript when we received
    /// it.
    ///
    /// An observation in the sense that it did not come from this program, but
    /// its own provenance is whatever produced that transcript, which may
    /// itself have been a machine. Treated as evidence because refusing to is
    /// the same as refusing to accept human transcription.
    TranscriptBullet,

    /// Cut down to fit a bound because it overshot.
    ///
    /// Wraps the origin of the value that was cut, so clamping a measurement
    /// and clamping an estimate stay distinguishable.
    ///
    /// This was `ClampedToRecording` until it was noticed that three call sites
    /// used it for three DIFFERENT bounds: the recording's end, the enclosing
    /// utterance's bullet, and the next word's onset. The variant named one of
    /// them, so two thirds of the provenance it recorded was wrong, and one of
    /// the offending call sites had a docstring saying in prose that its bound
    /// was "not the recording, and the two are different claims". Prose is
    /// gated by nothing; [`ClampBound`] puts the distinction in the value.
    ClampedTo {
        /// Which bound the value was cut down to.
        bound: ClampBound,
        /// Where the value came from before it was cut.
        was: Box<Origin>,
        /// The value before clamping.
        original: FileMs,
        /// How far past the bound it fell.
        overshoot: Ms,
    },

    /// Computed by distributing a gap between two anchors proportionally to
    /// word count.
    ///
    /// The placement used for utterances that carry no bullet. It is arithmetic
    /// over a word count and says nothing about when anyone spoke; the fields
    /// record the computation so a reader can see how thin the evidence is. A
    /// run of 25 utterances sharing 290 milliseconds of audio is visibly not a
    /// measurement, and without this variant that fact is invisible in output.
    EstimatedFromWordCount {
        /// The audio available to the whole run of untimed utterances.
        gap: Ms,
        /// Words placed before this one within the run.
        words_before: usize,
        /// Words in the entire run.
        words_total: usize,
    },

    /// Moved so that timings run in order.
    ///
    /// A repair pass adjusting a neighbour is a decision by this program, not
    /// an observation, however well justified.
    RepairedForOrder {
        /// Where the value came from before it was moved.
        was: Box<Origin>,
        /// The value before repair.
        original: FileMs,
    },

    /// Taken from a neighbouring boundary because this word had none of its own.
    ///
    /// The utterance bullet standing in for a word end the engine never
    /// reported. Better evidence than an invented constant, since a human or an
    /// earlier pass placed that bullet, and still not a measurement of THIS
    /// word.
    InheritedFromNeighbour {
        /// The boundary that was copied.
        from: FileMs,
    },

    /// One span covering several separately measured parts.
    ///
    /// A compound filler (`&-you_know`) is sent to the engine as N words and
    /// comes back as N timings, which are merged into the single span the CHAT
    /// word occupies. The same shape arises when an utterance's extent is taken
    /// as the min and max of its words' spans. The parts were measured; the
    /// envelope is our arithmetic over them, and it necessarily covers any
    /// silence between.
    MergedFromParts {
        /// How many measured spans the envelope covers.
        parts: usize,
    },

    /// The end of a word an ONSET-ONLY engine never reported, taken from the
    /// next token's onset.
    ///
    /// Whisper reports when a word starts and never when it stops, so every
    /// word end on that path is inferred from its successor. That is a
    /// reasonable inference and it is still not a measurement: the word may
    /// have ended long before the next one began.
    DerivedFromNextOnset,

    /// The end of the LAST word in a group, where no successor onset exists.
    ///
    /// A fixed duration stood in for a quantity nothing measured. Previously
    /// written as `unwrap_or(start + LAST_WORD_FALLBACK_MS)`, which made an
    /// invented 500 milliseconds indistinguishable from an observed one. A
    /// fallback that cannot be seen is a fabricated measurement.
    FallbackDuration {
        /// The duration that was assumed.
        assumed: Ms,
    },
}

/// What kind of number an [`Origin`] describes.
///
/// # Why this exists
///
/// `is_observation` and `ProvenanceTally::record` were two exhaustive matches
/// over the same variants, in the same file, kept consistent by nothing but
/// care: the tally's `observed` bucket had to be exactly the set
/// `is_observation` returns true for, and a third reading lived in
/// `WordSpan::is_fully_observed`. A test existed whose only job was to notice
/// the first two drifting apart, which is a standing confession that one of
/// them should not exist.
///
/// Now the classification has ONE owner and both callers ask it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginKind {
    /// An engine measured it, or the transcript already carried it.
    Observed,
    /// Inferred from a neighbouring measurement.
    Derived,
    /// Invented outright.
    Assumed,
    /// Adjusted after the fact.
    Adjusted,
}

impl Origin {
    /// What kind of number this is.
    ///
    /// Exhaustive on purpose: a new variant must be given a kind here, and both
    /// consumers then classify it correctly without being touched.
    pub fn kind(&self) -> OriginKind {
        match self {
            Self::EngineMeasured { .. } | Self::TranscriptBullet => OriginKind::Observed,
            Self::DerivedFromNextOnset
            | Self::InheritedFromNeighbour { .. }
            | Self::MergedFromParts { .. } => OriginKind::Derived,
            Self::FallbackDuration { .. } | Self::EstimatedFromWordCount { .. } => {
                OriginKind::Assumed
            }
            Self::ClampedTo { .. } | Self::RepairedForOrder { .. } => OriginKind::Adjusted,
        }
    }

    /// Whether this number came from outside this program.
    ///
    /// The one question every downstream consumer actually wants to ask, and
    /// the one that could not be asked at all before provenance was carried.
    /// Only observations may be scored as evidence; derived numbers are our own
    /// arithmetic and scoring them measures us.
    pub fn is_observation(&self) -> bool {
        matches!(self.kind(), OriginKind::Observed)
    }

    /// The origin this one was derived FROM, when it wraps another.
    ///
    /// The single owner of "which variants nest". Both questions below walk the
    /// same chain, and each used to carry its own exhaustive nine-arm match, so
    /// adding a wrapping variant meant remembering to update two places that
    /// nothing kept in step. Now a new variant has exactly one match to answer.
    fn was(&self) -> Option<&Origin> {
        match self {
            Self::ClampedTo { was, .. } | Self::RepairedForOrder { was, .. } => Some(was),
            // Written out rather than left to a catch-all so a new variant that
            // wraps another cannot silently report that it wraps nothing.
            Self::EngineMeasured { .. }
            | Self::TranscriptBullet
            | Self::EstimatedFromWordCount { .. }
            | Self::InheritedFromNeighbour { .. }
            | Self::MergedFromParts { .. }
            | Self::DerivedFromNextOnset
            | Self::FallbackDuration { .. } => None,
        }
    }

    /// This origin and every origin it was derived from, outermost first.
    fn chain(&self) -> impl Iterator<Item = &Origin> {
        std::iter::successors(Some(self), |origin| origin.was())
    }

    /// The original observation underneath any adjustments, when there was one.
    ///
    /// Lets a caller ask "was there ever a measurement here?" separately from
    /// "is this number still one?", which are different questions: the first
    /// decides whether re-running alignment could help, the second decides
    /// whether the number may be used as evidence.
    pub fn underlying(&self) -> &Origin {
        // `fold` rather than `last`, because the chain always yields at least
        // `self`, and this way that fact needs no unwrapping to express.
        self.chain().fold(self, |_, origin| origin)
    }

    /// Whether this number was cut down to the end of the recording, at any
    /// depth in its history.
    ///
    /// The one clamp that is a fact about the DATA rather than about our
    /// arithmetic: it means the transcript described speech continuing past the
    /// end of the audio, so either the bullet is wrong or the media is the wrong
    /// file. The other two bounds are routine. Before [`ClampBound`] existed all
    /// three were the same variant, so this question could not be asked and the
    /// case was pooled with the ordinary ones in every count.
    ///
    /// Asks the whole chain, because a value clamped to the recording and then
    /// repaired for order must still report it.
    pub fn overran_recording(&self) -> bool {
        self.chain().any(|origin| {
            matches!(
                origin,
                Self::ClampedTo {
                    bound: ClampBound::RecordingEnd,
                    ..
                }
            )
        })
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EngineMeasured { engine } => write!(f, "measured by {engine}"),
            Self::TranscriptBullet => f.write_str("from the source transcript"),
            Self::ClampedTo {
                bound,
                was,
                original,
                overshoot,
            } => write!(
                f,
                "clamped to {bound} from {original} ({overshoot} over), was {was}"
            ),
            Self::EstimatedFromWordCount {
                gap,
                words_before,
                words_total,
            } => write!(
                f,
                "estimated: word {words_before} of {words_total} across {gap}"
            ),
            Self::RepairedForOrder { was, original } => {
                write!(f, "reordered from {original}, was {was}")
            }
            Self::MergedFromParts { parts } => write!(f, "merged from {parts} measured parts"),
            Self::InheritedFromNeighbour { from } => write!(f, "inherited from {from}"),
            Self::DerivedFromNextOnset => f.write_str("derived from the next word's onset"),
            Self::FallbackDuration { assumed } => write!(f, "assumed duration of {assumed}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured() -> Origin {
        Origin::EngineMeasured {
            engine: EngineId::new("whisper-fa-large-v2"),
        }
    }

    #[test]
    fn engine_measurements_and_source_bullets_are_observations() {
        assert!(measured().is_observation());
        assert!(Origin::TranscriptBullet.is_observation());
    }

    #[test]
    fn our_own_arithmetic_is_never_an_observation() {
        // The 25-utterances-in-290ms case, which is the shape that made a
        // corpus comparison report 37.2 percent agreement.
        let estimated = Origin::EstimatedFromWordCount {
            gap: Ms(290),
            words_before: 40,
            words_total: 180,
        };
        assert!(!estimated.is_observation());
        assert!(
            !Origin::InheritedFromNeighbour {
                from: FileMs::new(1_000)
            }
            .is_observation()
        );
    }

    #[test]
    fn adjusting_a_measurement_stops_it_being_one_but_remembers_it_was() {
        let clamped = Origin::ClampedTo {
            bound: ClampBound::RecordingEnd,
            was: Box::new(measured()),
            original: FileMs::new(1_288_185),
            overshoot: Ms(28_217),
        };
        assert!(!clamped.is_observation());
        assert_eq!(clamped.underlying(), &measured());
    }

    #[test]
    fn adjustments_compose_without_losing_the_bottom() {
        // Nesting is why `underlying` recurses: a value clamped twice must
        // still name the observation at the bottom, not the previous clamp.
        let stacked = Origin::ClampedTo {
            bound: ClampBound::RecordingEnd,
            was: Box::new(Origin::ClampedTo {
                bound: ClampBound::RecordingEnd,
                was: Box::new(Origin::TranscriptBullet),
                original: FileMs::new(500),
                overshoot: Ms(100),
            }),
            original: FileMs::new(480),
            overshoot: Ms(80),
        };
        assert_eq!(stacked.underlying(), &Origin::TranscriptBullet);
        assert!(!stacked.is_observation());
    }
}

#[cfg(test)]
mod laundering_tests {
    use super::*;

    /// An invented timing must not come back claiming to be an observation.
    ///
    /// This is the regression the 2026-08-15 review found: forced alignment
    /// wrote `WordTiming { span, origin }` into a `Bullet` (two integers), and
    /// post-processing read those bullets back and stamped every one
    /// `TranscriptBullet`, whose `is_observation()` is TRUE. So a 500 ms
    /// fallback this program invented came back labelled as observed, to
    /// exactly the consumer provenance exists for.
    ///
    /// The property is stated here rather than at the seam because it is about
    /// the MEANING of the variants, and it fails loudly if anyone ever makes an
    /// invented origin report as observed.
    #[test]
    fn no_invented_origin_reports_itself_as_an_observation() {
        let invented = [
            Origin::FallbackDuration { assumed: Ms(500) },
            Origin::DerivedFromNextOnset,
            Origin::MergedFromParts { parts: 2 },
            Origin::EstimatedFromWordCount {
                gap: Ms(4_000),
                words_before: 0,
                words_total: 175,
            },
            Origin::InheritedFromNeighbour {
                from: FileMs::new(1_000),
            },
            Origin::ClampedTo {
                bound: ClampBound::RecordingEnd,
                was: Box::new(Origin::EngineMeasured {
                    engine: EngineId::new("whisper-fa-large-v2"),
                }),
                original: FileMs::new(1_288_185),
                overshoot: Ms(28_217),
            },
            Origin::RepairedForOrder {
                was: Box::new(Origin::TranscriptBullet),
                original: FileMs::new(400),
            },
        ];
        for origin in invented {
            assert!(
                !origin.is_observation(),
                "{origin} reported itself as an observation"
            );
        }
    }
}

#[cfg(test)]
mod cache_boundary_tests {
    use super::*;
    use crate::chat_ops::fa::WordTiming;

    /// A stored zero-width timing must not deserialize back into existence.
    ///
    /// This guard was lost, not designed away: adding `origin` to `WordTiming`
    /// replaced `#[serde(try_from = "TimeSpan")]` with a plain derive, and the
    /// docstring next to it went on claiming the check still happened. It is
    /// what makes the FA cache self-cleaning, so losing it silently would have
    /// let a cached `T_T` bullet survive every future run.
    #[test]
    fn a_stored_zero_width_timing_fails_to_load() {
        let stored = r#"{"span":{"start_ms":5,"end_ms":5},"start_origin":"TranscriptBullet","end_origin":"TranscriptBullet"}"#;
        assert!(
            serde_json::from_str::<WordTiming>(stored).is_err(),
            "a zero-width span must not survive a cache round trip"
        );
    }

    /// An entry written before timings carried provenance is retired the same
    /// way, which is why the field needed no migration.
    #[test]
    fn a_pre_provenance_entry_fails_to_load() {
        let stored = r#"{"start_ms":100,"end_ms":600}"#;
        assert!(serde_json::from_str::<WordTiming>(stored).is_err());
    }

    /// A well-formed entry round-trips with its provenance intact, which is the
    /// whole point of storing it.
    #[test]
    fn a_real_timing_round_trips_with_its_origin() {
        // A measured start with an inferred end: the ordinary onset-engine
        // word, and the case a single origin could not express.
        let timing = WordTiming::new(
            100,
            600,
            Origin::EngineMeasured {
                engine: EngineId::new("whisper-fa-large-v2"),
            },
            Origin::DerivedFromNextOnset,
        )
        .expect("has extent");
        let json = serde_json::to_string(&timing).expect("serializes");
        let back: WordTiming = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, timing);
        assert_eq!(back.end_origin(), &Origin::DerivedFromNextOnset);
        assert!(back.start_origin().is_observation());
        assert!(
            !back.end_origin().is_observation(),
            "an inferred end is not an observation"
        );
    }
}

/// How a set of timings was produced, counted by kind.
///
/// # Why this exists
///
/// Provenance that reaches the cache but not the transcript answers nobody's
/// question. A `Bullet` is two integers and cannot carry an `Origin`, so the
/// per-word chain necessarily stops at the write; what CAN cross is a summary,
/// and a summary is what a reader of a corpus actually needs: "of this
/// utterance's sixteen word timings, twelve were measured, three inferred from
/// a neighbour, one invented outright."
///
/// Before this, a consumer scoring our timings against a reference had no way
/// to exclude the ones we made up, which is how a comparison came to report
/// 37.2% agreement by time against 76.4% by text.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ProvenanceTally {
    /// An engine measured it, or the transcript already carried it.
    pub observed: usize,
    /// Inferred from a neighbouring measurement: the next word's onset, the
    /// utterance's own bullet, or several parts merged into one span.
    pub derived: usize,
    /// Invented outright: a fallback duration, or a word-count distribution.
    pub assumed: usize,
    /// Adjusted after the fact: clamped to a bound, or moved to restore order.
    pub adjusted: usize,
    /// Of those, how many were cut down to the END OF THE RECORDING.
    ///
    /// A subset of `adjusted`, not a fifth bucket, so the four still sum to
    /// `total`. Counted separately because it is the only adjustment that says
    /// something is wrong with the INPUT rather than with our arithmetic: the
    /// transcript claimed speech past the end of the audio.
    pub overran_recording: usize,
}

impl ProvenanceTally {
    /// Count one timing, by the way it most recently came to be.
    ///
    /// Classified on the OUTERMOST origin, because that is what the number now
    /// is: a measurement that was later clamped is no longer a measurement.
    /// Exhaustive on purpose, so a new `Origin` variant must be given a bucket
    /// rather than silently joining one.
    pub fn record(&mut self, origin: &Origin) {
        match origin.kind() {
            OriginKind::Observed => self.observed += 1,
            OriginKind::Derived => self.derived += 1,
            OriginKind::Assumed => self.assumed += 1,
            OriginKind::Adjusted => self.adjusted += 1,
        }
        // Asked of every origin, not only the adjusted ones: a recording clamp
        // can sit underneath a later repair, and the outermost kind would then
        // report `Adjusted` for a reason that is not this one.
        if origin.overran_recording() {
            self.overran_recording += 1;
        }
    }

    /// How many timings were counted.
    pub fn total(self) -> usize {
        self.observed + self.derived + self.assumed + self.adjusted
    }

    /// Whether anything here is not a straightforward observation.
    pub fn any_not_observed(self) -> bool {
        self.total() > self.observed
    }

    /// Whether a human should look at this utterance's timings.
    ///
    /// Two cases earn attention, and they are different complaints. An INVENTED
    /// timing is anchored to nothing, where a derived one is anchored to a real
    /// measurement next door. A timing cut back to the recording's end says the
    /// transcript described speech the audio does not contain, which is a
    /// problem with the delivery rather than with the alignment.
    ///
    /// The second case used to be unaskable: every clamp was one variant, so a
    /// word cut to the end of the file counted the same as one capped at the
    /// next word's onset, and only the routine case was common enough to notice.
    pub fn needs_review(self) -> bool {
        self.assumed > 0 || self.overran_recording > 0
    }
}

impl fmt::Display for ProvenanceTally {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "measured={} derived={} assumed={} adjusted={}",
            self.observed, self.derived, self.assumed, self.adjusted
        )?;
        // Only when it happened, and stated separately from `adjusted` because
        // it is the one adjustment that says something is wrong with the INPUT.
        // Omitting it made an utterance flagged solely for a recording overrun
        // read byte-identically to a routine next-onset clamp that is NOT
        // flagged, so the reviewer was told to look and not told why.
        match self.overran_recording {
            0 => Ok(()),
            n => write!(f, " past-recording-end={n}"),
        }
    }
}

#[cfg(test)]
mod tally_tests {
    use super::*;

    #[test]
    fn each_kind_lands_in_its_own_bucket() {
        let mut tally = ProvenanceTally::default();
        tally.record(&Origin::EngineMeasured {
            engine: EngineId::new("wav2vec_fa"),
        });
        tally.record(&Origin::TranscriptBullet);
        tally.record(&Origin::DerivedFromNextOnset);
        tally.record(&Origin::FallbackDuration { assumed: Ms(500) });
        tally.record(&Origin::ClampedTo {
            bound: ClampBound::RecordingEnd,
            was: Box::new(Origin::TranscriptBullet),
            original: FileMs::new(10),
            overshoot: Ms(5),
        });
        assert_eq!(tally.observed, 2);
        assert_eq!(tally.derived, 1);
        assert_eq!(tally.assumed, 1);
        assert_eq!(tally.adjusted, 1);
        assert_eq!(tally.total(), 5);
    }

    #[test]
    fn only_an_invented_timing_asks_for_review() {
        // A derived end is anchored to a real neighbouring measurement; an
        // assumed one is anchored to nothing, and that is the difference a
        // reviewer's time should be spent on.
        let mut derived = ProvenanceTally::default();
        derived.record(&Origin::DerivedFromNextOnset);
        assert!(derived.any_not_observed());
        assert!(!derived.needs_review());

        let mut assumed = ProvenanceTally::default();
        assumed.record(&Origin::FallbackDuration { assumed: Ms(500) });
        assert!(assumed.needs_review());
    }

    #[test]
    fn all_measured_needs_nothing() {
        let mut tally = ProvenanceTally::default();
        tally.record(&Origin::TranscriptBullet);
        assert!(!tally.any_not_observed());
        assert!(!tally.needs_review());
    }

    #[test]
    fn only_the_recording_clamp_calls_for_a_reviewer() {
        // The three bounds were one variant until 2026-08-15, so this
        // distinction could not be drawn at all. A word capped at the next
        // word's onset is routine; a word cut back to the end of the file means
        // the transcript described speech the audio does not contain.
        let routine = Origin::ClampedTo {
            bound: ClampBound::NextOnset,
            was: Box::new(Origin::FallbackDuration { assumed: Ms(500) }),
            original: FileMs::new(1_500),
            overshoot: Ms(200),
        };
        let overran = Origin::ClampedTo {
            bound: ClampBound::RecordingEnd,
            was: Box::new(Origin::TranscriptBullet),
            original: FileMs::new(1_288_185),
            overshoot: Ms(28_217),
        };

        assert!(!routine.overran_recording());
        assert!(overran.overran_recording());

        // Both are `Adjusted`, which is exactly why the kind alone cannot
        // answer this.
        assert_eq!(routine.kind(), OriginKind::Adjusted);
        assert_eq!(overran.kind(), OriginKind::Adjusted);

        let mut tally = ProvenanceTally::default();
        tally.record(&routine);
        assert_eq!(tally.adjusted, 1);
        assert_eq!(tally.overran_recording, 0);
        // `assumed` is 0 here: the fallback underneath was wrapped by the
        // clamp, so the outermost kind is what counts.
        assert!(!tally.needs_review());

        tally.record(&overran);
        assert_eq!(tally.adjusted, 2);
        assert_eq!(tally.overran_recording, 1);
        assert!(tally.needs_review());
        // The subset does not inflate the total.
        assert_eq!(tally.total(), 2);
    }

    #[test]
    fn a_recording_clamp_survives_a_later_repair() {
        // Why `overran_recording` recurses: an ordering repair on top of a
        // recording clamp leaves the outermost variant reporting the repair,
        // and the overrun would go uncounted if only the top were inspected.
        let repaired = Origin::RepairedForOrder {
            was: Box::new(Origin::ClampedTo {
                bound: ClampBound::RecordingEnd,
                was: Box::new(Origin::EngineMeasured {
                    engine: EngineId::new("whisper-fa-large-v2"),
                }),
                original: FileMs::new(1_288_185),
                overshoot: Ms(28_217),
            }),
            original: FileMs::new(1_259_968),
        };

        assert!(repaired.overran_recording());
        let mut tally = ProvenanceTally::default();
        tally.record(&repaired);
        assert_eq!(tally.overran_recording, 1);
        assert!(tally.needs_review());
    }
}
