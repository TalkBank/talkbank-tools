//! Media timing bullets (`\u{0015}start_end\u{0015}`).
//!
//! Bullets anchor utterance content to media offsets.
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
/// \u{0015}start_end\u{0015}
/// ```
///
/// # Examples
///
/// ```text
/// *CHI: I want cookie . \u{0015}0_1073\u{0015}
/// *MOT: here you go . \u{0015}1073_2456\u{0015}
/// ```
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

    /// Source location metadata for diagnostics (not serialized).
    #[serde(skip)]
    #[schemars(skip)]
    pub span: crate::Span,
}

impl Bullet {
    /// Build a bullet with dummy span metadata.
    pub fn new(start_ms: u64, end_ms: u64) -> Self {
        Self {
            timing: MediaTiming::new(start_ms, end_ms),
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
}

impl WriteChat for Bullet {
    /// Serializes canonical CHAT bullet syntax.
    fn write_chat<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_char('\u{0015}')?;
        write!(w, "{}_{}", self.timing.start_ms, self.timing.end_ms)?;
        w.write_char('\u{0015}')
    }
}

impl std::fmt::Display for Bullet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_chat(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bullet_roundtrip() {
        let bullet = Bullet::new(0, 1073);
        let output = bullet.to_string();
        assert_eq!(output, "\u{0015}0_1073\u{0015}");
    }

    #[test]
    fn bullet_zero_time_roundtrip() {
        let bullet = Bullet::new(0, 0);
        let output = bullet.to_string();
        assert_eq!(output, "\u{0015}0_0\u{0015}");
    }
}
