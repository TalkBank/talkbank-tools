//! Document-level emission — the `<CHAT>` root, `<Participants>`,
//! body-level headers and comments, and utterance orchestration.
//!
//! This file owns the "top-down" traversal of a `ChatFile`. Word-level
//! emission delegates to `super::word`; morphology subtrees delegate
//! to `super::mor`. Metadata helpers (corpus lookup, date/age/sex
//! formatting, `@Options` flags, per-speaker extras from body-level
//! `@Birthplace` / `@L1` headers) also live here because they feed
//! attributes on `<CHAT>` and `<participant>`.

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use std::collections::HashMap;

use talkbank_model::alignment::TierDomain;
use talkbank_model::alignment::helpers::{counts_for_tier, is_tag_marker_separator};
use talkbank_model::model::{
    AgeValue, BulletContent, BulletContentSegment, ChatDate, ChatFile, ChatOptionFlag, Header,
    Line, Month, Sex, SpeakerCode, UtteranceContent,
};
use talkbank_model::validation::ValidationState;

use super::error::XmlWriteError;
use super::mor::collect_utterance_tiers;
use super::writer::{
    SCHEMA_LOCATION, SCHEMA_VERSION, TALKBANK_NS, XSI_NS, XmlEmitter, escape_text,
};

impl XmlEmitter {
    /// Serialize the full document: XML decl, `<CHAT>` root with its
    /// attributes, `<Participants>`, and the body. This is the single
    /// entry point invoked by the public `write_chat_xml` wrapper.
    pub(super) fn emit_document<S: ValidationState>(
        &mut self,
        file: &ChatFile<S>,
    ) -> Result<(), XmlWriteError> {
        // <?xml version="1.0" encoding="UTF-8"?>
        self.writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

        let corpus = find_corpus(file)?;

        let mut root = BytesStart::new("CHAT");
        root.push_attribute(("xmlns:xsi", XSI_NS));
        root.push_attribute(("xmlns", TALKBANK_NS));
        root.push_attribute(("xsi:schemaLocation", SCHEMA_LOCATION));
        // `@Media` comes before `Version` in Java Chatter's output on
        // audio/video files. The structural comparator ignores order,
        // but we keep Java's ordering to minimize diff noise during
        // development.
        if let Some(media) = file.media.as_deref() {
            // CHAT `@Media:` values can be bare filenames or
            // double-quoted URLs (`"https://…"`). The XSD
            // `mediaRefType` is `xs:anyURI`, which rejects the
            // embedded quotes — strip them at the emission
            // boundary so the attribute is schema-legal.
            let raw = media.filename.as_str();
            let stripped = raw
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .unwrap_or(raw);
            root.push_attribute(("Media", stripped));
            root.push_attribute(("Mediatypes", media.media_type.as_str()));
        }
        root.push_attribute(("Version", SCHEMA_VERSION));
        root.push_attribute(("Lang", join_language_codes(file).as_str()));
        if let Some(options) = format_options_attribute(file) {
            root.push_attribute(("Options", options.as_str()));
        }
        root.push_attribute(("Corpus", corpus.as_str()));
        if let Some(pid) = find_root_pid(file) {
            root.push_attribute(("PID", pid));
        }
        if let Some(date) = find_root_date(file)? {
            root.push_attribute(("Date", date.as_str()));
        }
        // `@Types: design, activity, group` projects onto three root
        // attributes in addition to the body-level `<comment
        // type="Types">` (which is emitted during the body walk).
        // Java Chatter names these
        // `DesignType` / `ActivityType` / `GroupType`.
        if let Some(types) = find_root_types(file) {
            root.push_attribute(("DesignType", types.design.as_str()));
            root.push_attribute(("ActivityType", types.activity.as_str()));
            root.push_attribute(("GroupType", types.group.as_str()));
        }
        self.writer.write_event(Event::Start(root))?;

        self.emit_participants(file)?;
        self.emit_body(file)?;

        self.writer.write_event(Event::End(BytesEnd::new("CHAT")))?;
        Ok(())
    }

