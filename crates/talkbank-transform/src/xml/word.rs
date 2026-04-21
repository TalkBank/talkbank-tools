//! Word-level emission — `<w>`, `<tagMarker>`, `<pause>`, `<t>`, and
//! the `<g>` wrappers for retraces and annotated words.
//!
//! Every function here writes the spoken-text side of a main-tier
//! chunk. `%mor` subtrees inside these elements are delegated to
//! `super::mor` via `emit_word_mor_subtree` so morphology logic stays
//! in one place.
//!
//! Staged features are reported via
//! [`XmlWriteError::FeatureNotImplemented`] with a descriptive
//! `feature:` string. This keeps the golden-XML harness producing
//! single-phenomenon failure diagnostics rather than swallowing
//! unimplemented constructs silently.

use quick_xml::events::{BytesEnd, BytesStart, Event};

use talkbank_model::model::{
    Action, Annotated, BracketedItem, CADelimiter, CADelimiterType, CAElement, CAElementType,
    ContentAnnotation, Event as CEvent, GrammaticalRelation, Linker, Mor, OverlapPointKind, Pause,
    PauseDuration, ReplacedWord, Retrace, RetraceKind, Separator, Terminator, Word, WordContent,
    WordStressMarkerType,
};

use super::error::XmlWriteError;
use super::mor::{UtteranceTiers, gra_entry};
use super::writer::{XmlEmitter, escape_text};

impl XmlEmitter {
    /// Emit `<w[ type="…"][ untranscribed="…"]>TEXT[<mor…/>]</w>` for
    /// a main-tier [`Word`]. Form-type suffixes (`@a`, `@b`, …),
    /// language markers (`@s`), and word-internal POS suffixes are
    /// staged — fail loud so the harness picks up the missing
    /// increments.
    pub(super) fn emit_word(
        &mut self,
        word: &Word,
        mor: Option<&Mor>,
        gra: Option<&[GrammaticalRelation]>,
        chunk_index_1based: usize,
    ) -> Result<(), XmlWriteError> {
        if word.form_type.is_some() || word.lang.is_some() || word.part_of_speech.is_some() {
            return Err(XmlWriteError::FeatureNotImplemented {
                feature: "word with form/language/POS marker".to_owned(),
            });
        }

        let mut start = BytesStart::new("w");
        if let Some(cat) = &word.category {
            // `CAOmission` (`(parens)`) has no `type=` attribute — it
            // renders as a whole-word shortening (see
            // `emit_word_contents`), not as a `type="omission"` word.
            if let Some(attr) = word_category_attr(cat) {
                start.push_attribute(("type", attr));
            }
        }
        // `word.untranscribed()` is case-insensitive — it intentionally
        // treats `XXX`/`YYY`/`WWW` the same as the lowercase forms to
        // keep downstream Stanza/MOR pipelines from producing spurious
        // entries on uppercase variants (see
        // `word_type.rs::compute_untranscribed`). The CHAT schema,
        // however, only attaches the `untranscribed` XML attribute to
        // the strictly-lowercase placeholders — `XXX` in source text
        // is plain content. Gate on the literal text here.
        if let Some(status) = untranscribed_attribute_for_xml(word) {
            start.push_attribute(("untranscribed", status));
        }

        self.writer.write_event(Event::Start(start))?;
        self.emit_word_contents(word)?;

        if let Some(mor) = mor {
            self.emit_word_mor_subtree(mor, gra, chunk_index_1based)?;
        }

        self.writer.write_event(Event::End(BytesEnd::new("w")))?;
        Ok(())
    }

