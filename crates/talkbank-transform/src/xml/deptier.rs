//! Text-content dependent-tier emission.
//!
//! These are the "side tiers" that Java Chatter maps uniformly to
//! `<a type="…">text</a>` elements inside `<u>`, placed after the
//! terminator / media / `<wor>` block:
//!
//! | CHAT tier | XML shape |
//! |-----------|-----------|
//! | `%act`    | `<a type="actions">…</a>` |
//! | `%com`    | `<a type="comments">…</a>` |
//! | `%exp`    | `<a type="explanation">…</a>` |
//! | `%gpx`    | `<a type="gesture">…</a>` |
//! | `%sit`    | `<a type="situation">…</a>` |
//! | `%xLABEL` | `<a type="extension" flavor="LABEL">…</a>` |
//!
//! Tiers with richer per-word structure (`%mor`, `%gra`, `%wor`,
//! `%pho`, `%mod`, `%sin`) are handled in their own submodules. The
//! remaining text-content tiers (`%add`, `%int`, `%spa`, `%cod` and
//! the simple `TextTier` variants `%alt`, `%coh`, `%def`, `%eng`,
//! `%err`, `%fac`, `%flo`, plus syllable tiers) are staged — they
//! fall through to `FeatureNotImplemented` so the harness surfaces
//! each one as a distinct increment rather than silently dropping
//! it.

use quick_xml::events::{BytesEnd, BytesStart, Event};

use talkbank_model::model::DependentTier;

use super::error::XmlWriteError;
use super::mor::tier_kind;
use super::root::bullet_content_plain_text;
use super::writer::{XmlEmitter, escape_text};

impl XmlEmitter {
    /// Emit one or more `<a type="…">text</a>` elements for the
    /// "side tiers" collected during utterance-tier classification.
    /// Tiers are emitted in the order the CHAT source presents them
    /// — the comparator ignores order, but preserving it minimizes
    /// diff noise during development.
    pub(super) fn emit_side_tiers(
        &mut self,
        side_tiers: &[&DependentTier],
    ) -> Result<(), XmlWriteError> {
        for tier in side_tiers {
            self.emit_side_tier(tier)?;
        }
        Ok(())
    }

    fn emit_side_tier(&mut self, tier: &DependentTier) -> Result<(), XmlWriteError> {
        let (tag_type, flavor, text): (&str, Option<&str>, String) = match tier {
            DependentTier::Act(t) => ("actions", None, bullet_content_plain_text(&t.content)?),
            DependentTier::Com(t) => ("comments", None, bullet_content_plain_text(&t.content)?),
            DependentTier::Exp(t) => ("explanation", None, bullet_content_plain_text(&t.content)?),
            DependentTier::Sit(t) => ("situation", None, bullet_content_plain_text(&t.content)?),
            DependentTier::Gpx(t) => ("gesture", None, bullet_content_plain_text(&t.content)?),
            // `%xLABEL` — the tier label carries the x-prefix (e.g.
            // `xpho` for `%xpho`). Java Chatter strips the prefix
            // and uses the remainder as the `flavor` attribute.
            DependentTier::UserDefined(t) => {
                let label = t.label.as_str();
                let flavor = label.strip_prefix('x').unwrap_or(label);
                ("extension", Some(flavor), t.content.as_str().to_owned())
            }
            other => {
                return Err(XmlWriteError::FeatureNotImplemented {
                    feature: format!("side tier emission for {}", tier_kind(other)),
                });
            }
        };

        let mut tag = BytesStart::new("a");
        tag.push_attribute(("type", tag_type));
        if let Some(flavor) = flavor {
            tag.push_attribute(("flavor", flavor));
        }
        self.writer.write_event(Event::Start(tag))?;
        self.writer.write_event(Event::Text(escape_text(&text)))?;
        self.writer.write_event(Event::End(BytesEnd::new("a")))?;
        Ok(())
    }
}