    /// Emit the `<Participants>` block. Pulls speaker metadata from
    /// the `@Participants` / `@ID` headers plus body-level
    /// `@Birthplace` / `@L1` extras collected via
    /// [`collect_per_speaker_metadata`].
    fn emit_participants<S: ValidationState>(
        &mut self,
        file: &ChatFile<S>,
    ) -> Result<(), XmlWriteError> {
        // Pre-scan body headers that attach extra metadata to an existing
        // participant (`@Birthplace of X`, `@L1 of X`). These live outside
        // Participant itself because they are independent CHAT headers.
        let extra = collect_per_speaker_metadata(file);

        self.writer
            .write_event(Event::Start(BytesStart::new("Participants")))?;

        for participant in file.participants.values() {
            // Attribute order below matches Java Chatter's output to make
            // manual diffing against the golden easier; the structural
            // comparator treats order as insignificant.
            let mut start = BytesStart::new("participant");
            start.push_attribute(("id", participant.code.as_str()));
            start.push_attribute(("role", participant.role.as_ref()));

            let lang = join_codes_with_space(&participant.id.language.0);
            start.push_attribute(("language", lang.as_str()));

            if let Some(age) = &participant.id.age {
                let iso = format_age_iso8601(age)?;
                start.push_attribute(("age", iso.as_str()));
            }
            if let Some(sex) = &participant.id.sex {
                start.push_attribute(("sex", sex_to_xml(sex)?));
            }
            if let Some(group) = &participant.id.group {
                start.push_attribute(("group", group.as_str()));
            }
            if let Some(name) = &participant.name {
                start.push_attribute(("name", name.as_str()));
            }
            // `ses` / `education` and SES's typed variants are staged
            // for a later increment; reaching them here is intentional
            // TDD work, not silently dropped.
            if let Some(birth_date) = &participant.birth_date {
                let iso = format_chat_date_iso(birth_date)?;
                start.push_attribute(("birthday", iso.as_str()));
            }
            if let Some(meta) = extra.get(&participant.code) {
                if let Some(place) = &meta.birthplace {
                    start.push_attribute(("birthplace", place.as_str()));
                }
                if let Some(lang1) = &meta.first_language {
                    start.push_attribute(("first-language", lang1.as_str()));
                }
            }
            if let Some(custom) = &participant.id.custom_field {
                start.push_attribute(("custom-field", custom.as_str()));
            }

            self.writer.write_event(Event::Empty(start))?;
        }

        self.writer
            .write_event(Event::End(BytesEnd::new("Participants")))?;
        Ok(())
    }

    /// Walk `file.lines` in order, dispatching each line to either a
    /// body-level header emitter or an utterance emitter. Root-level
    /// headers (`@Begin`, `@Languages`, `@ID`, …) that have already
    /// contributed attributes on `<CHAT>` are filtered out inside
    /// [`Self::emit_header_if_body`].
    fn emit_body<S: ValidationState>(&mut self, file: &ChatFile<S>) -> Result<(), XmlWriteError> {
        for line in file.lines.iter() {
            match line {
                Line::Header { header, .. } => self.emit_header_if_body(header)?,
                Line::Utterance(utterance) => self.emit_utterance(utterance)?,
            }
        }
        Ok(())
    }