    /// Walk the word's `content` vector and emit each segment inline
    /// inside the open `<w>`. Plain text becomes XML text; word-internal
    /// markers (compound `+`, clitic `~`, syllable pause `^`, lengthening
    /// `:`, stress `ˈ`/`ˌ`) become element siblings in the same order
    /// the CHAT source presents them.
    ///
    /// Mapping table (staged increments for the rest):
    ///
    /// | CHAT | Rust variant | XML |
    /// |------|--------------|-----|
    /// | text | [`WordContent::Text`] | (raw text) |
    /// | `+` (compound) | [`WordContent::CompoundMarker`] | `<wk type="cmp"/>` |
    /// | `~` (clitic) | [`WordContent::CliticBoundary`] | `<wk type="cli"/>` |
    /// | `^` (syllable pause) | [`WordContent::SyllablePause`] | `<p type="pause"/>` |
    /// | `:` (lengthening, count N) | [`WordContent::Lengthening`] | N × `<p type="drawl"/>` |
    /// | `ˈ` primary stress | [`WordContent::StressMarker`] (Primary) | `<ca-element type="primary stress"/>` |
    /// | `ˌ` secondary stress | [`WordContent::StressMarker`] (Secondary) | `<ca-element type="secondary stress"/>` |
    ///
    /// Other variants (`(text)` shortenings, CA delimiters/elements,
    /// overlap points, underline markers) fall through to
    /// `FeatureNotImplemented`. Each is a distinct TDD increment.
    fn emit_word_contents(&mut self, word: &Word) -> Result<(), XmlWriteError> {
        // CA delimiters come in pairs (`°…°`, `∆…∆`, …) but the AST
        // stores each `CADelimiter` occurrence on its own, without an
        // explicit begin/end marker. We recover begin/end by
        // toggling a bit per delimiter type during the walk: the
        // first occurrence of a given type sets the bit (`begin`),
        // the second clears it (`end`), and a doubled `°°` pair
        // opens and immediately closes again. The state is local to
        // one word — delimiters don't span word boundaries. A `u16`
        // bitset fits all 15 `CADelimiterType` variants without
        // allocating.
        let mut ca_delim_open: u16 = 0;

        // `CAOmission` (`(fullword)`) renders as a whole-word
        // shortening in the XML schema — `<w><shortening>text</shortening></w>`
        // rather than `<w type="omission">text</w>`. Open the wrapper
        // here and close it after the content loop so every emitted
        // segment lands inside `<shortening>`.
        let ca_omission = matches!(
            word.category,
            Some(talkbank_model::model::WordCategory::CAOmission)
        );
        if ca_omission {
            self.writer
                .write_event(Event::Start(BytesStart::new("shortening")))?;
        }

        // Leading OverlapPoint items were hoisted out as siblings
        // of `<w>` by `emit_leading_overlap_points`; skip them here
        // so they don't emit twice.
        let skip_count = word
            .content
            .iter()
            .take_while(|c| matches!(c, WordContent::OverlapPoint(_)))
            .count();

        for segment in word.content.iter().skip(skip_count) {
            match segment {
                WordContent::Text(text) => {
                    self.writer
                        .write_event(Event::Text(escape_text(text.0.as_str())))?;
                }
                WordContent::CompoundMarker(_) => {
                    let mut tag = BytesStart::new("wk");
                    tag.push_attribute(("type", "cmp"));
                    self.writer.write_event(Event::Empty(tag))?;
                }
                WordContent::CliticBoundary(_) => {
                    let mut tag = BytesStart::new("wk");
                    tag.push_attribute(("type", "cli"));
                    self.writer.write_event(Event::Empty(tag))?;
                }
                WordContent::SyllablePause(_) => {
                    let mut tag = BytesStart::new("p");
                    tag.push_attribute(("type", "pause"));
                    self.writer.write_event(Event::Empty(tag))?;
                }
                WordContent::Lengthening(lengthening) => {
                    // `no::` is `count = 2` → emit two `<p type="drawl"/>`
                    // siblings. `count` is a small positive u8 from the
                    // grammar; zero would be a parser bug.
                    for _ in 0..lengthening.count {
                        let mut tag = BytesStart::new("p");
                        tag.push_attribute(("type", "drawl"));
                        self.writer.write_event(Event::Empty(tag))?;
                    }
                }
                WordContent::StressMarker(marker) => {
                    let type_attr = match marker.marker_type {
                        WordStressMarkerType::Primary => "primary stress",
                        WordStressMarkerType::Secondary => "secondary stress",
                    };
                    let mut tag = BytesStart::new("ca-element");
                    tag.push_attribute(("type", type_attr));
                    self.writer.write_event(Event::Empty(tag))?;
                }
                WordContent::Shortening(shortening) => {
                    // `(text)` inside a word — a shortened/omitted sound
                    // segment that Java Chatter preserves as an inline
                    // `<shortening>text</shortening>` wrapper around the
                    // unspoken characters.
                    self.writer
                        .write_event(Event::Start(BytesStart::new("shortening")))?;
                    self.writer
                        .write_event(Event::Text(escape_text(shortening.0.as_str())))?;
                    self.writer
                        .write_event(Event::End(BytesEnd::new("shortening")))?;
                }
                WordContent::CAElement(element) => {
                    let mut tag = BytesStart::new("ca-element");
                    tag.push_attribute(("type", ca_element_label(element)));
                    self.writer.write_event(Event::Empty(tag))?;
                }
                WordContent::OverlapPoint(point) => {
                    self.emit_overlap_point(point)?;
                }
                WordContent::CADelimiter(delimiter) => {
                    let bit = 1u16 << ca_delimiter_bit_index(delimiter.delimiter_type);
                    let was_open = (ca_delim_open & bit) != 0;
                    ca_delim_open ^= bit;
                    let type_attr = if was_open { "end" } else { "begin" };
                    let mut tag = BytesStart::new("ca-delimiter");
                    tag.push_attribute(("type", type_attr));
                    tag.push_attribute(("label", ca_delimiter_label(delimiter)));
                    self.writer.write_event(Event::Empty(tag))?;
                }
                other => {
                    return Err(XmlWriteError::FeatureNotImplemented {
                        feature: format!("word content variant: {}", word_content_kind(other)),
                    });
                }
            }
        }

        if ca_omission {
            self.writer
                .write_event(Event::End(BytesEnd::new("shortening")))?;
        }
        Ok(())
    }

