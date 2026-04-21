//! `%mor` / `%gra` emission — morphology tier handling inside
//! `<w>`, `<tagMarker>`, and `<t>` elements.
//!
//! This submodule owns:
//! - The `<mor type="mor"><mw>…</mw>[<gra/>]</mor>` subtree emission
//!   attached to every main-tier chunk that has a paired `%mor` item.
//! - The `<mw><pos><c>…</c></pos><stem>…</stem><mk type="sfx">…</mk></mw>`
//!   serialization of a single [`MorWord`].
//! - The `<gra type="gra" index=… head=… relation=…/>` serialization
//!   of a single [`GrammaticalRelation`].
//! - The per-utterance tier collector [`UtteranceTiers`] + helper that
//!   separates `%mor` / `%gra` from other dependent tiers (the rest
//!   are staged increments).
//!
//! The `XmlEmitter` instance is owned by `super::writer`; this file
//! extends it via a separate `impl` block.

use quick_xml::events::{BytesEnd, BytesStart, Event};

use talkbank_model::model::WriteChat;
use talkbank_model::model::{DependentTier, GrammaticalRelation, Mor, MorWord, WorTier};

use super::error::XmlWriteError;
use super::writer::{XmlEmitter, escape_text};

impl XmlEmitter {
    /// Emit the `<mor type="mor"><mw>...</mw>[<gra.../>]</mor>` subtree
    /// inside a `<w>` or `<tagMarker>`. Post-clitic Mor items and
    /// non-`Mor` MorTierType values are staged increments — fail loud
    /// on encounter so the golden harness points at the missing feature.
    pub(super) fn emit_word_mor_subtree(
        &mut self,
        mor: &Mor,
        gra: Option<&[GrammaticalRelation]>,
        chunk_index_1based: usize,
    ) -> Result<(), XmlWriteError> {
        let mut mor_start = BytesStart::new("mor");
        mor_start.push_attribute(("type", "mor"));
        self.writer.write_event(Event::Start(mor_start))?;

        // Main morphological word + its %gra edge at the caller-
        // supplied chunk index.
        self.emit_morword(&mor.main)?;
        if let Some(rel) = gra_entry(gra, chunk_index_1based) {
            self.emit_gra(rel)?;
        }

        // Post-clitics (`~aux|be-Fin-...` chained onto the main mor
        // via `~`) each consume one additional `%gra` index.
        // `what's` with mor `pron|what-Int-S1~aux|be-Fin-Ind-Pres-S3`
        // emits the `<aux|be>` post-clitic inside `<mor-post>` with
        // index = main_index + 1.
        for (offset, post_clitic) in mor.post_clitics.iter().enumerate() {
            self.writer
                .write_event(Event::Start(BytesStart::new("mor-post")))?;
            self.emit_morword(post_clitic)?;
            let post_index = chunk_index_1based + offset + 1;
            if let Some(rel) = gra_entry(gra, post_index) {
                self.emit_gra(rel)?;
            }
            self.writer
                .write_event(Event::End(BytesEnd::new("mor-post")))?;
        }

        self.writer.write_event(Event::End(BytesEnd::new("mor")))?;
        Ok(())
    }

    /// Serialize a single [`MorWord`] as `<mw><pos><c>POS</c></pos>
    /// <stem>LEMMA</stem>[<mk type="sfx">FEATURE</mk>…]</mw>`.
    fn emit_morword(&mut self, word: &MorWord) -> Result<(), XmlWriteError> {
        self.writer
            .write_event(Event::Start(BytesStart::new("mw")))?;

        // <pos><c>noun</c></pos>
        self.writer
            .write_event(Event::Start(BytesStart::new("pos")))?;
        self.writer
            .write_event(Event::Start(BytesStart::new("c")))?;
        self.writer
            .write_event(Event::Text(escape_text(word.pos.as_str())))?;
        self.writer.write_event(Event::End(BytesEnd::new("c")))?;
        self.writer.write_event(Event::End(BytesEnd::new("pos")))?;

        // <stem>lemma</stem>
        self.writer
            .write_event(Event::Start(BytesStart::new("stem")))?;
        self.writer
            .write_event(Event::Text(escape_text(word.lemma.as_str())))?;
        self.writer.write_event(Event::End(BytesEnd::new("stem")))?;

        // <mk type="sfx">Feature</mk> per flat feature. Keyed
        // features (key=value form) are a separate TDD increment.
        for feature in word.features.iter() {
            let text = format_mor_feature(feature)?;
            let mut mk = BytesStart::new("mk");
            mk.push_attribute(("type", "sfx"));
            self.writer.write_event(Event::Start(mk))?;
            self.writer.write_event(Event::Text(escape_text(&text)))?;
            self.writer.write_event(Event::End(BytesEnd::new("mk")))?;
        }

        self.writer.write_event(Event::End(BytesEnd::new("mw")))?;
        Ok(())
    }