    /// Most headers contribute to the root element or the Participants
    /// block and have already been consumed by `emit_document`; only
    /// body-level headers emit their own XML element. Stage 1 handles
    /// `@Comment`; all other body-level headers (`@Bg`/`@Eg`, `@G`,
    /// `@Media`, `@Situation`, `@Date`, `@Pid`, `@Types`, pre-begin
    /// headers, warnings, etc.) report `FeatureNotImplemented`.
    fn emit_header_if_body(&mut self, header: &Header) -> Result<(), XmlWriteError> {
        match header {
            // Scaffold + root-attribute + per-speaker metadata headers
            // already consumed by `emit_document` / `emit_participants`.
            Header::Utf8
            | Header::Begin
            | Header::End
            | Header::Languages { .. }
            | Header::Participants { .. }
            | Header::ID(_)
            | Header::Birth { .. }
            | Header::Birthplace { .. }
            | Header::L1Of { .. }
            | Header::Options { .. }
            | Header::Media(_)
            | Header::Pid { .. } => Ok(()),

            // @Bg/@Eg/@G gems render as standalone XML elements at
            // the same level as `<u>`, not as `<comment>` children.
            // The `label` attribute is required in the XSD; when CHAT
            // source omits it, emit an empty string to stay
            // schema-valid.
            Header::BeginGem { label } => self.emit_gem("begin-gem", gem_label(label)),
            Header::EndGem { label } => self.emit_gem("end-gem", gem_label(label)),
            Header::LazyGem { label } => self.emit_gem("lazy-gem", gem_label(label)),

            // @Date appears twice: once as a root `Date="YYYY-MM-DD"`
            // attribute and once as a `<comment type="Date">DD-MMM-
            // YYYY</comment>` preserving the original CHAT text.
            Header::Date { date } => self.emit_typed_comment("Date", date.as_str()),

            Header::Comment { content } => {
                self.emit_typed_comment("Generic", &bullet_content_plain_text(content)?)
            }
            Header::Location { location } => self.emit_typed_comment("Location", location.as_str()),
            Header::Situation { text } => self.emit_typed_comment("Situation", text.as_str()),
            Header::Activities { activities } => {
                self.emit_typed_comment("Activities", activities.as_str())
            }
            Header::Transcriber { transcriber } => {
                self.emit_typed_comment("Transcriber", transcriber.as_str())
            }
            Header::Transcription { transcription } => {
                self.emit_typed_comment("Transcription", transcription.as_str())
            }
            Header::Warning { text } => self.emit_typed_comment("Warning", text.as_str()),
            Header::Bck { bck } => self.emit_typed_comment("Bck", bck.as_str()),
            Header::Number { number } => self.emit_typed_comment("Number", number.as_str()),
            Header::RecordingQuality { quality } => {
                self.emit_typed_comment("Recording Quality", quality.as_str())
            }
            Header::TapeLocation { location } => {
                self.emit_typed_comment("Tape Location", location.as_str())
            }
            Header::TimeDuration { duration } => {
                self.emit_typed_comment("Time Duration", duration.as_str())
            }
            Header::TimeStart { start } => self.emit_typed_comment("Time Start", start.as_str()),
            Header::RoomLayout { layout } => {
                self.emit_typed_comment("Room Layout", layout.as_str())
            }
            Header::Page { page } => self.emit_typed_comment("Page", page.as_str()),
            Header::T { text } => self.emit_typed_comment("T", text.as_str()),

            // `@Types: design, activity, group` → `<comment
            // type="Types">design, activity, group</comment>`. The three
            // fields are always emitted, comma-space separated.
            Header::Types(types) => {
                let payload = format!(
                    "{}, {}, {}",
                    types.design.as_str(),
                    types.activity.as_str(),
                    types.group.as_str()
                );
                self.emit_typed_comment("Types", &payload)
            }

            // Display-preference headers (font/window/color-words)
            // and videos all route through a "Generic" comment since
            // the XSD doesn't have typed elements for them.
            Header::Font { font } => self.emit_typed_comment("Generic", font.as_str()),
            Header::Window { geometry } => self.emit_typed_comment("Generic", geometry.as_str()),
            Header::ColorWords { colors } => self.emit_typed_comment("Generic", colors.as_str()),
            Header::Videos { videos } => self.emit_typed_comment("Generic", videos.as_str()),
            // Marker headers with no payload. The XSD has dedicated
            // `"New Episode"` and `"Blank"` `commentTypeType` values.
            Header::NewEpisode => self.emit_typed_comment("New Episode", ""),
            Header::Blank => self.emit_typed_comment("Blank", ""),

            // Lenient-parse fallback. Preserve the original text in a
            // generic comment so the utterance stays well-formed; the
            // diagnostic `parse_reason` / `suggested_fix` fields are
            // validator metadata, not content, and don't project to XML.
            Header::Unknown { text, .. } => self.emit_typed_comment("Generic", text.as_str()),
        }
    }

    /// Emit `<begin-gem>` / `<end-gem>` / `<lazy-gem>` with the
    /// `label` attribute. Java Chatter emits them as standalone empty
    /// elements alongside `<u>`, not inside `<comment>`.
    fn emit_gem(&mut self, element: &'static str, label: &str) -> Result<(), XmlWriteError> {
        let mut tag = BytesStart::new(element);
        tag.push_attribute(("label", label));
        self.writer.write_event(Event::Empty(tag))?;
        Ok(())
    }
}

/// Resolve an optional [`GemLabel`] to the `label="…"` attribute
/// value for `<begin-gem>` / `<end-gem>` / `<lazy-gem>`. The XSD
/// marks `label` as required, so when CHAT source omits it we emit
/// an empty string — schema-legal and round-trippable.
fn gem_label(label: &Option<talkbank_model::model::GemLabel>) -> &str {
    label.as_ref().map(|l| l.as_str()).unwrap_or("")
}

// Re-open the `impl XmlEmitter` block for the remaining methods.
impl XmlEmitter {
    /// Emit `<comment type="X">text</comment>`. Shared by `@Comment`
    /// and the typed metadata headers that project onto comments in
    /// the XML schema.
    fn emit_typed_comment(&mut self, type_value: &str, text: &str) -> Result<(), XmlWriteError> {
        let mut start = BytesStart::new("comment");
        start.push_attribute(("type", type_value));
        self.writer.write_event(Event::Start(start))?;
        self.writer.write_event(Event::Text(escape_text(text)))?;
        self.writer
            .write_event(Event::End(BytesEnd::new("comment")))?;
        Ok(())
    }

