//! Word-level timing emission — utterance-level `<media>` and the
//! `<wor>` tier.
//!
//! Two elements live in this file because they share a common
//! concern: lowering millisecond-integer bullets to the seconds-
//! float XML shape Java Chatter emits. Keeping them together avoids
//! duplicated formatting logic.
//!
//! # XML shape
//!
//! ```xml
//! <u who="…" uID="…">
//!   <w>…</w>            (main-tier words via super::word)
//!   <t type="…"/>       (terminator via super::word)
//!   <media start="0.000" end="3.042" unit="s"/>    ← from main.content.bullet
//!   <wor>
//!     <w>let's</w>
//!     <internal-media start="0.000" end="0.240" unit="s"/>
//!     <w>see</w>
//!     <internal-media start="0.240" end="0.380" unit="s"/>
//!     …
//!   </wor>
//! </u>
//! ```
//!
//! # Unit conversion
//!
//! Bullets in the CHAT model are integer milliseconds (`MediaTiming`
//! with `start_ms` / `end_ms` as `u64`). Java Chatter's XML emits
//! seconds with three-decimal precision (`"3.042"`). The conversion
//! is lossless for any millisecond value and keeps the golden diffs
//! exact-match.

use quick_xml::events::{BytesEnd, BytesStart, Event};

use talkbank_model::model::dependent_tier::wor::WorItem;
use talkbank_model::model::{Bullet, Terminator, WorTier};

use super::error::XmlWriteError;
use super::writer::{XmlEmitter, escape_text};

impl XmlEmitter {
    /// Emit `<media start="…" end="…" unit="s"/>` for a main-tier
    /// trailing bullet (the `Utterance.main.content.bullet` slot).
    /// Java Chatter emits this between the terminator `<t>` and any
    /// following `<wor>` tier.
    pub(super) fn emit_utterance_media(&mut self, bullet: &Bullet) -> Result<(), XmlWriteError> {
        let start = format_seconds(bullet.timing.start_ms);
        let end = format_seconds(bullet.timing.end_ms);
        let mut tag = BytesStart::new("media");
        tag.push_attribute(("start", start.as_str()));
        tag.push_attribute(("end", end.as_str()));
        tag.push_attribute(("unit", "s"));
        self.writer.write_event(Event::Empty(tag))?;
        Ok(())
    }

    /// Emit `<wor><w>text</w><internal-media .../>…</wor>` for a
    /// `%wor` tier. Word text is copied from `Word.cleaned_text()`
    /// — per the model docs, that text is display-only "eye candy"
    /// in the CHAT source and is allowed to differ from the main
    /// tier. Timing comes from `Word.inline_bullet` on each WorItem.
    ///
    /// Separators (`,`, `„`, `‡`) inside `%wor` are a staged
    /// increment — their schema shape (`<tagMarker>` inside `<wor>`?
    /// an untimed `<w>`?) needs to be pinned to Java's output on a
    /// file that exercises them before implementation. Encountering
    /// one today reports `FeatureNotImplemented` so the harness
    /// makes the gap visible.
    pub(super) fn emit_wor(&mut self, wor: &WorTier) -> Result<(), XmlWriteError> {
        self.writer
            .write_event(Event::Start(BytesStart::new("wor")))?;

        for item in wor.items.iter() {
            match item {
                WorItem::Word(word) => {
                    self.writer
                        .write_event(Event::Start(BytesStart::new("w")))?;
                    self.writer
                        .write_event(Event::Text(escape_text(word.cleaned_text())))?;
                    self.writer.write_event(Event::End(BytesEnd::new("w")))?;

                    if let Some(bullet) = &word.inline_bullet {
                        let start = format_seconds(bullet.timing.start_ms);
                        let end = format_seconds(bullet.timing.end_ms);
                        let mut tag = BytesStart::new("internal-media");
                        tag.push_attribute(("start", start.as_str()));
                        tag.push_attribute(("end", end.as_str()));
                        tag.push_attribute(("unit", "s"));
                        self.writer.write_event(Event::Empty(tag))?;
                    }
                    // A %wor word without an inline_bullet is legal
                    // per the model (`inline_bullet: Option<Bullet>`)
                    // but the Java goldens always carry timing on
                    // every word. No `<internal-media>` is emitted in
                    // the missing-bullet case — matches the schema
                    // (internal-media is optional).
                }
                WorItem::Separator { .. } => {
                    return Err(XmlWriteError::FeatureNotImplemented {
                        feature: "%wor separator (`,` / `„` / `‡`) XML shape".to_owned(),
                    });
                }
            }
        }

        // Java Chatter closes every `<wor>` block with a terminator
        // `<t type="…"/>` matching the utterance's main-tier
        // terminator. Staged variants (trailing-off, interruption)
        // follow the same staged-feature pattern as main-tier
        // terminators — fail loud so the harness surfaces them.
        if let Some(terminator) = &wor.terminator {
            let ty = wor_terminator_type(terminator)?;
            let mut t = BytesStart::new("t");
            t.push_attribute(("type", ty));
            self.writer.write_event(Event::Empty(t))?;
        }

        self.writer.write_event(Event::End(BytesEnd::new("wor")))?;
        Ok(())
    }
}

/// Map a `%wor` terminator to the `<t type="…"/>` attribute value
/// inside `<wor>`. Identical to the main-tier mapping in
/// `super::word::emit_terminator`, intentionally duplicated here to
/// keep the `%wor` emission self-contained and because the staged
/// feature set may diverge (some CA terminators are valid on
/// `%wor` but not the main tier, or vice versa).
fn wor_terminator_type(terminator: &Terminator) -> Result<&'static str, XmlWriteError> {
    Ok(match terminator {
        Terminator::Period { .. } => "p",
        Terminator::Question { .. } => "q",
        Terminator::Exclamation { .. } => "e",
        _ => {
            return Err(XmlWriteError::FeatureNotImplemented {
                feature: "non-standard terminator inside %wor".to_owned(),
            });
        }
    })
}

/// Lossless millisecond → `"S.sss"` seconds formatter. `3042` →
/// `"3.042"`, `500` → `"0.500"`, `0` → `"0.000"`. Always three
/// decimal places, matching Java Chatter.
fn format_seconds(ms: u64) -> String {
    let whole = ms / 1000;
    let frac = ms % 1000;
    format!("{whole}.{frac:03}")
}

#[cfg(test)]
mod tests {
    use super::format_seconds;

    #[test]
    fn format_seconds_pads_fractional() {
        assert_eq!(format_seconds(0), "0.000");
        assert_eq!(format_seconds(1), "0.001");
        assert_eq!(format_seconds(50), "0.050");
        assert_eq!(format_seconds(500), "0.500");
        assert_eq!(format_seconds(1000), "1.000");
        assert_eq!(format_seconds(1234), "1.234");
        assert_eq!(format_seconds(3042), "3.042");
        assert_eq!(format_seconds(307628), "307.628");
    }
}