    /// Emit `<t type="p|q|e"/>` for an utterance terminator. When the
    /// paired `%mor` tier carries a terminator chunk, nests
    /// `<mor><mt/><gra/></mor>` inside the `<t>` to match Java
    /// Chatter's output shape. Staged terminators (trailing-off,
    /// interruption, …) fail loud.
    pub(super) fn emit_terminator(
        &mut self,
        terminator: &Terminator,
        tiers: &UtteranceTiers<'_>,
        chunk_index_1based: usize,
    ) -> Result<(), XmlWriteError> {
        // CA intonation-contour terminators (`⇗`, `↗`, `→`, `↘`,
        // `⇘`) were parsed by Rust as a single `Terminator` variant
        // but Java Chatter renders them as two sibling XML elements:
        // an `<s type="rising to high"/>` separator followed by the
        // `<t type="missing CA terminator"/>` placeholder. Peel off
        // the `<s>` here when the terminator carries pitch-contour
        // semantics — the `<t>` then collapses to the "missing"
        // placeholder via the same path as utterances with no
        // terminator at all.
        if let Some(s_label) = ca_terminator_separator_label(terminator) {
            let mut s_tag = BytesStart::new("s");
            s_tag.push_attribute(("type", s_label));
            self.writer.write_event(Event::Empty(s_tag))?;
        }

        // `<t type="…"/>`. Java Chatter uses the short letter code
        // for the three standard sentence terminators and a prose
        // phrase for CA-specific variants.
        let ty = match terminator {
            Terminator::Period { .. } => "p",
            Terminator::Question { .. } => "q",
            Terminator::Exclamation { .. } => "e",
            Terminator::TrailingOff { .. } => "trail off",
            Terminator::CaRisingToHigh { .. }
            | Terminator::CaRisingToMid { .. }
            | Terminator::CaLevel { .. }
            | Terminator::CaFallingToMid { .. }
            | Terminator::CaFallingToLow { .. } => "missing CA terminator",
            Terminator::CaNoBreak { .. } | Terminator::CaNoBreakLinker { .. } => {
                "no break TCU continuation"
            }
            Terminator::CaTechnicalBreak { .. } | Terminator::CaTechnicalBreakLinker { .. } => {
                "technical break TCU continuation"
            }
            other => {
                return Err(XmlWriteError::FeatureNotImplemented {
                    feature: format!("terminator variant: {}", terminator_kind(other)),
                });
            }
        };
        let mut start = BytesStart::new("t");
        start.push_attribute(("type", ty));

        // When %mor is present and carries a terminator chunk, Java
        // Chatter nests `<mor type="mor"><mt type="X"/><gra.../></mor>`
        // inside `<t>`, making it a non-empty element. Without %mor,
        // `<t>` stays empty (structural comparator folds `<t/>` vs
        // `<t></t>`).
        let has_mor_terminator = tiers
            .mor
            .as_ref()
            .and_then(|m| m.terminator.as_ref())
            .is_some();
        if has_mor_terminator {
            self.writer.write_event(Event::Start(start))?;
            let mut mor = BytesStart::new("mor");
            mor.push_attribute(("type", "mor"));
            self.writer.write_event(Event::Start(mor))?;
            let mut mt = BytesStart::new("mt");
            mt.push_attribute(("type", ty));
            self.writer.write_event(Event::Empty(mt))?;
            if let Some(rel) = gra_entry(tiers.gra, chunk_index_1based) {
                self.emit_gra(rel)?;
            }
            self.writer.write_event(Event::End(BytesEnd::new("mor")))?;
            self.writer.write_event(Event::End(BytesEnd::new("t")))?;
        } else {
            self.writer.write_event(Event::Empty(start))?;
        }
        Ok(())
    }

    /// Emit a main-tier separator token. Java Chatter uses two XML
    /// element shapes here depending on the separator kind:
    ///
    /// - `<tagMarker type="…"/>` for structural separators (`,`,
    ///   `;`, `:`, `„`, `‡`). The tag-marker variants that
    ///   participate in `%mor` alignment (Comma, Tag, Vocative) get
    ///   a nested `<mor>` subtree when tiers are present.
    /// - `<s type="…"/>` for CA intonation / uptake / unmarked-ending
    ///   separators. These are empty elements — no `%mor` alignment.
    ///
    /// `CaContinuation` ([^c]) is a staged increment — its schema
    /// shape differs from both of the above and it doesn't appear in
    /// the reference corpus yet.
    pub(super) fn emit_separator(
        &mut self,
        sep: &Separator,
        mor: Option<&Mor>,
        gra: Option<&[GrammaticalRelation]>,
        chunk_index_1based: usize,
    ) -> Result<(), XmlWriteError> {
        if let Some(s_type) = ca_intonation_separator_label(sep) {
            let mut tag = BytesStart::new("s");
            tag.push_attribute(("type", s_type));
            self.writer.write_event(Event::Empty(tag))?;
            return Ok(());
        }

        let tag_type = separator_tag_type(sep)?;
        let mut start = BytesStart::new("tagMarker");
        start.push_attribute(("type", tag_type));

        match mor {
            Some(mor) => {
                self.writer.write_event(Event::Start(start))?;
                self.emit_word_mor_subtree(mor, gra, chunk_index_1based)?;
                self.writer
                    .write_event(Event::End(BytesEnd::new("tagMarker")))?;
            }
            None => {
                self.writer.write_event(Event::Empty(start))?;
            }
        }
        Ok(())
    }

    /// Emit `<pause symbolic-length="simple|long|very long"/>`.
    /// Timed pauses are a separate TDD increment — they require a
    /// `<pause length="X" unit="s"/>` shape and careful unit handling.
    pub(super) fn emit_pause(&mut self, pause: &Pause) -> Result<(), XmlWriteError> {
        let length = match &pause.duration {
            PauseDuration::Short => "simple",
            PauseDuration::Medium => "long",
            PauseDuration::Long => "very long",
            PauseDuration::Timed(_) => {
                return Err(XmlWriteError::FeatureNotImplemented {
                    feature: "timed pause (numeric duration)".to_owned(),
                });
            }
        };
        let mut start = BytesStart::new("pause");
        start.push_attribute(("symbolic-length", length));
        self.writer.write_event(Event::Empty(start))?;
        Ok(())
    }