    /// Emit a single `<u who=… uID=…>…<t/>…</u>` utterance. This is
    /// the orchestration point that walks main-tier content in parallel
    /// with the `%mor` cursor, dispatching each content item to the
    /// appropriate word-level emitter. The terminator closes the
    /// utterance and, if `%mor` carries a terminator chunk, picks up
    /// the matching `<gra/>` from chunk index `n+1`.
    fn emit_utterance(
        &mut self,
        utterance: &talkbank_model::model::Utterance,
    ) -> Result<(), XmlWriteError> {
        // Pre-begin headers attached to this utterance.
        for header in utterance.preceding_headers.iter() {
            self.emit_header_if_body(header)?;
        }

        // Split dependent tiers into recognized Mor / Gra / other.
        // Phonetic / syllabification tiers (%pho, %mod, %phosyl,
        // %modsyl, %phoaln) are permanently unsupported — see
        // `XmlWriteError::PhoneticTierUnsupported`. Other staged
        // tiers surface via `FeatureNotImplemented` one at a time.
        let tiers = collect_utterance_tiers(utterance, self.next_utterance_id as usize)?;

        let uid = format!("u{}", self.next_utterance_id);
        self.next_utterance_id += 1;

        let mut start = BytesStart::new("u");
        start.push_attribute(("who", utterance.main.speaker.as_str()));
        start.push_attribute(("uID", uid.as_str()));
        self.writer.write_event(Event::Start(start))?;

        // Discourse linkers (`+<`, `++`, `+≈`, `+≋`, …) sit at the
        // very start of tier content and render as `<linker
        // type="…"/>` children of `<u>` ahead of any `<w>` content.
        for linker in utterance.main.content.linkers.0.iter() {
            self.emit_linker(linker)?;
        }

        // Walk main-tier content in parallel with two cursors:
        //
        // - `mor_cursor` indexes into `mor_tier.items`. Each alignable
        //   content item (Word, Separator) consumes one Mor item —
        //   even when that item carries post-clitics (`~aux|be` is
        //   still *one* Mor, with `post_clitics` populated).
        // - `gra_chunk` is the 1-based `%gra` index for the next
        //   main `<mw>`. It advances by `1 + post_clitics.len()`
        //   per Mor because each post-clitic contributes its own
        //   `%gra` edge (see `emit_word_mor_subtree`).
        //
        // Pause and Retrace do not consume either cursor, matching
        // the CHAT manual's definition of mor alignability.
        let mut mor_cursor: usize = 0;
        let mut gra_chunk: usize = 1;
        for item in utterance.main.content.content.iter() {
            match item {
                UtteranceContent::Word(word) => {
                    // Leading overlap markers (`⌈`, `⌊`) attached to
                    // the front of a word get hoisted out as
                    // top-level `<overlap-point/>` siblings before
                    // the `<w>`. The Rust parser bundles them into
                    // `word.content`; Java Chatter emits them
                    // outside. Peel them here so the word body starts
                    // with its actual first lexical segment.
                    self.emit_leading_overlap_points(word)?;

                    // Nonword (`&~`), filler (`&-`), phonological
                    // fragment (`&+`), and untranscribed (`xxx` /
                    // `yyy` / `www`) tokens appear on the main tier
                    // but have no corresponding `%mor` item, so we
                    // pass `None` for `mor` and keep the cursor
                    // where it is. Using the model's canonical
                    // `counts_for_tier(TierDomain::Mor)` predicate
                    // keeps this check aligned with validation logic.
                    let counts = counts_for_tier(word, TierDomain::Mor);
                    let mor_for_word = if counts {
                        tiers
                            .mor
                            .as_ref()
                            .and_then(|mor| mor.items.0.get(mor_cursor))
                    } else {
                        None
                    };
                    self.emit_word(word, mor_for_word, tiers.gra, gra_chunk)?;
                    if counts && tiers.mor.is_some() {
                        let post_count = mor_for_word.map(|m| m.post_clitics.len()).unwrap_or(0);
                        mor_cursor += 1;
                        gra_chunk += 1 + post_count;
                    }
                }
                UtteranceContent::Separator(sep) => {
                    // Only Comma / Tag / Vocative separators
                    // participate in `%mor` alignment (they produce
                    // `cm|cm`, `end|end`, `beg|beg` mor items). CA
                    // intonation markers and other structural
                    // separators render to `<s>` / `<tagMarker>`
                    // without consuming mor chunks.
                    let counts_for_mor = is_tag_marker_separator(sep);
                    let mor_for_sep = if counts_for_mor {
                        tiers
                            .mor
                            .as_ref()
                            .and_then(|mor| mor.items.0.get(mor_cursor))
                    } else {
                        None
                    };
                    self.emit_separator(sep, mor_for_sep, tiers.gra, gra_chunk)?;
                    if counts_for_mor && tiers.mor.is_some() {
                        let post_count = mor_for_sep.map(|m| m.post_clitics.len()).unwrap_or(0);
                        mor_cursor += 1;
                        gra_chunk += 1 + post_count;
                    }
                }
                UtteranceContent::Pause(pause) => {
                    self.emit_pause(pause)?;
                }
                UtteranceContent::Retrace(retrace) => {
                    self.emit_retrace(retrace)?;
                }
                UtteranceContent::AnnotatedWord(annotated) => {
                    let mor_for_chunk = tiers.mor.and_then(|mor| mor.items.0.get(mor_cursor));
                    self.emit_annotated_word(annotated, mor_for_chunk, tiers.gra, gra_chunk)?;
                    if tiers.mor.is_some() {
                        let post_count = mor_for_chunk.map(|m| m.post_clitics.len()).unwrap_or(0);
                        mor_cursor += 1;
                        gra_chunk += 1 + post_count;
                    }
                }
                UtteranceContent::ReplacedWord(rw) => {
                    // Multi-word replacements (`dunno [: don't know]`)
                    // consume N mor items; `emit_replaced_word`
                    // reports the exact mor + gra counts so we can
                    // advance both cursors without re-walking the
                    // replacement words.
                    let (mor_used, gra_used) =
                        self.emit_replaced_word(rw, &tiers, mor_cursor, gra_chunk)?;
                    mor_cursor += mor_used;
                    gra_chunk += gra_used;
                }
                UtteranceContent::Event(event) => {
                    // Inline `&=descriptor` event in main-tier content
                    // (e.g. `&=laughs`). Events don't consume mor/gra
                    // chunks — they're outside the word alignment.
                    self.emit_event(event)?;
                }
                UtteranceContent::AnnotatedAction(annotated) => {
                    // Bare main-tier action (`0 .` utterance or `0`
                    // token) — scoped annotations on the action are a
                    // separate increment; the bare `<e><action/></e>`
                    // shape is all the reference corpus uses.
                    self.emit_annotated_action(annotated)?;
                }
                UtteranceContent::OverlapPoint(point) => {
                    // Top-level overlap markers (`⌈` / `⌉` / `⌊` /
                    // `⌋` appearing outside a word) render as
                    // `<overlap-point/>` children of `<u>`.
                    self.emit_overlap_point(point)?;
                }
                UtteranceContent::AnnotatedGroup(annotated) => {
                    // `<word1 word2> [annotation]` renders as
                    // `<g><w>word1</w><w>word2</w><annotation/></g>`.
                    // Each word inside the group consumes its own
                    // `%mor` item; `emit_annotated_group` returns
                    // the mor / gra cursor advances.
                    let (mor_used, gra_used) =
                        self.emit_annotated_group(annotated, &tiers, mor_cursor, gra_chunk)?;
                    mor_cursor += mor_used;
                    gra_chunk += gra_used;
                }
                UtteranceContent::AnnotatedEvent(annotated) => {
                    // `&=descriptor [!]` → `<e><happening>text</happening>
                    // <k type="stressing"/></e>`. Annotations on an
                    // event attach *inside* `<e>` rather than wrapping
                    // it in `<g>`, per the XSD `<e>` choice sequence.
                    self.emit_annotated_event(annotated)?;
                }
                UtteranceContent::Group(group) => {
                    // Bare `<word word>` group (no scoped annotations).
                    // Rendered as `<g>` with inner `<w>` children —
                    // same shape as `AnnotatedGroup` minus the
                    // annotation siblings.
                    let (mor_used, gra_used) =
                        self.emit_bare_group(group, &tiers, mor_cursor, gra_chunk)?;
                    mor_cursor += mor_used;
                    gra_chunk += gra_used;
                }
                UtteranceContent::Quotation(quotation) => {
                    self.emit_quotation(quotation)?;
                }
                UtteranceContent::Freecode(freecode) => {
                    self.emit_freecode(freecode)?;
                }
                UtteranceContent::LongFeatureBegin(lf) => {
                    self.emit_long_feature("begin", lf.label.as_str())?;
                }
                UtteranceContent::LongFeatureEnd(lf) => {
                    self.emit_long_feature("end", lf.label.as_str())?;
                }
                UtteranceContent::NonvocalBegin(nv) => {
                    self.emit_nonvocal("begin", nv.label.as_str())?;
                }
                UtteranceContent::NonvocalEnd(nv) => {
                    self.emit_nonvocal("end", nv.label.as_str())?;
                }
                UtteranceContent::NonvocalSimple(nv) => {
                    self.emit_nonvocal("simple", nv.label.as_str())?;
                }
                UtteranceContent::UnderlineBegin(_) => {
                    let mut tag = quick_xml::events::BytesStart::new("underline");
                    tag.push_attribute(("type", "begin"));
                    self.writer
                        .write_event(quick_xml::events::Event::Empty(tag))?;
                }
                UtteranceContent::UnderlineEnd(_) => {
                    let mut tag = quick_xml::events::BytesStart::new("underline");
                    tag.push_attribute(("type", "end"));
                    self.writer
                        .write_event(quick_xml::events::Event::Empty(tag))?;
                }
                UtteranceContent::InternalBullet(bullet) => {
                    // Standalone bullet inside main content (rare;
                    // usually bullets attach to a word) — emit as
                    // `<internal-media>` using the same seconds
                    // formatting as `%wor` bullets.
                    self.emit_internal_media(bullet)?;
                }
                UtteranceContent::OtherSpokenEvent(event) => {
                    // `&*WHO=word` interposed-speaker marker. Per the
                    // XSD, `<otherSpokenEvent>` nests inside `<e>`
                    // alongside `<action>` and `<happening>`.
                    self.emit_other_spoken_event(event)?;
                }
                UtteranceContent::PhoGroup(_) | UtteranceContent::SinGroup(_) => {
                    // `<pg>` / `<sg>` are Phon-specific structured
                    // payloads. Permanently out of scope (same
                    // policy as `%pho` / `%mod` tiers) — surface as
                    // `PhoneticTierUnsupported` at the utterance
                    // level rather than an open-ended
                    // `FeatureNotImplemented`.
                    return Err(XmlWriteError::PhoneticTierUnsupported {
                        utterance_index: self.next_utterance_id.saturating_sub(1) as usize,
                    });
                }
            }
        }

        // %mor always emits exactly one extra item (the terminator
        // chunk); we feed that index to the terminator emission so
        // its `<mor>` subtree picks up the matching `<gra>`.
        match utterance.main.content.terminator.as_ref() {
            Some(terminator) => {
                self.emit_terminator(terminator, &tiers, gra_chunk)?;
            }
            None => {
                // CA transcripts commonly end an utterance with a
                // pitch-contour separator (`⇗`, `⇘`, `∞`, …) and no
                // conventional terminator. Java Chatter emits
                // `<t type="missing CA terminator"/>` in that case.
                let mut tag = quick_xml::events::BytesStart::new("t");
                tag.push_attribute(("type", "missing CA terminator"));
                self.writer
                    .write_event(quick_xml::events::Event::Empty(tag))?;
            }
        }

        // Utterance-level `<media>` element: the main tier's trailing
        // bullet (`· start_end ·` after the terminator) becomes a
        // `<media start="s.sss" end="s.sss" unit="s"/>` sibling of
        // `<t>` in Java Chatter's output. Emission order is:
        // main-tier words → `<t>` → `<media>` → `<wor>`.
        if let Some(bullet) = utterance.main.content.bullet.as_ref() {
            self.emit_utterance_media(bullet)?;
        }

        // `<wor>` — the word-level timing sidecar. Emitted only when
        // the utterance carried a `%wor` tier.
        if let Some(wor) = tiers.wor {
            self.emit_wor(wor)?;
        }

        // Text-content "side tiers" (`%act`, `%com`, `%exp`, `%gpx`,
        // `%sit`, `%xLABEL`) become `<a type="…">text</a>` children
        // of `<u>` per Java Chatter's XML shape.
        if !tiers.side_tiers.is_empty() {
            self.emit_side_tiers(&tiers.side_tiers)?;
        }

        self.writer.write_event(Event::End(BytesEnd::new("u")))?;
        Ok(())
    }
}