    /// Serialize a single [`GrammaticalRelation`] as a self-closing
    /// `<gra type="gra" index=… head=… relation=…/>`.
    pub(super) fn emit_gra(&mut self, rel: &GrammaticalRelation) -> Result<(), XmlWriteError> {
        let index_str = rel.index.to_string();
        let head_str = rel.head.to_string();
        let relation_str = rel.relation.as_str().to_owned();
        let mut gra = BytesStart::new("gra");
        gra.push_attribute(("type", "gra"));
        gra.push_attribute(("index", index_str.as_str()));
        gra.push_attribute(("head", head_str.as_str()));
        gra.push_attribute(("relation", relation_str.as_str()));
        self.writer.write_event(Event::Empty(gra))?;
        Ok(())
    }
}

/// Borrowed handles to the `%mor` and `%gra` tiers (if present) on
/// one utterance. Holding references means the emitter walks the
/// original AST in place — no cloning of tier payloads.
pub(super) struct UtteranceTiers<'u> {
    pub(super) mor: Option<&'u talkbank_model::model::MorTier>,
    pub(super) gra: Option<&'u [GrammaticalRelation]>,
    /// `%wor` word-timing sidecar. Captured here alongside `%mor` /
    /// `%gra` so `emit_utterance` can orchestrate element order
    /// (`<wor>` follows the terminator and the utterance-level
    /// `<media>`).
    pub(super) wor: Option<&'u WorTier>,
    /// Text-content "side tiers" (`%act`, `%com`, `%exp`, `%gpx`,
    /// `%sit`, `%xLABEL`). Each becomes an `<a type="…">text</a>`
    /// element emitted after `<wor>` inside `<u>`. See
    /// [`super::deptier`].
    pub(super) side_tiers: Vec<&'u DependentTier>,
}

/// Split an utterance's dependent tiers into recognized `%mor` /
/// `%gra` / `%wor` / text-content slots. Phonetic / syllabification
/// tiers (`%pho`, `%mod`, `%phosyl`, `%modsyl`, `%phoaln`) report
/// [`XmlWriteError::PhoneticTierUnsupported`] — these are
/// permanently out of scope, not staged. Any other tier kind reports
/// `FeatureNotImplemented` so the harness surfaces each missing
/// staged feature individually.
pub(super) fn collect_utterance_tiers(
    utterance: &talkbank_model::model::Utterance,
    utterance_index: usize,
) -> Result<UtteranceTiers<'_>, XmlWriteError> {
    let mut out = UtteranceTiers {
        mor: None,
        gra: None,
        wor: None,
        side_tiers: Vec::new(),
    };
    for tier in utterance.dependent_tiers.iter() {
        match tier {
            // Text-content tiers with a known Java-Chatter XML shape
            // (`<a type="…">`) collect into `side_tiers`; emission
            // happens in `super::deptier::emit_side_tiers` after the
            // main-tier block.
            DependentTier::Act(_)
            | DependentTier::Com(_)
            | DependentTier::Exp(_)
            | DependentTier::Sit(_)
            | DependentTier::Gpx(_)
            | DependentTier::UserDefined(_) => {
                out.side_tiers.push(tier);
                continue;
            }
            DependentTier::Mor(m) => {
                if out.mor.is_some() {
                    return Err(XmlWriteError::FeatureNotImplemented {
                        feature: "multiple %mor tiers on one utterance".to_owned(),
                    });
                }
                out.mor = Some(m);
            }
            DependentTier::Gra(g) => {
                if out.gra.is_some() {
                    return Err(XmlWriteError::FeatureNotImplemented {
                        feature: "multiple %gra tiers on one utterance".to_owned(),
                    });
                }
                out.gra = Some(&g.relations.0);
            }
            DependentTier::Wor(w) => {
                if out.wor.is_some() {
                    return Err(XmlWriteError::FeatureNotImplemented {
                        feature: "multiple %wor tiers on one utterance".to_owned(),
                    });
                }
                out.wor = Some(w);
            }
            DependentTier::Pho(_)
            | DependentTier::Mod(_)
            | DependentTier::Modsyl(_)
            | DependentTier::Phosyl(_)
            | DependentTier::Phoaln(_) => {
                // Phon-specific phonetic / syllabification tiers are
                // permanently out of scope — see
                // `XmlWriteError::PhoneticTierUnsupported`.
                return Err(XmlWriteError::PhoneticTierUnsupported { utterance_index });
            }
            other => {
                return Err(XmlWriteError::FeatureNotImplemented {
                    feature: format!("dependent tier '{}'", tier_kind(other)),
                });
            }
        }
    }
    Ok(out)
}