    /// Emit `<g>…<k type="retracing"/></g>` for a retrace. The inner
    /// content is walked with NO `%mor` alignment (retraced text is
    /// excluded from `%mor` by CHAT convention).
    pub(super) fn emit_retrace(&mut self, retrace: &Retrace) -> Result<(), XmlWriteError> {
        if !retrace.annotations.is_empty() {
            return Err(XmlWriteError::FeatureNotImplemented {
                feature: "retrace with attached annotations".to_owned(),
            });
        }
        self.writer
            .write_event(Event::Start(BytesStart::new("g")))?;
        for item in retrace.content.content.iter() {
            self.emit_bracketed_word_only(item)?;
        }
        let kind = retrace_kind_attr(retrace.kind);
        let mut k = BytesStart::new("k");
        k.push_attribute(("type", kind));
        self.writer.write_event(Event::Empty(k))?;
        self.writer.write_event(Event::End(BytesEnd::new("g")))?;
        Ok(())
    }

    /// Emit `<g><w>word[<mor>...]</w><error/></g>` for a word carrying
    /// a single `[*]` error annotation. Richer annotation kinds — `[=]`,
    /// `[+]`, `[!]`, overlap markers etc. — each need their own
    /// increment.
    pub(super) fn emit_annotated_word(
        &mut self,
        annotated: &Annotated<Word>,
        mor: Option<&Mor>,
        gra: Option<&[GrammaticalRelation]>,
        chunk_index_1based: usize,
    ) -> Result<(), XmlWriteError> {
        // Java Chatter wraps every annotated word in `<g>…</g>` and
        // emits one child per scoped annotation after the `<w>`. We
        // reject unknown annotation kinds up front so the harness
        // reports the missing feature precisely.
        let annotations = annotated.scoped_annotations.as_slice();
        if annotations.is_empty() {
            return Err(XmlWriteError::FeatureNotImplemented {
                feature: "annotated word with zero scoped annotations".to_owned(),
            });
        }

        self.writer
            .write_event(Event::Start(BytesStart::new("g")))?;
        self.emit_word(&annotated.inner, mor, gra, chunk_index_1based)?;

        for annotation in annotations {
            self.emit_scoped_annotation(annotation)?;
        }

        self.writer.write_event(Event::End(BytesEnd::new("g")))?;
        Ok(())
    }

    /// Emit the XML child element corresponding to a single
    /// scoped-annotation. Called by `emit_annotated_word` and
    /// (eventually) by replaced-word / retrace emitters that share
    /// the same annotation surface. Mapping from CHAT to XML:
    ///
    /// | CHAT | Rust variant | XML |
    /// |------|--------------|-----|
    /// | `[*]` (no code) | [`ContentAnnotation::Error`] | `<error/>` |
    /// | `[= text]`      | [`ContentAnnotation::Explanation`] | `<ga type="explanation">text</ga>` |
    /// | `[!]` | [`ContentAnnotation::Stressing`] | `<k type="stressing"/>` |
    ///
    /// All other scoped-annotation variants (`[!!]`, `[!*]`, `[?]`,
    /// `[=! text]`, `[+ text]`, `[% text]`, alternatives, overlaps,
    /// `[e]` exclude, …) fall through to `FeatureNotImplemented` —
    /// each is a separate TDD increment.
    fn emit_scoped_annotation(
        &mut self,
        annotation: &ContentAnnotation,
    ) -> Result<(), XmlWriteError> {
        match annotation {
            ContentAnnotation::Error(err) if err.code.is_none() => {
                self.writer
                    .write_event(Event::Empty(BytesStart::new("error")))?;
                Ok(())
            }
            ContentAnnotation::Explanation(expl) => {
                let mut tag = BytesStart::new("ga");
                tag.push_attribute(("type", "explanation"));
                self.writer.write_event(Event::Start(tag))?;
                self.writer
                    .write_event(Event::Text(escape_text(expl.text.as_str())))?;
                self.writer.write_event(Event::End(BytesEnd::new("ga")))?;
                Ok(())
            }
            ContentAnnotation::Stressing => {
                let mut tag = BytesStart::new("k");
                tag.push_attribute(("type", "stressing"));
                self.writer.write_event(Event::Empty(tag))?;
                Ok(())
            }
            other => Err(XmlWriteError::FeatureNotImplemented {
                feature: format!(
                    "scoped annotation variant: {}",
                    scoped_annotation_kind(other)
                ),
            }),
        }
    }

