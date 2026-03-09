//! Media timing bullets (`\u{0015}start_end\u{0015}`).
//!
//! Bullets anchor utterance content to media offsets and optionally mark spans
//! that should be skipped in continuous playback flows.
//!
//! # CHAT Format References
//!
//! - [Bullets](https://talkbank.org/0info/manuals/CHAT.html#Bullets)

use super::WriteChat;
use crate::model::MediaTiming;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use talkbank_derive::{SemanticEq, SpanShift};

/// Media timing bullet attached to an utterance or header-level comment payload.
///
/// Start/end offsets are stored in milliseconds.
///
/// # CHAT Format
///
/// Bullets appear at the end of utterances using the `\u{0015}` (NEGATIVE ACKNOWLEDGE)
/// character as a delimiter:
///
/// ```text
/// Normal: \u{0015}start_end\u{0015}
/// Skip:   \u{0015}start_end-\u{0015}
/// ```
///
/// # Examples
///
/// ```text
/// *CHI: I want cookie . \u{0015}0_1073\u{0015}
/// *MOT: here you go . \u{0015}1073_2456\u{0015}
/// *CHI: [+ skip] xxx . \u{0015}2456_3120-\u{0015}
/// ```
///
/// # Skip Flag
///
/// The skip flag (indicated by a dash before the closing delimiter) marks segments
/// that should be skipped during continuous playback. This is typically used for
/// unintelligible speech, background noise, or other content that doesn't warrant
/// repeated listening.
///
/// # References
///
/// - [Bullets](https://talkbank.org/0info/manuals/CHAT.html#Bullets)
/// - [Media Linking](https://talkbank.org/0info/manuals/CHAT.html#MediaLinking)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, SemanticEq, SpanShift)]
pub struct Bullet {
    /// Start/end media offsets in milliseconds.
    #[serde(flatten)]
    pub timing: MediaTiming,

    /// Whether playback should skip this range during autoplay/continuous modes.
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub skip: bool,

    /// Source location metadata for diagnostics (not serialized).
    #[serde(skip)]
    #[schemars(skip)]
    pub span: crate::Span,
}

impl Bullet {
    /// Build a non-skip bullet with dummy span metadata.
    ///
    /// Use this constructor for regular timing bullets; call `with_skip(true)`
    /// when representing skip ranges.
    pub fn new(start_ms: u64, end_ms: u64) -> Self {
        Self {
            timing: MediaTiming::new(start_ms, end_ms),
            skip: false,
            span: crate::Span::DUMMY,
        }
    }

    /// Attach source span metadata used by diagnostics.
    ///
    /// Span metadata is not serialized and does not affect timing semantics.
    pub fn with_span(mut self, span: crate::Span) -> Self {
        self.span = span;
        self
    }

    /// Sets skip behavior flag for continuous playback.
    pub fn with_skip(mut self, skip: bool) -> Self {
        self.skip = skip;
        self
    }
}

impl WriteChat for Bullet {
    /// Serializes canonical CHAT bullet syntax with optional trailing `-`.
    fn write_chat<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_char('\u{0015}')?; // Bullet start
        write!(w, "{}_{}", self.timing.start_ms, self.timing.end_ms)?;
        if self.skip {
            w.write_char('-')?; // Skip marker before closing delimiter
        }
        w.write_char('\u{0015}') // Bullet end
    }
}

impl std::fmt::Display for Bullet {
    /// Formats this media bullet in CHAT delimiter form.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_chat(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trips a standard bullet without skip marker.
    #[test]
    fn bullet_normal_roundtrip() {
        let bullet = Bullet::new(0, 1073);
        let output = bullet.to_string();
        assert_eq!(
            output, "\u{0015}0_1073\u{0015}",
            "Normal bullet roundtrip failed"
        );
    }

    /// Round-trips a bullet that carries the trailing skip marker.
    #[test]
    fn bullet_with_skip_roundtrip() {
        let bullet = Bullet::new(1000, 2000).with_skip(true);
        let output = bullet.to_string();
        assert_eq!(
            output, "\u{0015}1000_2000-\u{0015}",
            "Skip bullet roundtrip failed"
        );
    }

    /// Round-trips a zero-duration bullet.
    #[test]
    fn bullet_zero_time_roundtrip() {
        let bullet = Bullet::new(0, 0);
        let output = bullet.to_string();
        assert_eq!(
            output, "\u{0015}0_0\u{0015}",
            "Zero-time bullet roundtrip failed"
        );
    }
}
