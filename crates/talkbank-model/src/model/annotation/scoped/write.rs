//! CHAT serialization for scoped annotations (`[*]`, `[=]`, `[/]`, ...).
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Scoped_Symbols>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Error_Coding>
//!
//! This module keeps display semantics centralized so annotation labels can be
//! rendered consistently in validation diagnostics, caret reconstruction, and
//! batchalign exports.

use super::ScopedAnnotation;

impl ScopedAnnotation {
    /// Serializes one scoped annotation in canonical CHAT bracket syntax.
    ///
    /// Known variants emit normalized spellings (`[*]`, `[/]`, `[= ...]`, ...),
    /// while unknown markers are preserved in a lossy-free fallback form so
    /// roundtrip tooling can keep corpus-specific extensions intact. Keeping
    /// formatting logic centralized here guarantees consistent rendering across
    /// main-tier, bracketed, and replacement contexts.
    pub fn write_chat<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        match self {
            ScopedAnnotation::Error(error) => match error.code.as_ref() {
                None => w.write_str("[*]"),
                Some(code) => {
                    w.write_str("[* ")?;
                    w.write_str(code)?;
                    w.write_char(']')
                }
            },
            ScopedAnnotation::Explanation(explanation) => {
                w.write_str("[= ")?;
                w.write_str(&explanation.text)?;
                w.write_char(']')
            }
            ScopedAnnotation::Addition(addition) => {
                w.write_str("[+ ")?;
                w.write_str(&addition.text)?;
                w.write_char(']')
            }
            ScopedAnnotation::OverlapBegin(overlap) => {
                if let Some(idx) = overlap.index.as_ref() {
                    write!(w, "[<{}]", idx)
                } else {
                    w.write_str("[<]")
                }
            }
            ScopedAnnotation::OverlapEnd(overlap) => {
                if let Some(idx) = overlap.index.as_ref() {
                    write!(w, "[>{}]", idx)
                } else {
                    w.write_str("[>]")
                }
            }
            ScopedAnnotation::PartialRetracing => w.write_str("[/]"),
            ScopedAnnotation::Retracing => w.write_str("[//]"),
            ScopedAnnotation::MultipleRetracing => w.write_str("[///]"),
            ScopedAnnotation::Reformulation => w.write_str("[/-]"),
            ScopedAnnotation::UncertainRetracing => w.write_str("[/?]"),
            ScopedAnnotation::CaContinuationMarker => w.write_str("[^c]"),
            ScopedAnnotation::ScopedStressing => w.write_str("[!]"),
            ScopedAnnotation::ScopedContrastiveStressing => w.write_str("[!!]"),
            ScopedAnnotation::ScopedBestGuess => w.write_str("[!*]"),
            ScopedAnnotation::ScopedUncertain => w.write_str("[?]"),
            ScopedAnnotation::Paralinguistic(paralinguistic) => {
                w.write_str("[=! ")?;
                w.write_str(&paralinguistic.text)?;
                w.write_char(']')
            }
            ScopedAnnotation::Alternative(alternative) => {
                w.write_str("[=? ")?;
                w.write_str(&alternative.text)?;
                w.write_char(']')
            }
            ScopedAnnotation::PercentComment(comment) => {
                w.write_str("[% ")?;
                w.write_str(&comment.text)?;
                w.write_char(']')
            }
            ScopedAnnotation::Duration(duration) => {
                w.write_str("[# ")?;
                w.write_str(&duration.time)?;
                w.write_char(']')
            }
            ScopedAnnotation::ExcludeMarker => w.write_str("[e]"),
            ScopedAnnotation::Unknown(unknown) => {
                w.write_char('[')?;
                w.write_str(&unknown.marker)?;
                if !unknown.text.is_empty() {
                    w.write_char(' ')?;
                    w.write_str(&unknown.text)?;
                }
                w.write_char(']')
            }
        }
    }
}

impl std::fmt::Display for ScopedAnnotation {
    /// Formats one scoped annotation using CHAT serialization.
    ///
    /// This wraps [`Self::write_chat`] so consumers writing to `Display` receive
    /// identical output without dealing with format buffers directly.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_chat(f)
    }
}