/// Extracts the first `@ID` header's corpus field and returns it as a
/// string. Every reference-corpus file carries a populated corpus slot;
/// absence here indicates malformed input reaching the emitter.
fn find_corpus<S: ValidationState>(file: &ChatFile<S>) -> Result<String, XmlWriteError> {
    for line in file.lines.iter() {
        if let Line::Header { header, .. } = line
            && let Header::ID(id) = header.as_ref()
            && let Some(corpus) = &id.corpus
        {
            return Ok(corpus.as_ref().to_owned());
        }
    }
    Err(XmlWriteError::MissingMetadata {
        what: "Corpus attribute (no @ID header with a corpus field)".to_owned(),
    })
}

/// Space-joined language codes for the root `Lang` attribute, matching
/// Java Chatter's format (`"eng ara"`, not `"eng, ara"`).
fn join_language_codes<S: ValidationState>(file: &ChatFile<S>) -> String {
    join_codes_with_space(&file.languages.0)
}

fn join_codes_with_space(codes: &[talkbank_model::model::LanguageCode]) -> String {
    let mut out = String::new();
    for (i, code) in codes.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(code.as_str());
    }
    out
}

/// Locate the `@Types` header (if any). Returns a borrowed
/// reference so `emit_document` can push the `DesignType` /
/// `ActivityType` / `GroupType` attributes onto the `<CHAT>` root
/// without owning the underlying `TypesHeader`.
fn find_root_types<S: ValidationState>(
    file: &ChatFile<S>,
) -> Option<&talkbank_model::model::TypesHeader> {
    for line in file.lines.iter() {
        if let Line::Header { header, .. } = line
            && let Header::Types(types) = header.as_ref()
        {
            return Some(types);
        }
    }
    None
}

