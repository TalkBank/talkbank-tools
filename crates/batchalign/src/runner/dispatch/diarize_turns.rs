//! Typed model for the `diarize` command's speaker-turns JSON artifact.
//!
//! Contract: the emitted document is exactly the input schema consumed by
//! `chatter rediarize --turns` (talkbank-transform's `TurnsFile`):
//!
//! ```json
//! {"source": "batchalign3:pyannote",
//!  "turns": [{"track": "PAR0", "start_ms": 0, "end_ms": 1200}]}
//! ```
//!
//! chatter parses the file strictly (unknown or misspelled fields fail the
//! parse rather than being dropped), so the field names here are
//! load-bearing wire format, not internal naming.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::types::worker_v2::SpeakerSegmentV2;

/// Provenance recorded in the artifact's `source` field: which tool and
/// engine produced these turns.
const PYANNOTE_TURNS_SOURCE: &str = "batchalign3:pyannote";

/// Anonymous CHAT-style speaker track (`PAR0`..`PARn`).
///
/// A track is an acoustic identity assigned by the diarizer, NOT a CHAT
/// role; role assignment is downstream work (`chatter rediarize` /
/// speaker-id). The inner index is the track number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct AnonymousTrack(pub(crate) u32);

impl Serialize for AnonymousTrack {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&format_args!("PAR{}", self.0))
    }
}

/// One diarized speaker turn: an anonymous track plus its media span.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub(crate) struct DiarizedTurn {
    /// Millisecond media offsets first so the derived `Ord` sorts turns
    /// chronologically (start, end, track), the order chatter expects.
    pub(crate) start_ms: u64,
    /// Exclusive end of the turn in media milliseconds.
    pub(crate) end_ms: u64,
    /// Anonymous track code for the voice heard in this span.
    pub(crate) track: AnonymousTrack,
}

/// The complete turns artifact for one media file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct DiarizedTurnsFile {
    /// Engine provenance (`batchalign3:pyannote`).
    pub(crate) source: &'static str,
    /// All detected turns, chronologically ordered.
    pub(crate) turns: Vec<DiarizedTurn>,
}

/// Errors while converting worker speaker segments to the turns artifact.
#[derive(Debug, thiserror::Error)]
pub(crate) enum TurnsBuildError {
    /// A worker segment ended before it started; defective engine output
    /// must fail the file, not be silently reordered or dropped.
    #[error("diarizer returned an inverted segment: start_ms {start_ms} > end_ms {end_ms}")]
    InvertedSegment {
        /// Reported segment start (ms).
        start_ms: u64,
        /// Reported segment end (ms).
        end_ms: u64,
    },

    /// The typed model failed to serialize (indicates a programming error,
    /// surfaced rather than panicking per the no-panic policy).
    #[error("failed to serialize turns JSON: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// Convert raw worker segments into the canonical turns artifact text.
///
/// Diarizer-native labels (pyannote's `SPEAKER_00`, ...) map to anonymous
/// track codes deterministically: distinct labels sorted lexically become
/// `PAR0..PARn`. Turns are emitted in chronological order.
pub(crate) fn format_turns_json(segments: &[SpeakerSegmentV2]) -> Result<String, TurnsBuildError> {
    // Deterministic label -> track assignment via the sorted-set order.
    let track_by_label: BTreeMap<&str, AnonymousTrack> = segments
        .iter()
        .map(|segment| segment.speaker.as_str())
        .collect::<std::collections::BTreeSet<&str>>()
        .into_iter()
        .enumerate()
        .map(|(index, label)| (label, AnonymousTrack(index as u32)))
        .collect();

    let mut turns = Vec::with_capacity(segments.len());
    for segment in segments {
        let (start_ms, end_ms) = (segment.start_ms.0, segment.end_ms.0);
        if start_ms > end_ms {
            return Err(TurnsBuildError::InvertedSegment { start_ms, end_ms });
        }
        // Map-lookup invariant: every segment label was inserted into
        // `track_by_label` by the collection pass above.
        #[allow(clippy::expect_used)]
        let track = *track_by_label
            .get(segment.speaker.as_str())
            .expect("segment label must be present in the label->track map");
        turns.push(DiarizedTurn {
            start_ms,
            end_ms,
            track,
        });
    }
    turns.sort();

    let file = DiarizedTurnsFile {
        source: PYANNOTE_TURNS_SOURCE,
        turns,
    };
    let mut text = serde_json::to_string_pretty(&file)?;
    text.push('\n');
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::DurationMs;

    fn segment(speaker: &str, start_ms: u64, end_ms: u64) -> SpeakerSegmentV2 {
        SpeakerSegmentV2 {
            start_ms: DurationMs(start_ms),
            end_ms: DurationMs(end_ms),
            speaker: speaker.to_owned(),
        }
    }

    #[test]
    fn maps_sorted_labels_to_par_tracks_and_sorts_turns() {
        let segments = [
            segment("SPEAKER_01", 5000, 6000),
            segment("SPEAKER_00", 0, 1200),
            segment("SPEAKER_01", 1200, 2000),
        ];
        let json = format_turns_json(&segments).expect("valid segments");
        let value: serde_json::Value = serde_json::from_str(&json).expect("well-formed JSON");
        assert_eq!(value["source"], "batchalign3:pyannote");
        let turns = value["turns"].as_array().expect("turns array");
        // Chronological order, SPEAKER_00 -> PAR0, SPEAKER_01 -> PAR1.
        assert_eq!(turns[0]["track"], "PAR0");
        assert_eq!(turns[0]["start_ms"], 0);
        assert_eq!(turns[1]["track"], "PAR1");
        assert_eq!(turns[1]["start_ms"], 1200);
        assert_eq!(turns[2]["end_ms"], 6000);
    }

    #[test]
    fn empty_segments_produce_an_empty_turns_document() {
        let json = format_turns_json(&[]).expect("empty input is valid");
        let value: serde_json::Value = serde_json::from_str(&json).expect("well-formed JSON");
        assert_eq!(value["turns"].as_array().map(Vec::len), Some(0));
    }

    #[test]
    fn inverted_segment_is_a_typed_error() {
        let result = format_turns_json(&[segment("SPEAKER_00", 2000, 1000)]);
        assert!(matches!(
            result,
            Err(TurnsBuildError::InvertedSegment { .. })
        ));
    }

    #[test]
    fn wire_shape_matches_the_chatter_rediarize_contract_exactly() {
        // chatter's TurnsFile parser is strict; pin the exact field set.
        let json = format_turns_json(&[segment("SPEAKER_00", 10, 20)]).expect("valid");
        let value: serde_json::Value = serde_json::from_str(&json).expect("well-formed JSON");
        let object = value.as_object().expect("top-level object");
        assert_eq!(
            object.keys().collect::<Vec<_>>(),
            ["source", "turns"],
            "top-level field set drifted from the chatter contract"
        );
        let turn = value["turns"][0].as_object().expect("turn object");
        let mut keys = turn.keys().collect::<Vec<_>>();
        keys.sort();
        assert_eq!(
            keys,
            ["end_ms", "start_ms", "track"],
            "turn field set drifted from the chatter contract"
        );
    }
}