    /// Emit `<w>original<replacement><w>r1</w><w>r2</w>…</replacement></w>`.
    /// Each replacement word consumes one `%mor` / `%gra` chunk, so
    /// the caller tracks the cursor via the returned
    /// `chunks_consumed` value.
    ///
    /// `%mor` items align to the *replacement* words, not the
    /// original — that matches Java Chatter's output and the CHAT
    /// convention of aligning morphology to the intended form. A
    /// single-word replacement consumes one chunk; `dunno [: don't
    /// know]` consumes two.
    ///
    /// Returns the number of `%mor`/`%gra` chunks the replacement
    /// consumed. Callers in `emit_utterance` use this to advance
    /// their running cursor.
    ///
    /// Scoped annotations attached to the replaced word are a
    /// staged increment — `[: text] [= explanation]` and similar
    /// patterns require emitting both a `<replacement>` and
    /// annotation siblings inside a `<g>` wrapper, which isn't
    /// wired yet.
    pub(super) fn emit_replaced_word(
        &mut self,
        rw: &ReplacedWord,
        tiers: &UtteranceTiers<'_>,
        starting_mor_cursor: usize,
        starting_gra_chunk: usize,
    ) -> Result<(usize, usize), XmlWriteError> {
        // `полетел [: полетела] [*]` — replacement + error — renders
        // as `<g><w>…<replacement>…</replacement></w><error/></g>`.
        // When scoped annotations are present, wrap the whole
        // replaced-word shape in `<g>` and emit each annotation as
        // a sibling after the outer `<w>` closes.
        let wrap_in_g = !rw.scoped_annotations.is_empty();
        if wrap_in_g {
            self.writer
                .write_event(Event::Start(BytesStart::new("g")))?;
        }

        // Outer <w> with the original spoken text, no mor subtree.
        // category / untranscribed on the original carry through so
        // `0word [: replacement]` still emits `type="omission"`.
        // (CAOmission intentionally omits the attribute — see
        // `word_category_attr`.)
        let mut outer = BytesStart::new("w");
        if let Some(cat) = &rw.word.category
            && let Some(attr) = word_category_attr(cat)
        {
            outer.push_attribute(("type", attr));
        }
        self.writer.write_event(Event::Start(outer))?;
        self.writer
            .write_event(Event::Text(escape_text(rw.word.cleaned_text())))?;

        self.writer
            .write_event(Event::Start(BytesStart::new("replacement")))?;
        let replacement_words = rw.replacement.words.0.as_slice();
        let mut mor_cursor = starting_mor_cursor;
        let mut gra_chunk = starting_gra_chunk;
        for replacement_word in replacement_words.iter() {
            let mor_for_word = tiers
                .mor
                .as_ref()
                .and_then(|mor| mor.items.0.get(mor_cursor));
            self.emit_word(replacement_word, mor_for_word, tiers.gra, gra_chunk)?;
            // Each replacement word consumes one Mor item (with its
            // post-clitics inline) plus `1 + post_clitics.len()`
            // `%gra` edges. Track both cursors so a replacement like
            // `dunno [: don't know's]` (if that ever appears) stays
            // aligned even if one expansion carries a clitic chain.
            let post_count = mor_for_word.map(|m| m.post_clitics.len()).unwrap_or(0);
            mor_cursor += 1;
            gra_chunk += 1 + post_count;
        }
        self.writer
            .write_event(Event::End(BytesEnd::new("replacement")))?;

        self.writer.write_event(Event::End(BytesEnd::new("w")))?;

        if wrap_in_g {
            for annotation in rw.scoped_annotations.iter() {
                self.emit_scoped_annotation(annotation)?;
            }
            self.writer.write_event(Event::End(BytesEnd::new("g")))?;
        }

        // When no %mor tier is present, we consume zero chunks on
        // either axis — there's nothing for the caller to advance
        // past.
        if tiers.mor.is_some() {
            Ok((
                mor_cursor - starting_mor_cursor,
                gra_chunk - starting_gra_chunk,
            ))
        } else {
            Ok((0, 0))
        }
    }

    /// Emit an inline `&=descriptor` event as `<e><happening>text</happening></e>`.
    /// `&=laughs`, `&=rire`, `&=coughs` and similar non-speech event
    /// markers sit in main-tier content outside the word alignment —
    /// they never consume `%mor` chunks.
    pub(super) fn emit_event(&mut self, event: &CEvent) -> Result<(), XmlWriteError> {
        self.writer
            .write_event(Event::Start(BytesStart::new("e")))?;
        self.writer
            .write_event(Event::Start(BytesStart::new("happening")))?;
        self.writer
            .write_event(Event::Text(escape_text(event.event_type.as_str())))?;
        self.writer
            .write_event(Event::End(BytesEnd::new("happening")))?;
        self.writer.write_event(Event::End(BytesEnd::new("e")))?;
        Ok(())
    }

    /// Emit an annotated bare-action token as `<e><action/></e>`. The
    /// `Annotated<Action>` wrapper carries scoped annotations in
    /// principle; currently only the empty-annotations case is wired
    /// because that is the only shape exercised by the reference
    /// corpus. Richer cases (`0 [= description]`) fail loud.
    pub(super) fn emit_annotated_action(
        &mut self,
        annotated: &Annotated<Action>,
    ) -> Result<(), XmlWriteError> {
        if !annotated.scoped_annotations.is_empty() {
            return Err(XmlWriteError::FeatureNotImplemented {
                feature: "annotated action with scoped annotations".to_owned(),
            });
        }
        self.writer
            .write_event(Event::Start(BytesStart::new("e")))?;
        self.writer
            .write_event(Event::Empty(BytesStart::new("action")))?;
        self.writer.write_event(Event::End(BytesEnd::new("e")))?;
        Ok(())
    }

    /// Emit `<linker type="…"/>` for a discourse linker (`+<`, `++`,
    /// `+≈`, `+≋`, …). Called once per item in
    /// `utterance.main.content.linkers` before any main-tier word is
    /// written. Staged variants (`+<` lazy-overlap, `++` completion)
    /// fail loud so each missing mapping shows up in the harness.
    pub(super) fn emit_linker(&mut self, linker: &Linker) -> Result<(), XmlWriteError> {
        let ty = match linker {
            Linker::NoBreakTcuContinuation => "no break TCU completion",
            Linker::TcuContinuation => "technical break TCU completion",
            other => {
                return Err(XmlWriteError::FeatureNotImplemented {
                    feature: format!("linker variant: {other:?}"),
                });
            }
        };
        let mut tag = BytesStart::new("linker");
        tag.push_attribute(("type", ty));
        self.writer.write_event(Event::Empty(tag))?;
        Ok(())
    }