/// Locate the `@PID` header (if any). Returns a borrowed `&str` view
/// of the value for direct push onto the root element.
fn find_root_pid<S: ValidationState>(file: &ChatFile<S>) -> Option<&str> {
    for line in file.lines.iter() {
        if let Line::Header { header, .. } = line
            && let Header::Pid { pid } = header.as_ref()
        {
            return Some(pid.as_str());
        }
    }
    None
}

/// Find the `@Date` header (if any) and format it for the root
/// `Date="YYYY-MM-DD"` attribute. Returns `Ok(None)` when the file has
/// no `@Date`; returns an error if the date is present but unparseable.
fn find_root_date<S: ValidationState>(file: &ChatFile<S>) -> Result<Option<String>, XmlWriteError> {
    for line in file.lines.iter() {
        if let Line::Header { header, .. } = line
            && let Header::Date { date } = header.as_ref()
        {
            return Ok(Some(format_chat_date_iso(date)?));
        }
    }
    Ok(None)
}

/// Space-joined `@Options` flags for the root `Options` attribute.
/// Returns `None` when the file declares no options (so the attribute
/// is simply omitted, matching Java Chatter).
fn format_options_attribute<S: ValidationState>(file: &ChatFile<S>) -> Option<String> {
    if file.options.is_empty() {
        return None;
    }
    let mut out = String::new();
    for (i, flag) in file.options.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        out.push_str(ChatOptionFlag::as_str(flag));
    }
    Some(out)
}