/// Look up the grammatical-relation edge for a 1-based chunk index.
/// Returns `None` when `%gra` is absent or the chunk is unaligned; the
/// caller omits the `<gra/>` child in that case.
pub(super) fn gra_entry(
    gra: Option<&[GrammaticalRelation]>,
    chunk_index_1based: usize,
) -> Option<&GrammaticalRelation> {
    let relations = gra?;
    relations.iter().find(|r| r.index == chunk_index_1based)
}

/// Serialize one [`talkbank_model::model::MorFeature`] back to its
/// compact CHAT token (`Fin`, `key=value`, …). This is the content of
/// the `<mk type="sfx">` element. Uses the feature's own `WriteChat`
/// impl so the XML emitter stays in lockstep with the rest of the
/// toolchain's CHAT serialization.
fn format_mor_feature(
    feature: &talkbank_model::model::MorFeature,
) -> Result<String, XmlWriteError> {
    let mut buf = String::new();
    feature
        .write_chat(&mut buf)
        .map_err(|e| XmlWriteError::MissingMetadata {
            what: format!("failed to format MorFeature: {e}"),
        })?;
    Ok(buf)
}

/// Short display name for a [`DependentTier`] variant. Used only in
/// `FeatureNotImplemented` diagnostics, so we keep it as a flat match
/// rather than reaching into each tier's label newtype. Shared with
/// [`super::deptier`].
pub(super) fn tier_kind(tier: &DependentTier) -> &'static str {
    match tier {
        DependentTier::Mor(_) => "%mor",
        DependentTier::Gra(_) => "%gra",
        DependentTier::Wor(_) => "%wor",
        DependentTier::Pho(_) => "%pho",
        DependentTier::Mod(_) => "%mod",
        DependentTier::Sin(_) => "%sin",
        DependentTier::Act(_) => "%act",
        DependentTier::Cod(_) => "%cod",
        DependentTier::Add(_) => "%add",
        DependentTier::Com(_) => "%com",
        DependentTier::Exp(_) => "%exp",
        DependentTier::Gpx(_) => "%gpx",
        DependentTier::Int(_) => "%int",
        DependentTier::Sit(_) => "%sit",
        DependentTier::Spa(_) => "%spa",
        DependentTier::Alt(_) => "%alt",
        DependentTier::Coh(_) => "%coh",
        DependentTier::Def(_) => "%def",
        DependentTier::Eng(_) => "%eng",
        DependentTier::Err(_) => "%err",
        DependentTier::Fac(_) => "%fac",
        DependentTier::Flo(_) => "%flo",
        DependentTier::Modsyl(_) => "%modsyl",
        DependentTier::Phosyl(_) => "%phosyl",
        DependentTier::Phoaln(_) => "%phoaln",
        DependentTier::UserDefined(_) => "%xLABEL",
        DependentTier::Unsupported(_) => "(unsupported tier)",
        _ => "(other dependent tier)",
    }
}