    /// Hoist leading `OverlapPoint` items out of a word's content
    /// as top-level `<overlap-point/>` siblings. The Rust parser
    /// bundles markers at the start of a word (e.g. `⌈` in
    /// `⌈°overlapping+soft⌉°`) into `word.content`; Java Chatter
    /// keeps the *leading* ones outside the `<w>` element — only
    /// internal / trailing overlap points remain inside `<w>`. This
    /// method emits the leading prefix only; `emit_word_contents`
    /// knows to skip them when walking the word body.
    pub(super) fn emit_leading_overlap_points(&mut self, word: &Word) -> Result<(), XmlWriteError> {
        for item in word.content.iter() {
            match item {
                WordContent::OverlapPoint(point) => {
                    self.emit_overlap_point(point)?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    /// Emit `<overlap-point start-end=… top-bottom=… [index=…]/>`.
    /// Shared between three callers: `emit_utterance` for top-level
    /// overlap markers (siblings of `<w>`), `emit_leading_overlap_points`
    /// for hoisted leading markers, and `emit_word_contents` for
    /// word-internal trailer markers. The XML shape is identical in
    /// all three contexts — only the enclosing element differs.
    /// Concurrent overlap regions carry an `index` attribute so
    /// downstream consumers can pair up begin/end markers; absent when
    /// the file uses only one overlap pair at a time.
    pub(super) fn emit_overlap_point(
        &mut self,
        point: &talkbank_model::model::OverlapPoint,
    ) -> Result<(), XmlWriteError> {
        let (start_end, top_bottom) = match point.kind {
            OverlapPointKind::TopOverlapBegin => ("start", "top"),
            OverlapPointKind::TopOverlapEnd => ("end", "top"),
            OverlapPointKind::BottomOverlapBegin => ("start", "bottom"),
            OverlapPointKind::BottomOverlapEnd => ("end", "bottom"),
        };
        let index_str = point.index.as_ref().map(|i| i.to_string());
        let mut tag = BytesStart::new("overlap-point");
        if let Some(s) = index_str.as_deref() {
            tag.push_attribute(("index", s));
        }
        tag.push_attribute(("start-end", start_end));
        tag.push_attribute(("top-bottom", top_bottom));
        self.writer.write_event(Event::Empty(tag))?;
        Ok(())
    }

    /// Emit `<g><w>word1</w>…<annotation/></g>` for a group wrapped
    /// in scoped annotations — the main-tier shape behind
    /// `<lá em casa> [!]`. Each inner `BracketedItem::Word`
    /// consumes one `%mor` item (plus its post-clitic chain) from
    /// the caller-supplied cursors; the advance counts are
    /// returned so `emit_utterance` can thread the cursors forward.
    ///
    /// Staged increments: bracketed items beyond `Word` (Event,
    /// Pause, Separator, AnnotatedWord inside a group) fail loud.
    /// The reference corpus currently only exercises the
    /// `<word word word>` shape inside `AnnotatedGroup`.
    pub(super) fn emit_annotated_group(
        &mut self,
        annotated: &talkbank_model::model::Annotated<talkbank_model::model::Group>,
        tiers: &UtteranceTiers<'_>,
        starting_mor_cursor: usize,
        starting_gra_chunk: usize,
    ) -> Result<(usize, usize), XmlWriteError> {
        let annotations = annotated.scoped_annotations.as_slice();
        if annotations.is_empty() {
            return Err(XmlWriteError::FeatureNotImplemented {
                feature: "annotated group with zero scoped annotations".to_owned(),
            });
        }

        self.writer
            .write_event(Event::Start(BytesStart::new("g")))?;

        let mut mor_cursor = starting_mor_cursor;
        let mut gra_chunk = starting_gra_chunk;
        for item in annotated.inner.content.content.iter() {
            match item {
                BracketedItem::Word(word) => {
                    let mor_for_word = tiers
                        .mor
                        .as_ref()
                        .and_then(|mor| mor.items.0.get(mor_cursor));
                    self.emit_word(word, mor_for_word, tiers.gra, gra_chunk)?;
                    if tiers.mor.is_some() {
                        let post_count = mor_for_word.map(|m| m.post_clitics.len()).unwrap_or(0);
                        mor_cursor += 1;
                        gra_chunk += 1 + post_count;
                    }
                }
                other => {
                    return Err(XmlWriteError::FeatureNotImplemented {
                        feature: format!("group content item: {other:?}"),
                    });
                }
            }
        }

        for annotation in annotations {
            self.emit_scoped_annotation(annotation)?;
        }

        self.writer.write_event(Event::End(BytesEnd::new("g")))?;

        if tiers.mor.is_some() {
            Ok((
                mor_cursor - starting_mor_cursor,
                gra_chunk - starting_gra_chunk,
            ))
        } else {
            Ok((0, 0))
        }
    }

    /// Emit a single bracketed-content item that must be a bare word.
    /// Retrace payloads in the current goldens are uniformly bare
    /// words; richer content (nested retraces, separators inside a
    /// retrace, etc.) is staged as a future increment.
    fn emit_bracketed_word_only(&mut self, item: &BracketedItem) -> Result<(), XmlWriteError> {
        match item {
            BracketedItem::Word(word) => self.emit_word(word, None, None, 0),
            other => Err(XmlWriteError::FeatureNotImplemented {
                feature: format!("retrace content item: {other:?}"),
            }),
        }
    }
}

/// Map [`Separator`] variants to the `<tagMarker type="...">` token
/// used by Java Chatter. CA intonation / uptake separators are
/// caught earlier in `emit_separator` and rendered as `<s type="…"/>`
/// so they never reach this helper.
fn separator_tag_type(sep: &Separator) -> Result<&'static str, XmlWriteError> {
    Ok(match sep {
        Separator::Comma { .. } => "comma",
        Separator::Semicolon { .. } => "semicolon",
        Separator::Colon { .. } => "colon",
        Separator::Tag { .. } => "tag",
        Separator::Vocative { .. } => "vocative",
        Separator::CaContinuation { .. } => {
            return Err(XmlWriteError::FeatureNotImplemented {
                feature: "CA continuation separator ([^c])".to_owned(),
            });
        }
        _ => {
            return Err(XmlWriteError::FeatureNotImplemented {
                feature: "separator variant (non-standard)".to_owned(),
            });
        }
    })
}

/// Recognise CA intonation-contour terminators that Java Chatter
/// splits into a preceding `<s type="…"/>` plus `<t type="missing CA
/// terminator"/>`. Returns the `<s>` label for pitch-contour
/// terminators (`CaRisingToHigh`, `CaFallingToLow`, …). Other
/// terminator variants render as a single `<t>`.
fn ca_terminator_separator_label(terminator: &Terminator) -> Option<&'static str> {
    Some(match terminator {
        Terminator::CaRisingToHigh { .. } => "rising to high",
        Terminator::CaRisingToMid { .. } => "rising to mid",
        Terminator::CaLevel { .. } => "level",
        Terminator::CaFallingToMid { .. } => "falling to mid",
        Terminator::CaFallingToLow { .. } => "falling to low",
        _ => return None,
    })
}

/// Recognise CA intonation / uptake / unmarked-ending separators that
/// Java Chatter renders as an empty `<s type="…"/>` element rather
/// than `<tagMarker>`. Returns `None` for structural separators
/// (handled via `separator_tag_type`).
fn ca_intonation_separator_label(sep: &Separator) -> Option<&'static str> {
    Some(match sep {
        Separator::RisingToHigh { .. } => "rising to high",
        Separator::RisingToMid { .. } => "rising to mid",
        Separator::Level { .. } => "level",
        Separator::FallingToMid { .. } => "falling to mid",
        Separator::FallingToLow { .. } => "falling to low",
        Separator::UnmarkedEnding { .. } => "unmarked ending",
        Separator::Uptake { .. } => "uptake",
        _ => return None,
    })
}

/// Map [`talkbank_model::model::WordCategory`] to the `<w type="...">`
/// attribute value used by Java Chatter. Returns `None` for
/// [`WordCategory::CAOmission`] — `(parens)` is not an omission in
/// the schema sense; it renders as an all-content `<shortening>`
/// wrapper instead. See `emit_word_contents`.
fn word_category_attr(cat: &talkbank_model::model::WordCategory) -> Option<&'static str> {
    use talkbank_model::model::WordCategory;
    match cat {
        WordCategory::Omission => Some("omission"),
        WordCategory::CAOmission => None,
        WordCategory::Nonword => Some("nonword"),
        WordCategory::Filler => Some("filler"),
        WordCategory::PhonologicalFragment => Some("fragment"),
    }
}