/// Per-speaker metadata that lives outside `Participant` itself but
/// gets hoisted onto the `<participant>` element in XML.
#[derive(Default)]
struct SpeakerExtras {
    birthplace: Option<String>,
    first_language: Option<String>,
}

/// Scan all body-level headers for `@Birthplace of X` / `@L1 of X`
/// entries, keyed by the participant's `SpeakerCode`. The
/// `emit_participants` pass then hoists these onto the
/// `<participant>` element whose `id` matches.
fn collect_per_speaker_metadata<S: ValidationState>(
    file: &ChatFile<S>,
) -> HashMap<SpeakerCode, SpeakerExtras> {
    let mut out: HashMap<SpeakerCode, SpeakerExtras> = HashMap::new();
    for line in file.lines.iter() {
        let Line::Header { header, .. } = line else {
            continue;
        };
        match header.as_ref() {
            Header::Birthplace { participant, place } => {
                out.entry(participant.clone()).or_default().birthplace =
                    Some(place.as_str().to_owned());
            }
            Header::L1Of {
                participant,
                language,
            } => {
                out.entry(participant.clone()).or_default().first_language =
                    Some(language.as_str().to_owned());
            }
            _ => {}
        }
    }
    out
}

/// `ChatDate` → `YYYY-MM-DD`. Rejects unsupported dates up front so the
/// emitter never writes a malformed attribute.
fn format_chat_date_iso(date: &ChatDate) -> Result<String, XmlWriteError> {
    match date {
        ChatDate::Valid {
            day, month, year, ..
        } => Ok(format!(
            "{year:04}-{month:02}-{day:02}",
            year = year,
            month = month_to_number(month),
            day = day
        )),
        ChatDate::Unsupported(raw) => Err(XmlWriteError::MissingMetadata {
            what: format!("unparseable @Birth/@Date value: {raw}"),
        }),
    }
}