/// Map [`talkbank_model::model::content::word::UntranscribedStatus`]
/// to the `<w untranscribed="...">` attribute value.
fn untranscribed_attr(
    status: talkbank_model::model::content::word::UntranscribedStatus,
) -> &'static str {
    use talkbank_model::model::content::word::UntranscribedStatus;
    match status {
        UntranscribedStatus::Unintelligible => "unintelligible",
        UntranscribedStatus::Phonetic => "untranscribed",
        UntranscribedStatus::Untranscribed => "not-transcribable",
    }
}

/// CHAT-spec-strict check for the `untranscribed` XML attribute: only
/// the lowercase placeholders `xxx`, `yyy`, `www` trigger it. Bypasses
/// the model's case-insensitive `untranscribed()` helper (which is a
/// Stanza/MOR workaround, not the XML schema rule).
fn untranscribed_attribute_for_xml(word: &Word) -> Option<&'static str> {
    use talkbank_model::model::content::word::UntranscribedStatus;
    match word.cleaned_text() {
        "xxx" => Some(untranscribed_attr(UntranscribedStatus::Unintelligible)),
        "yyy" => Some(untranscribed_attr(UntranscribedStatus::Phonetic)),
        "www" => Some(untranscribed_attr(UntranscribedStatus::Untranscribed)),
        _ => None,
    }
}

/// Map a [`RetraceKind`] to the `<k type="...">` token Java Chatter
/// emits inside `<g>…</g>`.
fn retrace_kind_attr(kind: RetraceKind) -> &'static str {
    match kind {
        RetraceKind::Partial => "retracing",
        RetraceKind::Full => "retracing with correction",
        RetraceKind::Multiple => "retracing with multiple corrections",
        RetraceKind::Reformulation => "reformulation",
        RetraceKind::Uncertain => "uncertain retracing",
    }
}

/// Short display name for a [`ContentAnnotation`] variant. Used
/// inside `FeatureNotImplemented` diagnostics so the harness surfaces
/// each staged annotation kind as a distinct increment.
/// Map a [`CAElementType`] to the `<ca-element type="…"/>` attribute
/// value used by Java Chatter. Values are lowercase with a space
/// separator where the Rust enum uses CamelCase (e.g. `PitchUp` →
/// `"pitch up"`, `BlockedSegments` → `"blocked segments"`).
fn ca_element_label(element: &CAElement) -> &'static str {
    match element.element_type {
        CAElementType::BlockedSegments => "blocked segments",
        CAElementType::Constriction => "constriction",
        CAElementType::Hardening => "hardening",
        CAElementType::HurriedStart => "hurried start",
        CAElementType::Inhalation => "inhalation",
        CAElementType::LaughInWord => "laugh in word",
        CAElementType::PitchDown => "pitch down",
        CAElementType::PitchReset => "pitch reset",
        CAElementType::PitchUp => "pitch up",
        CAElementType::SuddenStop => "sudden stop",
    }
}