fn month_to_number(month: &Month) -> u8 {
    match month {
        Month::Jan => 1,
        Month::Feb => 2,
        Month::Mar => 3,
        Month::Apr => 4,
        Month::May => 5,
        Month::Jun => 6,
        Month::Jul => 7,
        Month::Aug => 8,
        Month::Sep => 9,
        Month::Oct => 10,
        Month::Nov => 11,
        Month::Dec => 12,
    }
}

/// `AgeValue` → ISO 8601 duration. Examples:
/// - `1;08.02` → `P1Y08M02D`
/// - `43;`    → `P43Y`
/// - `2;06`   → `P2Y06M`
///
/// Months and days are zero-padded to two digits to match Java Chatter's
/// output; years are unpadded.
fn format_age_iso8601(age: &AgeValue) -> Result<String, XmlWriteError> {
    match age {
        AgeValue::Valid {
            years,
            months,
            days,
            ..
        } => {
            let mut out = format!("P{years}Y");
            if let Some(m) = months {
                out.push_str(&format!("{m:02}M"));
            }
            if let Some(d) = days {
                out.push_str(&format!("{d:02}D"));
            }
            Ok(out)
        }
        AgeValue::Unsupported(raw) => Err(XmlWriteError::MissingMetadata {
            what: format!("unparseable @ID age value: {raw}"),
        }),
    }
}

/// Map [`Sex`] to its XML attribute value. Unsupported values escalate
/// rather than silently serializing the raw text — downstream consumers
/// expect exactly `male` or `female` in this slot.
fn sex_to_xml(sex: &Sex) -> Result<&'static str, XmlWriteError> {
    match sex {
        Sex::Male => Ok("male"),
        Sex::Female => Ok("female"),
        Sex::Unsupported(raw) => Err(XmlWriteError::MissingMetadata {
            what: format!("unsupported @ID sex value: {raw}"),
        }),
    }
}

/// Walk a [`BulletContent`] and concatenate its plain-text segments.
/// Bullet / picture / continuation segments are staged emission work
/// and report `FeatureNotImplemented` until their handler lands.
///
/// Exposed `pub(super)` so [`super::deptier`] can reuse the same
/// lowering for text-content dependent tiers — `%act`, `%com`,
/// `%sit`, and friends share the `BulletContent` shape with
/// `@Comment` headers.
pub(super) fn bullet_content_plain_text(content: &BulletContent) -> Result<String, XmlWriteError> {
    let mut out = String::new();
    for segment in content.segments.0.iter() {
        match segment {
            BulletContentSegment::Text(text) => out.push_str(&text.text),
            BulletContentSegment::Bullet(bullet) => {
                // `<comment>` is plain-text per XSD — the CHAT NAK
                // delimiter (U+0015) is a control char that XML 1.0
                // rejects outright. Emit the bullet payload as a
                // space-separated inline form (`[start_end]`) so the
                // information survives in a round-trip-friendly
                // representation without breaking the schema.
                if !out.is_empty() && !out.ends_with(' ') {
                    out.push(' ');
                }
                out.push('[');
                out.push_str(&bullet.start_ms.to_string());
                out.push('_');
                out.push_str(&bullet.end_ms.to_string());
                out.push(']');
            }
            BulletContentSegment::Picture(picture) => {
                // Same rationale as bullets — avoid the NAK
                // delimiter and render the picture reference
                // inline with `%pic:"filename"` minus the
                // surrounding control bytes.
                if !out.is_empty() && !out.ends_with(' ') {
                    out.push(' ');
                }
                out.push_str("%pic:\"");
                out.push_str(picture.filename.as_str());
                out.push('"');
            }
            BulletContentSegment::Continuation => {
                // Continuation = tab-indented wrapped line. In XML the
                // whole comment is a single logical string; the golden
                // output replaces the wrap boundary with a single space
                // rather than preserving the `\n\t` literally.
                if !out.ends_with(' ') {
                    out.push(' ');
                }
            }
        }
    }
    Ok(out)
}