/// Assign a unique `[0, 15)` bit index to each [`CADelimiterType`]
/// variant so `emit_word_contents` can track begin/end state in a
/// `u16` bitset instead of a per-word `HashMap` allocation.
fn ca_delimiter_bit_index(ty: CADelimiterType) -> u8 {
    match ty {
        CADelimiterType::Faster => 0,
        CADelimiterType::Slower => 1,
        CADelimiterType::Softer => 2,
        CADelimiterType::Louder => 3,
        CADelimiterType::LowPitch => 4,
        CADelimiterType::HighPitch => 5,
        CADelimiterType::SmileVoice => 6,
        CADelimiterType::BreathyVoice => 7,
        CADelimiterType::Unsure => 8,
        CADelimiterType::Whisper => 9,
        CADelimiterType::Yawn => 10,
        CADelimiterType::Singing => 11,
        CADelimiterType::SegmentRepetition => 12,
        CADelimiterType::Creaky => 13,
        CADelimiterType::Precise => 14,
    }
}

/// Map a [`CADelimiter`] to the `<ca-delimiter label="…"/>` attribute
/// value Java Chatter emits. Labels are not a mechanical
/// transformation of the variant name — the pitch pair uses hyphens
/// (`"low-pitch"`), most two-word variants use spaces
/// (`"smile voice"`, `"breathy voice"`), and `SegmentRepetition`
/// renames to `"repeated-segment"`. Kept as an explicit table so the
/// mapping is visible.
fn ca_delimiter_label(delimiter: &CADelimiter) -> &'static str {
    match delimiter.delimiter_type {
        CADelimiterType::Faster => "faster",
        CADelimiterType::Slower => "slower",
        CADelimiterType::Softer => "softer",
        CADelimiterType::Louder => "louder",
        CADelimiterType::LowPitch => "low-pitch",
        CADelimiterType::HighPitch => "high-pitch",
        CADelimiterType::SmileVoice => "smile voice",
        CADelimiterType::BreathyVoice => "breathy voice",
        CADelimiterType::Unsure => "unsure",
        CADelimiterType::Whisper => "whisper",
        CADelimiterType::Yawn => "yawn",
        CADelimiterType::Singing => "singing",
        CADelimiterType::SegmentRepetition => "repeated-segment",
        CADelimiterType::Creaky => "creaky",
        CADelimiterType::Precise => "precise",
    }
}

/// Short display name for a [`WordContent`] variant, used in
/// `FeatureNotImplemented` diagnostics so the harness surfaces each
/// staged word-internal marker as a distinct increment.
fn word_content_kind(c: &WordContent) -> &'static str {
    match c {
        WordContent::Text(_) => "Text",
        WordContent::Shortening(_) => "Shortening",
        WordContent::OverlapPoint(_) => "OverlapPoint",
        WordContent::CAElement(_) => "CAElement",
        WordContent::CADelimiter(_) => "CADelimiter",
        WordContent::StressMarker(_) => "StressMarker",
        WordContent::Lengthening(_) => "Lengthening",
        WordContent::SyllablePause(_) => "SyllablePause",
        WordContent::UnderlineBegin(_) => "UnderlineBegin",
        WordContent::UnderlineEnd(_) => "UnderlineEnd",
        WordContent::CompoundMarker(_) => "CompoundMarker",
        WordContent::CliticBoundary(_) => "CliticBoundary",
    }
}

fn scoped_annotation_kind(annotation: &ContentAnnotation) -> &'static str {
    match annotation {
        ContentAnnotation::Error(_) => "Error",
        ContentAnnotation::Explanation(_) => "Explanation",
        ContentAnnotation::Addition(_) => "Addition",
        ContentAnnotation::OverlapBegin(_) => "OverlapBegin",
        ContentAnnotation::OverlapEnd(_) => "OverlapEnd",
        ContentAnnotation::CaContinuation => "CaContinuation",
        ContentAnnotation::Stressing => "Stressing",
        ContentAnnotation::ContrastiveStressing => "ContrastiveStressing",
        ContentAnnotation::BestGuess => "BestGuess",
        ContentAnnotation::Uncertain => "Uncertain",
        ContentAnnotation::Paralinguistic(_) => "Paralinguistic",
        ContentAnnotation::Alternative(_) => "Alternative",
        ContentAnnotation::PercentComment(_) => "PercentComment",
        ContentAnnotation::Duration(_) => "Duration",
        ContentAnnotation::Exclude => "Exclude",
        ContentAnnotation::Unknown(_) => "Unknown",
    }
}

/// Short display name for a [`Terminator`] variant, used only in
/// `FeatureNotImplemented` diagnostics.
fn terminator_kind(t: &Terminator) -> &'static str {
    match t {
        Terminator::Period { .. } => "Period",
        Terminator::Question { .. } => "Question",
        Terminator::Exclamation { .. } => "Exclamation",
        Terminator::TrailingOff { .. } => "TrailingOff",
        Terminator::Interruption { .. } => "Interruption",
        Terminator::SelfInterruption { .. } => "SelfInterruption",
        Terminator::InterruptedQuestion { .. } => "InterruptedQuestion",
        _ => "(other terminator)",
    }
}
