//! Handwritten recursive-descent parser — translated from grammar.js.
//!
//! Entry points:
//! - `parse_main_tier(input)` — parse a single main tier line
//! - `parse_chat_file(input)` — parse a complete CHAT file (TODO)

use crate::ast::*;
use crate::lexer::{Lexer, LexerSpan};
use crate::token::{Token, TokenDiscriminants};
use talkbank_model::{ErrorSink, NullErrorSink, ParseError, Span};

/// Parser state.
pub struct Parser<'a, E: ErrorSink = NullErrorSink> {
    source: &'a str,
    tokens: Vec<(Token<'a>, LexerSpan)>,
    pos: usize,
    errors: E,
}

impl<'a> Parser<'a, NullErrorSink> {
    /// Create a parser from source text with no error reporting.
    pub fn new(source: &'a str, start_condition: usize) -> Self {
        Self::with_errors(source, start_condition, NullErrorSink)
    }
}

impl<'a, E: ErrorSink> Parser<'a, E> {
    /// Create a parser from source text, reporting errors to the given sink.
    pub fn with_errors(source: &'a str, start_condition: usize, errors: E) -> Self {
        let mut padded = source.to_string();
        padded.push('\0');
        let padded: &'a str = Box::leak(padded.into_boxed_str());
        let mut lexer = Lexer::new(padded, start_condition);
        let mut tokens = Vec::new();
        while let Some(t) = lexer.next() {
            tokens.push(t);
        }
        Self {
            source,
            tokens,
            pos: 0,
            errors,
        }
    }

    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek(&self) -> Option<&Token<'a>> {
        self.tokens.get(self.pos).map(|(t, _)| t)
    }

    fn peek_d(&self) -> Option<TokenDiscriminants> {
        self.peek().map(TokenDiscriminants::from)
    }

    fn advance(&mut self) -> Option<(Token<'a>, LexerSpan)> {
        if self.at_end() {
            None
        } else {
            let item = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(item)
        }
    }

    fn expect(&mut self, d: TokenDiscriminants) -> Option<(Token<'a>, LexerSpan)> {
        if self.peek_d() == Some(d) {
            self.advance()
        } else {
            None
        }
    }

    fn skip_ws(&mut self) {
        while matches!(
            self.peek_d(),
            Some(TokenDiscriminants::Whitespace | TokenDiscriminants::Continuation)
        ) {
            self.advance();
        }
    }

    // ═══════════════════════════════════════════════════════════
    // grammar.js: main_tier = seq(star, speaker, colon, tab, tier_body)
    // ═══════════════════════════════════════════════════════════

    pub fn parse_main_tier(&mut self) -> Option<MainTier<'a>> {
        self.expect(TokenDiscriminants::Star)?;
        let (speaker, _) = self.expect(TokenDiscriminants::Speaker)?;
        self.expect(TokenDiscriminants::TierSep)?;
        let tier_body = self.parse_tier_body();
        Some(MainTier { speaker, tier_body })
    }

    // ═══════════════════════════════════════════════════════════
    // grammar.js: tier_body = seq(
    //   optional(linkers),
    //   optional(seq(langcode, whitespaces)),
    //   contents,
    //   utterance_end
    // )
    // ═══════════════════════════════════════════════════════════

    fn parse_tier_body(&mut self) -> TierBody<'a> {
        let linkers = self.parse_linkers();

        let langcode = if self.peek_d() == Some(TokenDiscriminants::Langcode) {
            let (tok, _) = self.advance().unwrap();
            self.skip_ws();
            Some(tok)
        } else {
            None
        };

        let contents = self.parse_contents();

        // utterance_end: optional(terminator), optional(final_codes), optional(media_url), newline
        let terminator = self.parse_optional_terminator();
        let postcodes = self.parse_postcodes();

        self.skip_ws();
        let media_bullet = if self.peek_d() == Some(TokenDiscriminants::MediaBullet) {
            Some(self.advance().unwrap().0)
        } else {
            None
        };

        self.skip_ws();
        // Consume newline if present
        self.expect(TokenDiscriminants::Newline);

        TierBody {
            linkers,
            langcode,
            contents,
            terminator,
            postcodes,
            media_bullet,
        }
    }

    // grammar.js: linkers = repeat1(seq(linker, whitespaces))
    fn parse_linkers(&mut self) -> Vec<Token<'a>> {
        let mut linkers = Vec::new();
        while is_linker(self.peek_d()) {
            linkers.push(self.advance().unwrap().0);
            self.skip_ws();
        }
        linkers
    }

    // ═══════════════════════════════════════════════════════════
    // grammar.js: contents = repeat1(choice(
    //   whitespaces, content_item, separator, overlap_point
    // ))
    // ═══════════════════════════════════════════════════════════

    pub fn parse_contents(&mut self) -> Vec<ContentItem<'a>> {
        self.parse_contents_until(None)
    }

    fn parse_contents_until(
        &mut self,
        closing: Option<TokenDiscriminants>,
    ) -> Vec<ContentItem<'a>> {
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.at_end() {
                break;
            }
            let d = match self.peek_d() {
                Some(d) => d,
                None => break,
            };

            // Stop conditions
            if is_terminator(Some(d))
                || d == TokenDiscriminants::Postcode
                || d == TokenDiscriminants::MediaBullet
                || d == TokenDiscriminants::Newline
            {
                break;
            }
            if let Some(c) = closing {
                if d == c {
                    break;
                }
            }

            match d {
                // overlap_point
                TokenDiscriminants::OverlapTopBegin
                | TokenDiscriminants::OverlapTopEnd
                | TokenDiscriminants::OverlapBottomBegin
                | TokenDiscriminants::OverlapBottomEnd => {
                    items.push(ContentItem::OverlapPoint(self.advance().unwrap().0));
                }

                // separator
                _ if is_separator(Some(d)) => {
                    items.push(ContentItem::Separator(self.advance().unwrap().0));
                }

                // pause
                _ if is_pause(d) => {
                    items.push(ContentItem::Pause(self.advance().unwrap().0));
                }

                // annotation (atomic brackets)
                _ if is_annotation(Some(d)) => {
                    items.push(ContentItem::Annotation(self.advance().unwrap().0));
                }

                // group: < contents > annotations
                TokenDiscriminants::LessThan => {
                    items.push(self.parse_group());
                }

                // quotation: " contents "
                TokenDiscriminants::LeftDoubleQuote => {
                    items.push(self.parse_quotation());
                }

                // other_spoken_event: &*SPK:word
                TokenDiscriminants::OtherSpokenEvent => {
                    items.push(ContentItem::OtherSpokenEvent(self.advance().unwrap().0));
                }

                // event: &= segment
                TokenDiscriminants::EventMarker => {
                    items.push(self.parse_event());
                }

                // CA markers are word-internal tokens — handled by is_word_start/is_word_token.
                // They get parsed as word body content (WordBodyItem::CaElement/CaDelimiter).
                // Standalone CA markers (not adjacent to text) become single-content words.

                // Freecodes [^ text]
                TokenDiscriminants::Freecode => {
                    items.push(ContentItem::Annotation(self.advance().unwrap().0));
                }

                // Underline markers (content-level, not inside words)
                TokenDiscriminants::UnderlineBegin => {
                    items.push(ContentItem::UnderlineBegin(self.advance().unwrap().0));
                }
                TokenDiscriminants::UnderlineEnd => {
                    items.push(ContentItem::UnderlineEnd(self.advance().unwrap().0));
                }

                // Standalone zero (0) → action, vs 0word → omission word
                TokenDiscriminants::Zero => {
                    // Peek ahead: if next token (no whitespace skip) is a word body
                    // token, this is an omission word (0word). Otherwise standalone action.
                    let next_d = if self.pos + 1 < self.tokens.len() {
                        Some(TokenDiscriminants::from(&self.tokens[self.pos + 1].0))
                    } else {
                        None
                    };
                    let is_prefix = matches!(
                        next_d,
                        Some(
                            TokenDiscriminants::WordSegment
                                | TokenDiscriminants::Shortening
                                | TokenDiscriminants::StressPrimary
                                | TokenDiscriminants::StressSecondary
                        )
                    );
                    if is_prefix {
                        // 0word — parse as a normal word with Zero prefix
                        items.push(self.parse_word_with_annotations());
                    } else {
                        // Standalone 0 — action
                        let zero = self.advance().unwrap().0;
                        let mut annotations = Vec::new();
                        loop {
                            let saved = self.pos;
                            self.skip_ws();
                            if is_annotation(self.peek_d()) {
                                let tok = self.advance().unwrap().0;
                                annotations.push(token_to_parsed_annotation(tok));
                            } else {
                                self.pos = saved;
                                break;
                            }
                        }
                        items.push(ContentItem::Action { zero, annotations });
                    }
                }

                // Long feature markers
                TokenDiscriminants::LongFeatureBegin => {
                    items.push(ContentItem::LongFeatureBegin(self.advance().unwrap().0));
                }
                TokenDiscriminants::LongFeatureEnd => {
                    items.push(ContentItem::LongFeatureEnd(self.advance().unwrap().0));
                }
                // Nonvocal markers
                TokenDiscriminants::NonvocalBegin => {
                    items.push(ContentItem::NonvocalBegin(self.advance().unwrap().0));
                }
                TokenDiscriminants::NonvocalEnd => {
                    items.push(ContentItem::NonvocalEnd(self.advance().unwrap().0));
                }
                TokenDiscriminants::NonvocalSimple => {
                    items.push(ContentItem::NonvocalSimple(self.advance().unwrap().0));
                }

                // Phonological group: ‹ contents ›
                TokenDiscriminants::PhoGroupBegin => {
                    items.push(self.parse_pho_group());
                }

                // Sign group: 〔 contents 〕
                TokenDiscriminants::SinGroupBegin => {
                    items.push(self.parse_sin_group());
                }

                // Rich Word token from lexer
                TokenDiscriminants::Word => {
                    items.push(self.parse_rich_word_with_annotations());
                }

                // Legacy word sub-tokens (fallback for body re-lexing)
                _ if is_word_start(d) => {
                    items.push(self.parse_word_with_annotations());
                }

                // Every token type must be explicitly handled above.
                // Unhandled tokens are reported to the error sink, never silently dropped.
                _ => {
                    let (tok, span) = self.advance().unwrap();
                    self.errors.report(ParseError::new(
                        talkbank_model::errors::codes::ErrorCode::UnexpectedSyntax,
                        talkbank_model::Severity::Warning,
                        talkbank_model::SourceLocation::new(Span::new(
                            span.start as u32,
                            span.end as u32,
                        )),
                        None,
                        format!("unhandled token in content: {:?}", tok.text()),
                    ));
                }
            }
        }
        items
    }

    // ═══════════════════════════════════════════════════════════
    // grammar.js: word_with_optional_annotations =
    //   seq(standalone_word, repeat(annotation))
    //
    // standalone_word = prec.right(6, seq(
    //   optional(choice(word_prefix, zero)),
    //   word_body,
    //   optional(form_marker),
    //   optional(word_lang_suffix),
    //   optional(pos_tag),
    // ))
    //
    // word_body starts with word_segment | shortening | stress_marker
    // continues with _word_marker tokens (no whitespace between)
    // ═══════════════════════════════════════════════════════════

    fn parse_word_with_annotations(&mut self) -> ContentItem<'a> {
        let mut category = None;
        let mut body = Vec::new();
        let mut form_marker = None;
        let mut lang = None;
        let mut pos_tag = None;
        let mut span_start = usize::MAX;
        let mut span_end = 0;

        // Consume all adjacent word tokens, classifying into named fields
        while let Some(d) = self.peek_d() {
            if !is_word_token(d) {
                break;
            }
            let (tok, span) = self.advance().unwrap();
            if span_start == usize::MAX {
                span_start = span.start;
            }
            span_end = span.end;

            match tok {
                // Category prefixes
                Token::Zero(_) => category = Some(WordCategory::Omission),
                Token::PrefixFiller(_) => category = Some(WordCategory::Filler),
                Token::PrefixNonword(_) => category = Some(WordCategory::Nonword),
                Token::PrefixFragment(_) => category = Some(WordCategory::Fragment),
                // Suffix markers — tag-extracted content, stored directly
                Token::FormMarker(s) => form_marker = Some(s),
                Token::WordLangSuffix(opt) => {
                    lang = Some(match opt {
                        None => ParsedLangSuffix::Shortcut,
                        Some(codes) => ParsedLangSuffix::Explicit(codes),
                    });
                }
                Token::PosTag(s) => pos_tag = Some(s),
                // Word body content — explicitly typed
                Token::WordSegment(s) => body.push(WordBodyItem::Text(s)),
                Token::Shortening(s) => body.push(WordBodyItem::Shortening(s)),
                Token::Lengthening(s) => body.push(WordBodyItem::Lengthening(s.len() as u8)),
                Token::CompoundMarker(_) => body.push(WordBodyItem::CompoundMarker),
                Token::StressPrimary(_) => body.push(WordBodyItem::Stress(StressKind::Primary)),
                Token::StressSecondary(_) => body.push(WordBodyItem::Stress(StressKind::Secondary)),
                Token::OverlapTopBegin(s) => {
                    body.push(WordBodyItem::OverlapPoint(OverlapKind::TopBegin, s))
                }
                Token::OverlapTopEnd(s) => {
                    body.push(WordBodyItem::OverlapPoint(OverlapKind::TopEnd, s))
                }
                Token::OverlapBottomBegin(s) => {
                    body.push(WordBodyItem::OverlapPoint(OverlapKind::BottomBegin, s))
                }
                Token::OverlapBottomEnd(s) => {
                    body.push(WordBodyItem::OverlapPoint(OverlapKind::BottomEnd, s))
                }
                Token::SyllablePause(_) => body.push(WordBodyItem::SyllablePause),
                Token::Tilde(_) => body.push(WordBodyItem::CliticBoundary),
                // CA elements
                Token::CaBlockedSegments(_) => {
                    body.push(WordBodyItem::CaElement(CaElementKind::BlockedSegments))
                }
                Token::CaConstriction(_) => {
                    body.push(WordBodyItem::CaElement(CaElementKind::Constriction))
                }
                Token::CaHardening(_) => {
                    body.push(WordBodyItem::CaElement(CaElementKind::Hardening))
                }
                Token::CaHurriedStart(_) => {
                    body.push(WordBodyItem::CaElement(CaElementKind::HurriedStart))
                }
                Token::CaInhalation(_) => {
                    body.push(WordBodyItem::CaElement(CaElementKind::Inhalation))
                }
                Token::CaLaughInWord(_) => {
                    body.push(WordBodyItem::CaElement(CaElementKind::LaughInWord))
                }
                Token::CaPitchDown(_) => {
                    body.push(WordBodyItem::CaElement(CaElementKind::PitchDown))
                }
                Token::CaPitchReset(_) => {
                    body.push(WordBodyItem::CaElement(CaElementKind::PitchReset))
                }
                Token::CaPitchUp(_) => body.push(WordBodyItem::CaElement(CaElementKind::PitchUp)),
                Token::CaSuddenStop(_) => {
                    body.push(WordBodyItem::CaElement(CaElementKind::SuddenStop))
                }
                // CA delimiters
                Token::CaUnsure(_) => body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::Unsure)),
                Token::CaPrecise(_) => {
                    body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::Precise))
                }
                Token::CaCreaky(_) => body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::Creaky)),
                Token::CaSofter(_) => body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::Softer)),
                Token::CaSegmentRepetition(_) => body.push(WordBodyItem::CaDelimiter(
                    CaDelimiterKind::SegmentRepetition,
                )),
                Token::CaFaster(_) => body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::Faster)),
                Token::CaSlower(_) => body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::Slower)),
                Token::CaWhisper(_) => {
                    body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::Whisper))
                }
                Token::CaSinging(_) => {
                    body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::Singing))
                }
                Token::CaLowPitch(_) => {
                    body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::LowPitch))
                }
                Token::CaHighPitch(_) => {
                    body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::HighPitch))
                }
                Token::CaLouder(_) => body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::Louder)),
                Token::CaSmileVoice(_) => {
                    body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::SmileVoice))
                }
                Token::CaBreathyVoice(_) => {
                    body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::BreathyVoice))
                }
                Token::CaYawn(_) => body.push(WordBodyItem::CaDelimiter(CaDelimiterKind::Yawn)),
                // Should not reach here if is_word_token() is correct
                other => panic!("unexpected token in word body: {:?}", other.text()),
            }
        }

        let raw_text = &self.source[span_start..span_end];

        // Trailing annotations (possibly with whitespace before)
        let mut annotations = Vec::new();
        loop {
            let saved = self.pos;
            self.skip_ws();
            if is_annotation(self.peek_d()) {
                let tok = self.advance().unwrap().0;
                annotations.push(token_to_parsed_annotation(tok));
            } else {
                self.pos = saved;
                break;
            }
        }

        let word = WordWithAnnotations {
            category,
            body,
            form_marker,
            lang,
            pos_tag,
            annotations,
            raw_text,
        };

        word_to_content_item(word)
    }

    // ═══════════════════════════════════════════════════════════
    // Rich Word token handling — lexer provides Word { raw_text, prefix, body, ... }
    // Parser handles body internals and trailing annotations.
    // ═══════════════════════════════════════════════════════════

    fn parse_rich_word_with_annotations(&mut self) -> ContentItem<'a> {
        let (tok, _span) = self.advance().unwrap();
        let (raw_text, prefix, body_str, form_marker, lang_suffix_opt, pos_tag) = match tok {
            Token::Word {
                raw_text,
                prefix,
                body,
                form_marker,
                lang_suffix,
                pos_tag,
            } => (raw_text, prefix, body, form_marker, lang_suffix, pos_tag),
            _ => unreachable!("parse_rich_word_with_annotations called on non-Word token"),
        };

        // Map prefix to category
        let category = match prefix {
            Some("&-") => Some(WordCategory::Filler),
            Some("&~") => Some(WordCategory::Nonword),
            Some("&+") => Some(WordCategory::Fragment),
            Some("0") => Some(WordCategory::Omission),
            _ => None,
        };

        // Parse body for internal structure
        let body = parse_word_body(body_str);

        // Map language suffix
        let lang = match lang_suffix_opt {
            None => None,
            Some("") => Some(ParsedLangSuffix::Shortcut),
            Some(codes) => Some(ParsedLangSuffix::Explicit(codes)),
        };

        // Trailing annotations (possibly with whitespace before)
        let mut annotations = Vec::new();
        loop {
            let saved = self.pos;
            self.skip_ws();
            if is_annotation(self.peek_d()) {
                let tok = self.advance().unwrap().0;
                annotations.push(token_to_parsed_annotation(tok));
            } else {
                self.pos = saved;
                break;
            }
        }

        let word = WordWithAnnotations {
            category,
            body,
            form_marker,
            lang,
            pos_tag,
            annotations,
            raw_text,
        };

        // Check for retrace annotation — if found, all annotations belong
        // to the retrace, not the word (grammar.js: annotations are on the
        // word_with_optional_annotations wrapper, which becomes the retrace)
        word_to_content_item(word)
    }

    // grammar.js: group_with_annotations = seq(<, contents, >, base_annotations)
    fn parse_group(&mut self) -> ContentItem<'a> {
        self.advance(); // <
        let contents = self.parse_contents_until(Some(TokenDiscriminants::GreaterThan));
        self.expect(TokenDiscriminants::GreaterThan);
        let mut annotations: Vec<ParsedAnnotation<'a>> = Vec::new();
        self.skip_ws();
        while is_annotation(self.peek_d()) {
            let tok = self.advance().unwrap().0;
            annotations.push(token_to_parsed_annotation(tok));
        }
        // Check for retrace annotation on the group
        let retrace_idx = annotations.iter().position(|a| a.is_retrace());
        if let Some(idx) = retrace_idx {
            let ann = annotations.remove(idx);
            let kind = ann.retrace_kind().expect("is_retrace was true");
            ContentItem::Retrace(Retrace {
                content: contents,
                kind,
                is_group: true,
                annotations,
            })
        } else {
            ContentItem::Group(Group {
                contents,
                annotations,
            })
        }
    }

    // grammar.js: pho_group = seq(left_guillemet, contents, right_guillemet)
    fn parse_pho_group(&mut self) -> ContentItem<'a> {
        self.advance(); // ‹
        let contents = self.parse_contents_until(Some(TokenDiscriminants::PhoGroupEnd));
        self.expect(TokenDiscriminants::PhoGroupEnd);
        ContentItem::PhoGroup(contents)
    }

    // grammar.js: sin_group = seq(left_tortoise_shell, contents, right_tortoise_shell)
    fn parse_sin_group(&mut self) -> ContentItem<'a> {
        self.advance(); // 〔
        let contents = self.parse_contents_until(Some(TokenDiscriminants::SinGroupEnd));
        self.expect(TokenDiscriminants::SinGroupEnd);
        ContentItem::SinGroup(contents)
    }

    // grammar.js: quotation = seq(left_double_quote, contents, right_double_quote)
    fn parse_quotation(&mut self) -> ContentItem<'a> {
        self.advance(); // "
        let contents = self.parse_contents_until(Some(TokenDiscriminants::RightDoubleQuote));
        self.expect(TokenDiscriminants::RightDoubleQuote);
        ContentItem::Quotation(Quotation { contents })
    }

    // grammar.js: event = seq(event_marker, event_segment+)
    fn parse_event(&mut self) -> ContentItem<'a> {
        let mut toks = vec![self.advance().unwrap().0]; // &=
        // Consume event segment tokens (anything until ws/terminator/annotation)
        while let Some(d) = self.peek_d() {
            if matches!(
                d,
                TokenDiscriminants::Whitespace
                    | TokenDiscriminants::Newline
                    | TokenDiscriminants::Continuation
            ) || is_terminator(Some(d))
                || is_annotation(Some(d))
            {
                break;
            }
            toks.push(self.advance().unwrap().0);
        }
        ContentItem::Event(toks)
    }

    pub fn parse_optional_terminator(&mut self) -> Option<Token<'a>> {
        self.skip_ws();
        if is_terminator(self.peek_d()) {
            Some(self.advance().unwrap().0)
        } else {
            None
        }
    }

    // ═══════════════════════════════════════════════════════════
    // %mor tier parsing
    // grammar.js: mor_contents = seq(mor_content+, optional(terminator))
    // grammar.js: mor_content = seq(mor_word, repeat(seq(tilde, mor_word)))
    //
    // The lexer emits rich MorWord tokens. The parser just sequences them.
    // ═══════════════════════════════════════════════════════════

    fn parse_mor_tier(&mut self) -> MorTier<'a> {
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.at_end()
                || is_terminator(self.peek_d())
                || self.peek_d() == Some(TokenDiscriminants::Newline)
            {
                break;
            }
            if self.peek_d() == Some(TokenDiscriminants::MorWord) {
                if let Some(item) = self.parse_mor_item() {
                    items.push(item);
                }
            } else {
                self.advance(); // skip unknown
            }
        }
        let terminator = self.parse_optional_terminator();
        MorTier { items, terminator }
    }

    fn parse_mor_item(&mut self) -> Option<MorItem<'a>> {
        let main = self.parse_mor_word_token()?;
        let mut post_clitics = Vec::new();
        while self.peek_d() == Some(TokenDiscriminants::MorTilde) {
            self.advance(); // consume ~
            if let Some(clitic) = self.parse_mor_word_token() {
                post_clitics.push(clitic);
            }
        }
        Some(MorItem { main, post_clitics })
    }

    fn parse_mor_word_token(&mut self) -> Option<MorWordParsed<'a>> {
        let (tok, _) = self.expect(TokenDiscriminants::MorWord)?;
        match tok {
            Token::MorWord {
                pos,
                lemma_features,
            } => {
                // Split lemma_features on first '-' for lemma vs features
                let mut parts = lemma_features.splitn(2, '-');
                let lemma = parts.next().unwrap_or("");
                let features: Vec<&str> = match parts.next() {
                    Some(feat_str) => feat_str.split('-').collect(),
                    None => vec![],
                };
                Some(MorWordParsed {
                    pos,
                    lemma,
                    features,
                })
            }
            _ => None,
        }
    }

    // ═══════════════════════════════════════════════════════════
    // %gra tier parsing
    // grammar.js: gra_contents = seq(gra_relation+)
    //
    // The lexer emits rich GraRelation tokens. Parser sequences them.
    // ═══════════════════════════════════════════════════════════

    fn parse_gra_tier(&mut self) -> GraTier<'a> {
        let mut relations = Vec::new();
        loop {
            self.skip_ws();
            if self.at_end() || self.peek_d() == Some(TokenDiscriminants::Newline) {
                break;
            }
            if self.peek_d() == Some(TokenDiscriminants::GraRelation) {
                let (tok, _) = self.advance().unwrap();
                if let Token::GraRelation {
                    index,
                    head,
                    relation,
                } = tok
                {
                    relations.push(GraRelationParsed {
                        index,
                        head,
                        relation,
                    });
                }
            } else {
                self.advance(); // skip unknown
            }
        }
        GraTier { relations }
    }

    // ═══════════════════════════════════════════════════════════
    // %wor tier parsing — words with optional inline timing bullets
    // grammar.js: wor_tier_body = seq(optional(langcode), repeat(wor_word_item | bullet | sep), terminator?)
    // ═══════════════════════════════════════════════════════════

    fn parse_wor_tier_body(&mut self) -> (Vec<WorItemParsed<'a>>, Option<Token<'a>>) {
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.at_end()
                || is_terminator(self.peek_d())
                || self.peek_d() == Some(TokenDiscriminants::Newline)
            {
                break;
            }
            match self.peek_d() {
                Some(TokenDiscriminants::Word) => {
                    let word_item = self.parse_rich_word_with_annotations();
                    if let ContentItem::Word(w) = word_item {
                        // Check for adjacent bullet
                        self.skip_ws();
                        let bullet = if self.peek_d() == Some(TokenDiscriminants::MediaBullet) {
                            let (tok, _) = self.advance().unwrap();
                            if let Token::MediaBullet {
                                start_time,
                                end_time,
                            } = tok
                            {
                                let s: u64 = start_time.parse().unwrap_or(0);
                                let e: u64 = end_time.parse().unwrap_or(0);
                                Some((s, e))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        items.push(WorItemParsed::Word { word: w, bullet });
                    }
                }
                _ if is_word_start(self.peek_d().unwrap_or(TokenDiscriminants::Newline)) => {
                    let word_item = self.parse_word_with_annotations();
                    if let ContentItem::Word(w) = word_item {
                        self.skip_ws();
                        let bullet = if self.peek_d() == Some(TokenDiscriminants::MediaBullet) {
                            let (tok, _) = self.advance().unwrap();
                            if let Token::MediaBullet {
                                start_time,
                                end_time,
                            } = tok
                            {
                                let s: u64 = start_time.parse().unwrap_or(0);
                                let e: u64 = end_time.parse().unwrap_or(0);
                                Some((s, e))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        items.push(WorItemParsed::Word { word: w, bullet });
                    }
                }
                Some(d) if is_separator(Some(d)) => {
                    items.push(WorItemParsed::Separator(self.advance().unwrap().0));
                }
                Some(TokenDiscriminants::MediaBullet) => {
                    self.advance(); // orphan bullet, skip
                }
                _ => {
                    self.advance(); // skip unknown
                }
            }
        }
        let terminator = self.parse_optional_terminator();
        (items, terminator)
    }

    // ═══════════════════════════════════════════════════════════
    // %pho tier parsing
    // ═══════════════════════════════════════════════════════════

    fn parse_pho_tier_body(&mut self) -> PhoTier<'a> {
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.at_end()
                || is_terminator(self.peek_d())
                || self.peek_d() == Some(TokenDiscriminants::Newline)
            {
                break;
            }
            match self.peek_d() {
                Some(TokenDiscriminants::PhoWord) => {
                    items.push(PhoItemParsed::Word(self.parse_pho_word()));
                }
                Some(TokenDiscriminants::PhoGroupBegin) => {
                    self.advance(); // ‹
                    let mut group = Vec::new();
                    loop {
                        self.skip_ws();
                        match self.peek_d() {
                            Some(TokenDiscriminants::PhoGroupEnd) => {
                                self.advance(); // ›
                                break;
                            }
                            Some(TokenDiscriminants::PhoWord) => {
                                group.push(self.parse_pho_word());
                            }
                            None => break,
                            _ => {
                                self.advance();
                            }
                        }
                    }
                    items.push(PhoItemParsed::Group(group));
                }
                Some(
                    TokenDiscriminants::PauseLong
                    | TokenDiscriminants::PauseMedium
                    | TokenDiscriminants::PauseShort,
                ) => {
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }
        let terminator = self.parse_optional_terminator();
        PhoTier { items, terminator }
    }

    /// Parse a single pho word (possibly compound: word+word+word).
    fn parse_pho_word(&mut self) -> PhoWordParsed<'a> {
        let mut segments = vec![self.advance().unwrap().0.text()];
        while self.peek_d() == Some(TokenDiscriminants::PhoPlus) {
            self.advance(); // +
            if self.peek_d() == Some(TokenDiscriminants::PhoWord) {
                segments.push(self.advance().unwrap().0.text());
            }
        }
        PhoWordParsed { segments }
    }

    // ═══════════════════════════════════════════════════════════
    // %sin tier parsing
    // ═══════════════════════════════════════════════════════════

    fn parse_sin_tier_body(&mut self) -> SinTierParsed<'a> {
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.at_end() || self.peek_d() == Some(TokenDiscriminants::Newline) {
                break;
            }
            match self.peek_d() {
                Some(TokenDiscriminants::SinWord) | Some(TokenDiscriminants::Zero) => {
                    items.push(SinItemParsed::Token(self.advance().unwrap().0.text()));
                }
                Some(TokenDiscriminants::SinGroupBegin) => {
                    self.advance(); // 〔
                    let mut group = Vec::new();
                    loop {
                        self.skip_ws();
                        match self.peek_d() {
                            Some(TokenDiscriminants::SinGroupEnd) => {
                                self.advance(); // 〕
                                break;
                            }
                            Some(TokenDiscriminants::SinWord) | Some(TokenDiscriminants::Zero) => {
                                group.push(self.advance().unwrap().0.text());
                            }
                            None => break,
                            _ => {
                                self.advance();
                            }
                        }
                    }
                    items.push(SinItemParsed::Group(group));
                }
                _ => {
                    self.advance();
                }
            }
        }
        SinTierParsed { items }
    }

    // ═══════════════════════════════════════════════════════════
    // Text tier parsing (text_with_bullets)
    // ═══════════════════════════════════════════════════════════

    fn parse_text_tier_body(&mut self) -> TextTierParsed<'a> {
        let mut segments = Vec::new();
        loop {
            if self.at_end() || self.peek_d() == Some(TokenDiscriminants::Newline) {
                break;
            }
            match self.peek_d() {
                Some(TokenDiscriminants::TextSegment) => {
                    segments.push(TextTierSegment::Text(self.advance().unwrap().0.text()));
                }
                Some(TokenDiscriminants::MediaBullet) => {
                    segments.push(TextTierSegment::Bullet(self.advance().unwrap().0));
                }
                Some(TokenDiscriminants::InlinePic) => {
                    segments.push(TextTierSegment::Pic(self.advance().unwrap().0));
                }
                Some(TokenDiscriminants::Continuation) => {
                    self.advance(); // continuation is structural
                }
                _ => {
                    self.advance();
                }
            }
        }
        TextTierParsed { segments }
    }

    // ═══════════════════════════════════════════════════════════
    // @ID header parsing — extract 10 pipe-delimited fields
    // ═══════════════════════════════════════════════════════════

    fn parse_id_header(&mut self) -> Option<IdHeaderParsed<'a>> {
        let (tok, _) = self.expect(TokenDiscriminants::IdFields)?;
        match tok {
            Token::IdFields {
                language,
                corpus,
                speaker,
                age,
                sex,
                group,
                ses,
                role,
                education,
                custom,
            } => Some(IdHeaderParsed {
                language,
                corpus,
                speaker,
                age,
                sex,
                group,
                ses,
                role,
                education,
                custom_field: custom,
            }),
            _ => None,
        }
    }

    // ═══════════════════════════════════════════════════════════
    // @Languages header parsing — comma-separated codes
    // ═══════════════════════════════════════════════════════════

    fn parse_languages_header(&mut self) -> LanguagesHeaderParsed<'a> {
        let mut codes = Vec::new();
        loop {
            self.skip_ws();
            if self.at_end() || self.peek_d() == Some(TokenDiscriminants::Newline) {
                break;
            }
            if self.peek_d() == Some(TokenDiscriminants::LanguageCode) {
                codes.push(self.advance().unwrap().0.text());
            } else if self.peek_d() == Some(TokenDiscriminants::Comma) {
                self.advance();
            } else {
                self.advance(); // skip unknown
            }
        }
        LanguagesHeaderParsed { codes }
    }

    // ═══════════════════════════════════════════════════════════
    // @Participants header parsing — comma-separated entries
    // ═══════════════════════════════════════════════════════════

    fn parse_participants_header(&mut self) -> ParticipantsHeaderParsed<'a> {
        let mut entries = Vec::new();
        let mut current_words: Vec<&'a str> = Vec::new();

        loop {
            self.skip_ws();
            if self.at_end() || self.peek_d() == Some(TokenDiscriminants::Newline) {
                break;
            }
            match self.peek_d() {
                Some(TokenDiscriminants::ParticipantWord) => {
                    current_words.push(self.advance().unwrap().0.text());
                }
                Some(TokenDiscriminants::Comma) => {
                    self.advance();
                    if !current_words.is_empty() {
                        entries.push(ParticipantEntryParsed {
                            words: std::mem::take(&mut current_words),
                        });
                    }
                }
                _ => {
                    self.advance();
                }
            }
        }
        if !current_words.is_empty() {
            entries.push(ParticipantEntryParsed {
                words: current_words,
            });
        }
        ParticipantsHeaderParsed { entries }
    }

    // ═══════════════════════════════════════════════════════════
    // Full file parsing
    // ═══════════════════════════════════════════════════════════

    fn parse_file(&mut self) -> ChatFile<'a> {
        let mut lines = Vec::new();
        while !self.at_end() {
            match self.peek_d() {
                // No-content headers
                Some(
                    TokenDiscriminants::HeaderUtf8
                    | TokenDiscriminants::HeaderBegin
                    | TokenDiscriminants::HeaderEnd
                    | TokenDiscriminants::HeaderBlank
                    | TokenDiscriminants::HeaderNewEpisode,
                ) => {
                    let (tok, _) = self.advance().unwrap();
                    // Consume newline
                    if self.peek_d() == Some(TokenDiscriminants::Newline) {
                        self.advance();
                    }
                    lines.push(Line::Header(HeaderParsed {
                        prefix: tok,
                        content: vec![],
                    }));
                }
                // Headers with content (HeaderPrefix or specific header type + content tokens + newline)
                Some(
                    TokenDiscriminants::HeaderPrefix
                    | TokenDiscriminants::HeaderBirthOf
                    | TokenDiscriminants::HeaderBirthplaceOf
                    | TokenDiscriminants::HeaderL1Of,
                ) => {
                    let (prefix, _) = self.advance().unwrap();
                    // Collect ALL content tokens until newline.
                    // Keep Continuation tokens — they're semantically significant
                    // in BulletContent (preserved as separate segments).
                    let mut content = Vec::new();
                    while !self.at_end()
                        && !matches!(self.peek_d(), Some(TokenDiscriminants::Newline))
                    {
                        let (tok, _) = self.advance().unwrap();
                        // Skip whitespace (structural) but keep continuations
                        if !matches!(tok, Token::Whitespace(_)) {
                            content.push(tok);
                        }
                    }
                    if self.peek_d() == Some(TokenDiscriminants::Newline) {
                        self.advance();
                    }
                    lines.push(Line::Header(HeaderParsed { prefix, content }));
                }
                // Main tier
                Some(TokenDiscriminants::Star) => {
                    if let Some(main_tier) = self.parse_main_tier() {
                        // Collect dependent tiers
                        let mut dep_tiers = Vec::new();
                        while self.peek_d() == Some(TokenDiscriminants::TierPrefix) {
                            let (prefix, _) = self.advance().unwrap();
                            let prefix_text = prefix.text();
                            if prefix_text.starts_with("%mor") || prefix_text.starts_with("%trn") {
                                let tier = self.parse_mor_tier();
                                dep_tiers.push(DependentTierParsed::Mor(tier));
                                // Consume trailing newline so loop sees next TierPrefix
                                if self.peek_d() == Some(TokenDiscriminants::Newline) {
                                    self.advance();
                                }
                            } else if prefix_text.starts_with("%pho") {
                                let tier = self.parse_pho_tier_body();
                                dep_tiers.push(DependentTierParsed::Pho(tier));
                                if self.peek_d() == Some(TokenDiscriminants::Newline) {
                                    self.advance();
                                }
                            } else if prefix_text.starts_with("%mod") {
                                let tier = self.parse_pho_tier_body();
                                dep_tiers.push(DependentTierParsed::Mod(tier));
                                if self.peek_d() == Some(TokenDiscriminants::Newline) {
                                    self.advance();
                                }
                            } else if prefix_text.starts_with("%wor") {
                                let (items, terminator) = self.parse_wor_tier_body();
                                dep_tiers.push(DependentTierParsed::Wor { items, terminator });
                                if self.peek_d() == Some(TokenDiscriminants::Newline) {
                                    self.advance();
                                }
                            } else if prefix_text.starts_with("%gra") {
                                let tier = self.parse_gra_tier();
                                dep_tiers.push(DependentTierParsed::Gra(tier));
                                // Consume trailing newline
                                if self.peek_d() == Some(TokenDiscriminants::Newline) {
                                    self.advance();
                                }
                            } else if prefix_text.starts_with("%sin") {
                                let tier = self.parse_sin_tier_body();
                                dep_tiers.push(DependentTierParsed::Sin(tier));
                                if self.peek_d() == Some(TokenDiscriminants::Newline) {
                                    self.advance();
                                }
                            } else {
                                // Generic text tier — collect until newline
                                let mut content = Vec::new();
                                while !self.at_end()
                                    && !matches!(self.peek_d(), Some(TokenDiscriminants::Newline))
                                {
                                    content.push(self.advance().unwrap().0);
                                }
                                if self.peek_d() == Some(TokenDiscriminants::Newline) {
                                    self.advance();
                                }
                                dep_tiers.push(DependentTierParsed::Text { prefix, content });
                            }
                        }
                        lines.push(Line::Utterance(Utterance {
                            main_tier,
                            dependent_tiers: dep_tiers,
                        }));
                    }
                }
                // Skip whitespace, newlines, continuations, errors
                Some(
                    TokenDiscriminants::Whitespace
                    | TokenDiscriminants::Newline
                    | TokenDiscriminants::Continuation
                    | TokenDiscriminants::BOM,
                ) => {
                    self.advance();
                }
                // Orphan tier prefix (no preceding main tier)
                Some(TokenDiscriminants::TierPrefix) => {
                    // Skip until newline
                    while !self.at_end() && self.peek_d() != Some(TokenDiscriminants::Newline) {
                        self.advance();
                    }
                    if self.peek_d() == Some(TokenDiscriminants::Newline) {
                        self.advance();
                    }
                }
                // Every token type must be explicitly handled above.
                _ => {
                    let (tok, span) = self.advance().unwrap();
                    self.errors.report(ParseError::new(
                        talkbank_model::errors::codes::ErrorCode::UnexpectedSyntax,
                        talkbank_model::Severity::Warning,
                        talkbank_model::SourceLocation::new(Span::new(
                            span.start as u32,
                            span.end as u32,
                        )),
                        None,
                        format!("unhandled token in parse_chat_file: {:?}", tok.text()),
                    ));
                }
            }
        }
        ChatFile {
            lines,
            source: self.source,
        }
    }

    fn parse_postcodes(&mut self) -> Vec<Token<'a>> {
        let mut codes = Vec::new();
        loop {
            self.skip_ws();
            if self.peek_d() == Some(TokenDiscriminants::Postcode) {
                codes.push(self.advance().unwrap().0);
            } else {
                break;
            }
        }
        codes
    }
}

/// Convert a `WordWithAnnotations` to the appropriate `ContentItem`.
///
/// If annotations include a retrace marker (`[/]`, `[//]`, etc.), ALL
/// annotations are moved to the retrace level — the word inside the
/// retrace gets no annotations. This matches grammar.js semantics
/// where annotations attach to `word_with_optional_annotations`, not
/// `standalone_word`.
fn word_to_content_item<'a>(word: WordWithAnnotations<'a>) -> ContentItem<'a> {
    let retrace_idx = word.annotations.iter().position(|a| a.is_retrace());
    if let Some(idx) = retrace_idx {
        let mut word = word;
        let ann = word.annotations.remove(idx);
        let kind = ann.retrace_kind().expect("is_retrace was true");
        // grammar.js: word_with_optional_annotations has replacement AND base_annotations
        // as separate fields. Replacement stays on the word; base_annotations (scoped
        // markers like [?], [!], [= text]) go to retrace level.
        // BUT: when replacement exists, scoped annotations go on the ReplacedWord,
        // not on the retrace (TreeSitterParser behavior: ReplacedWord.with_scoped_annotations).
        let has_replacement = word
            .annotations
            .iter()
            .any(|a| matches!(a, ParsedAnnotation::Replacement(_)));
        let mut retrace_annotations = Vec::new();
        let mut word_annotations = Vec::new();
        for ann in std::mem::take(&mut word.annotations) {
            match &ann {
                ParsedAnnotation::Replacement(_) => word_annotations.push(ann),
                _ if has_replacement => {
                    // When replacement exists, scoped annotations stay with the word
                    // (they become ReplacedWord's scoped_annotations in the model)
                    word_annotations.push(ann);
                }
                _ => retrace_annotations.push(ann),
            }
        }
        word.annotations = word_annotations;
        let retrace = Retrace {
            content: vec![ContentItem::Word(word)],
            kind,
            is_group: false,
            annotations: retrace_annotations,
        };
        ContentItem::Retrace(retrace)
    } else {
        ContentItem::Word(word)
    }
}

/// Convert a raw annotation token to a typed ParsedAnnotation.
fn token_to_parsed_annotation<'a>(tok: Token<'a>) -> ParsedAnnotation<'a> {
    match tok {
        Token::RetracePartial(_) => ParsedAnnotation::Retrace(RetraceKindParsed::Partial),
        Token::RetraceComplete(_) => ParsedAnnotation::Retrace(RetraceKindParsed::Complete),
        Token::RetraceMultiple(_) => ParsedAnnotation::Retrace(RetraceKindParsed::Multiple),
        Token::RetraceReformulation(_) => {
            ParsedAnnotation::Retrace(RetraceKindParsed::Reformulation)
        }
        Token::RetraceUncertain(_) => ParsedAnnotation::Retrace(RetraceKindParsed::Uncertain),
        Token::ScopedStressing(_) => ParsedAnnotation::Stressing,
        Token::ScopedContrastiveStressing(_) => ParsedAnnotation::ContrastiveStressing,
        Token::ScopedBestGuess(_) => ParsedAnnotation::BestGuess,
        Token::ScopedUncertain(_) => ParsedAnnotation::Uncertain,
        Token::ExcludeMarker(_) => ParsedAnnotation::Exclude,
        Token::ErrorMarkerAnnotation(s) => ParsedAnnotation::Error(s),
        Token::OverlapPrecedes(s) => ParsedAnnotation::OverlapPrecedes(s),
        Token::OverlapFollows(s) => ParsedAnnotation::OverlapFollows(s),
        Token::ExplanationAnnotation(s) => ParsedAnnotation::Explanation(s),
        Token::ParaAnnotation(s) => ParsedAnnotation::Paralinguistic(s),
        Token::AltAnnotation(s) => ParsedAnnotation::Alternative(s),
        Token::PercentAnnotation(s) => ParsedAnnotation::PercentComment(s),
        Token::DurationAnnotation(s) => ParsedAnnotation::Duration(s),
        Token::Replacement(s) => ParsedAnnotation::Replacement(s),
        Token::Langcode(s) => ParsedAnnotation::Langcode(s),
        Token::Postcode(s) => ParsedAnnotation::Postcode(s),
        // Should not reach here if is_annotation() is correct
        other => panic!("token_to_parsed_annotation: unexpected {:?}", other.text()),
    }
}

// ── Token classification (from grammar.js rule definitions) ─────

fn is_terminator(d: Option<TokenDiscriminants>) -> bool {
    matches!(
        d,
        Some(
            TokenDiscriminants::Period
                | TokenDiscriminants::Question
                | TokenDiscriminants::Exclamation
                | TokenDiscriminants::TrailingOff
                | TokenDiscriminants::Interruption
                | TokenDiscriminants::SelfInterruption
                | TokenDiscriminants::InterruptedQuestion
                | TokenDiscriminants::BrokenQuestion
                | TokenDiscriminants::QuotedNewLine
                | TokenDiscriminants::QuotedPeriodSimple
                | TokenDiscriminants::SelfInterruptedQuestion
                | TokenDiscriminants::TrailingOffQuestion
                | TokenDiscriminants::BreakForCoding
                | TokenDiscriminants::CaNoBreak
                | TokenDiscriminants::CaTechnicalBreak
                | TokenDiscriminants::CaNoBreakLinker
                | TokenDiscriminants::CaTechnicalBreakLinker
        )
    )
}

fn is_linker(d: Option<TokenDiscriminants>) -> bool {
    matches!(
        d,
        Some(
            TokenDiscriminants::LinkerLazyOverlap
                | TokenDiscriminants::LinkerQuickUptake
                | TokenDiscriminants::LinkerQuickUptakeOverlap
                | TokenDiscriminants::LinkerQuotationFollows
                | TokenDiscriminants::LinkerSelfCompletion
                | TokenDiscriminants::CaNoBreakLinker
                | TokenDiscriminants::CaTechnicalBreakLinker
        )
    )
}

fn is_annotation(d: Option<TokenDiscriminants>) -> bool {
    matches!(
        d,
        Some(
            TokenDiscriminants::RetraceComplete
                | TokenDiscriminants::RetracePartial
                | TokenDiscriminants::RetraceMultiple
                | TokenDiscriminants::RetraceReformulation
                | TokenDiscriminants::RetraceUncertain
                | TokenDiscriminants::ScopedStressing
                | TokenDiscriminants::ScopedContrastiveStressing
                | TokenDiscriminants::ScopedBestGuess
                | TokenDiscriminants::ScopedUncertain
                | TokenDiscriminants::ExcludeMarker
                | TokenDiscriminants::ErrorMarkerAnnotation
                | TokenDiscriminants::OverlapPrecedes
                | TokenDiscriminants::OverlapFollows
                | TokenDiscriminants::ExplanationAnnotation
                | TokenDiscriminants::ParaAnnotation
                | TokenDiscriminants::AltAnnotation
                | TokenDiscriminants::PercentAnnotation
                | TokenDiscriminants::DurationAnnotation
                | TokenDiscriminants::Replacement
                | TokenDiscriminants::Langcode
        )
    )
}

fn is_separator(d: Option<TokenDiscriminants>) -> bool {
    matches!(
        d,
        Some(
            TokenDiscriminants::Comma
                | TokenDiscriminants::Semicolon
                | TokenDiscriminants::CaContinuationMarker
                | TokenDiscriminants::TagMarker
                | TokenDiscriminants::VocativeMarker
                | TokenDiscriminants::UnmarkedEnding
                | TokenDiscriminants::UptakeSymbol
                | TokenDiscriminants::RisingToHigh
                | TokenDiscriminants::RisingToMid
                | TokenDiscriminants::LevelPitch
                | TokenDiscriminants::FallingToMid
                | TokenDiscriminants::FallingToLow
        )
    )
}

fn is_pause(d: TokenDiscriminants) -> bool {
    matches!(
        d,
        TokenDiscriminants::PauseLong
            | TokenDiscriminants::PauseMedium
            | TokenDiscriminants::PauseShort
            | TokenDiscriminants::PauseTimed
    )
}

fn is_word_start(d: TokenDiscriminants) -> bool {
    matches!(
        d,
        TokenDiscriminants::WordSegment
            | TokenDiscriminants::Zero
            | TokenDiscriminants::PrefixFiller
            | TokenDiscriminants::PrefixNonword
            | TokenDiscriminants::PrefixFragment
            | TokenDiscriminants::Shortening
            | TokenDiscriminants::StressPrimary
            | TokenDiscriminants::StressSecondary
            | TokenDiscriminants::Ampersand
            // CA markers can start a word (standalone or preceding text)
            | TokenDiscriminants::CaBlockedSegments | TokenDiscriminants::CaConstriction
            | TokenDiscriminants::CaHardening | TokenDiscriminants::CaHurriedStart
            | TokenDiscriminants::CaInhalation | TokenDiscriminants::CaLaughInWord
            | TokenDiscriminants::CaPitchDown | TokenDiscriminants::CaPitchReset
            | TokenDiscriminants::CaPitchUp | TokenDiscriminants::CaSuddenStop
            | TokenDiscriminants::CaUnsure | TokenDiscriminants::CaPrecise
            | TokenDiscriminants::CaCreaky | TokenDiscriminants::CaSofter
            | TokenDiscriminants::CaSegmentRepetition | TokenDiscriminants::CaFaster
            | TokenDiscriminants::CaSlower | TokenDiscriminants::CaWhisper
            | TokenDiscriminants::CaSinging | TokenDiscriminants::CaLowPitch
            | TokenDiscriminants::CaHighPitch | TokenDiscriminants::CaLouder
            | TokenDiscriminants::CaSmileVoice | TokenDiscriminants::CaBreathyVoice
            | TokenDiscriminants::CaYawn
    )
}

fn is_word_token(d: TokenDiscriminants) -> bool {
    matches!(
        d,
        TokenDiscriminants::WordSegment
            | TokenDiscriminants::Zero
            | TokenDiscriminants::PrefixFiller
            | TokenDiscriminants::PrefixNonword
            | TokenDiscriminants::PrefixFragment
            | TokenDiscriminants::Shortening
            | TokenDiscriminants::Lengthening
            | TokenDiscriminants::StressPrimary | TokenDiscriminants::StressSecondary
            | TokenDiscriminants::CompoundMarker
            | TokenDiscriminants::OverlapTopBegin | TokenDiscriminants::OverlapTopEnd | TokenDiscriminants::OverlapBottomBegin | TokenDiscriminants::OverlapBottomEnd
            | TokenDiscriminants::SyllablePause
            | TokenDiscriminants::Tilde
            // Note: UnderlineBegin/End are NOT word tokens — they're content-level markers
            // | TokenDiscriminants::UnderlineBegin
            // | TokenDiscriminants::UnderlineEnd
            | TokenDiscriminants::CaBlockedSegments | TokenDiscriminants::CaConstriction
            | TokenDiscriminants::CaHardening | TokenDiscriminants::CaHurriedStart
            | TokenDiscriminants::CaInhalation | TokenDiscriminants::CaLaughInWord
            | TokenDiscriminants::CaPitchDown | TokenDiscriminants::CaPitchReset
            | TokenDiscriminants::CaPitchUp | TokenDiscriminants::CaSuddenStop
            | TokenDiscriminants::CaUnsure | TokenDiscriminants::CaPrecise
            | TokenDiscriminants::CaCreaky | TokenDiscriminants::CaSofter
            | TokenDiscriminants::CaSegmentRepetition | TokenDiscriminants::CaFaster
            | TokenDiscriminants::CaSlower | TokenDiscriminants::CaWhisper
            | TokenDiscriminants::CaSinging | TokenDiscriminants::CaLowPitch
            | TokenDiscriminants::CaHighPitch | TokenDiscriminants::CaLouder
            | TokenDiscriminants::CaSmileVoice | TokenDiscriminants::CaBreathyVoice
            | TokenDiscriminants::CaYawn
            | TokenDiscriminants::FormMarker
            | TokenDiscriminants::WordLangSuffix
            | TokenDiscriminants::PosTag
    )
}

// ── Public convenience ──────────────────────────────────────────

/// Parse a main tier string starting with '*'.
pub fn parse_main_tier(input: &str) -> Option<MainTier<'_>> {
    Parser::new(input, 0).parse_main_tier()
}

/// Parse an @ID header content (after `@ID:\t`).
/// Example: `"eng|corpus|CHI|3;0|female|typical||Child|||\n"`
pub fn parse_id_header(input: &str) -> Option<IdHeaderParsed<'_>> {
    let mut p = Parser::new(input, crate::lexer::COND_ID_CONTENT);
    p.parse_id_header()
}

/// Parse a @Languages header content (after `@Languages:\t`).
pub fn parse_languages_header(input: &str) -> LanguagesHeaderParsed<'_> {
    let mut p = Parser::new(input, crate::lexer::COND_LANGUAGES_CONTENT);
    p.parse_languages_header()
}

/// Parse a @Participants header content (after `@Participants:\t`).
pub fn parse_participants_header(input: &str) -> ParticipantsHeaderParsed<'_> {
    let mut p = Parser::new(input, crate::lexer::COND_PARTICIPANTS_CONTENT);
    p.parse_participants_header()
}

/// Parse a single word (content item) from main tier content.
/// Example: `"ice+cream@f"`
pub fn parse_word(input: &str) -> Option<WordWithAnnotations<'_>> {
    let mut p = Parser::new(input, crate::lexer::COND_MAIN_CONTENT);
    let d = p.peek_d()?;
    let item = if d == TokenDiscriminants::Word {
        p.parse_rich_word_with_annotations()
    } else if is_word_start(d) {
        p.parse_word_with_annotations()
    } else {
        return None;
    };
    match item {
        ContentItem::Word(w) => Some(w),
        _ => None,
    }
}

/// Parse a single MorWord from %mor content.
/// Example: `"verb|want-Fin-Ind-Pres"`
pub fn parse_mor_word(input: &str) -> Option<MorWordParsed<'_>> {
    let mut p = Parser::new(input, crate::lexer::COND_MOR_CONTENT);
    p.parse_mor_word_token()
}

/// Parse a single GraRelation from %gra content.
/// Example: `"1|2|SUBJ"`
pub fn parse_gra_relation(input: &str) -> Option<GraRelationParsed<'_>> {
    let mut p = Parser::new(input, crate::lexer::COND_GRA_CONTENT);
    if p.peek_d() == Some(TokenDiscriminants::GraRelation) {
        let (tok, _) = p.advance().unwrap();
        if let Token::GraRelation {
            index,
            head,
            relation,
        } = tok
        {
            return Some(GraRelationParsed {
                index,
                head,
                relation,
            });
        }
        None
    } else {
        None
    }
}

/// Parse a %pho tier body (content after `%pho:\t`).
/// Example: `"wɑ+kɪŋ hɛloʊ .\n"`
pub fn parse_pho_tier(input: &str) -> PhoTier<'_> {
    let mut p = Parser::new(input, crate::lexer::COND_PHO_CONTENT);
    p.parse_pho_tier_body()
}

/// Parse a text tier body (content after `%com:\t`, `%act:\t`, etc.).
/// Returns text segments and media bullets.
pub fn parse_text_tier(input: &str) -> TextTierParsed<'_> {
    let mut p = Parser::new(input, crate::lexer::COND_TIER_CONTENT);
    p.parse_text_tier_body()
}

/// Parse a complete CHAT file.
pub fn parse_chat_file(input: &str) -> ChatFile<'_> {
    let mut p = Parser::new(input, 0);
    p.parse_file()
}

/// Parse a %mor tier body (content after `%mor:\t`).
/// Input should be the mor content WITHOUT the `%mor:\t` prefix.
/// Example: `"pro|I v|want n|cookie-PL .\n"`
pub fn parse_mor_tier(input: &str) -> MorTier<'_> {
    let mut p = Parser::new(input, crate::lexer::COND_MOR_CONTENT);
    p.parse_mor_tier()
}

/// Parse a %gra tier body (content after `%gra:\t`).
/// Example: `"1|2|SUBJ 2|0|ROOT 3|2|OBJ\n"`
pub fn parse_gra_tier(input: &str) -> GraTier<'_> {
    let mut p = Parser::new(input, crate::lexer::COND_GRA_CONTENT);
    p.parse_gra_tier()
}

// ═══════════════════════════════════════════════════════════
// Word body parser — scans body &str for internal structure.
//
// The lexer determines word boundaries and extracts prefix/suffixes.
// The body contains: text segments, shortenings, lengthening,
// compound markers, stress, overlap points, syllable pause,
// clitic boundary, CA elements/delimiters, underline markers.
// ═══════════════════════════════════════════════════════════

/// Parse a word body string into structured `WordBodyItem` list.
/// The body is the interior of a word (no prefix, no suffixes).
pub fn parse_word_body(body: &str) -> Vec<WordBodyItem<'_>> {
    let mut items = Vec::new();
    let mut chars = body.char_indices().peekable();

    while let Some(&(i, ch)) = chars.peek() {
        match ch {
            // Shortening: (text)
            '(' => {
                chars.next();
                let content_start = chars.peek().map_or(body.len(), |&(j, _)| j);
                // Scan to closing )
                while let Some(&(_, c)) = chars.peek() {
                    if c == ')' {
                        break;
                    }
                    chars.next();
                }
                let content_end = chars.peek().map_or(body.len(), |&(j, _)| j);
                if chars.peek().is_some() {
                    chars.next(); // consume ')'
                }
                items.push(WordBodyItem::Shortening(&body[content_start..content_end]));
            }
            // Lengthening: one or more colons
            ':' => {
                let mut count: u8 = 0;
                while let Some(&(_, ':')) = chars.peek() {
                    chars.next();
                    count += 1;
                }
                items.push(WordBodyItem::Lengthening(count));
            }
            // Compound marker
            '+' => {
                chars.next();
                items.push(WordBodyItem::CompoundMarker);
            }
            // Stress markers
            '\u{02C8}' => {
                chars.next();
                items.push(WordBodyItem::Stress(StressKind::Primary));
            }
            '\u{02CC}' => {
                chars.next();
                items.push(WordBodyItem::Stress(StressKind::Secondary));
            }
            // Syllable pause
            '^' => {
                chars.next();
                items.push(WordBodyItem::SyllablePause);
            }
            // Clitic boundary
            '~' => {
                chars.next();
                items.push(WordBodyItem::CliticBoundary);
            }
            // Overlap points: ⌈ ⌉ ⌊ ⌋ with optional digit
            '\u{2308}' | '\u{2309}' | '\u{230A}' | '\u{230B}' => {
                let kind = match ch {
                    '\u{2308}' => OverlapKind::TopBegin,
                    '\u{2309}' => OverlapKind::TopEnd,
                    '\u{230A}' => OverlapKind::BottomBegin,
                    '\u{230B}' => OverlapKind::BottomEnd,
                    _ => unreachable!(),
                };
                chars.next();
                // Include the overlap char + optional digit in the slice
                let end = chars.peek().map_or(body.len(), |&(j, _)| j);
                let overlap_text = &body[i..end];
                // Check for trailing digit
                if let Some(&(_, d)) = chars.peek() {
                    if d.is_ascii_digit() && d != '0' {
                        chars.next();
                        let end2 = chars.peek().map_or(body.len(), |&(j, _)| j);
                        items.push(WordBodyItem::OverlapPoint(kind, &body[i..end2]));
                        continue;
                    }
                }
                items.push(WordBodyItem::OverlapPoint(kind, overlap_text));
            }
            // Underline markers
            '\u{0002}' => {
                chars.next();
                if let Some(&(_, next_ch)) = chars.peek() {
                    match next_ch {
                        '\u{0001}' => {
                            chars.next();
                            // Underline begin — not a WordBodyItem, skip for now
                        }
                        '\u{0002}' => {
                            chars.next();
                            // Underline end — not a WordBodyItem, skip for now
                        }
                        _ => {}
                    }
                }
            }
            // CA elements
            _ if is_ca_element(ch) => {
                chars.next();
                items.push(WordBodyItem::CaElement(char_to_ca_element(ch)));
            }
            // CA delimiters
            _ if is_ca_delimiter(ch) => {
                chars.next();
                items.push(WordBodyItem::CaDelimiter(char_to_ca_delimiter(ch)));
            }
            // Text segment: everything else until a special char
            _ => {
                chars.next();
                // Consume all text chars (including '0' in rest position)
                while let Some(&(_, c)) = chars.peek() {
                    if is_body_special_char(c) {
                        break;
                    }
                    chars.next();
                }
                let end = chars.peek().map_or(body.len(), |&(j, _)| j);
                items.push(WordBodyItem::Text(&body[i..end]));
            }
        }
    }
    items
}

/// Characters that break a text segment in word body parsing.
fn is_body_special_char(ch: char) -> bool {
    matches!(
        ch,
        '(' | ':'
            | '+'
            | '^'
            | '~'
            | '\u{02C8}'
            | '\u{02CC}'
            | '\u{2308}'
            | '\u{2309}'
            | '\u{230A}'
            | '\u{230B}'
            | '\u{0002}'
    ) || is_ca_element(ch)
        || is_ca_delimiter(ch)
}

fn is_ca_element(ch: char) -> bool {
    matches!(
        ch,
        '\u{2260}'
            | '\u{223E}'
            | '\u{2051}'
            | '\u{2907}'
            | '\u{2219}'
            | '\u{1F29}'
            | '\u{2193}'
            | '\u{21BB}'
            | '\u{2191}'
            | '\u{2906}'
    )
}

fn is_ca_delimiter(ch: char) -> bool {
    matches!(
        ch,
        '\u{2047}'
            | '\u{00A7}'
            | '\u{204E}'
            | '\u{00B0}'
            | '\u{21AB}'
            | '\u{2206}'
            | '\u{2207}'
            | '\u{222C}'
            | '\u{222E}'
            | '\u{2581}'
            | '\u{2594}'
            | '\u{25C9}'
            | '\u{263A}'
            | '\u{264B}'
            | '\u{03AB}'
    )
}

fn char_to_ca_element(ch: char) -> CaElementKind {
    match ch {
        '\u{2260}' => CaElementKind::BlockedSegments,
        '\u{223E}' => CaElementKind::Constriction,
        '\u{2051}' => CaElementKind::Hardening,
        '\u{2907}' => CaElementKind::HurriedStart,
        '\u{2219}' => CaElementKind::Inhalation,
        '\u{1F29}' => CaElementKind::LaughInWord,
        '\u{2193}' => CaElementKind::PitchDown,
        '\u{21BB}' => CaElementKind::PitchReset,
        '\u{2191}' => CaElementKind::PitchUp,
        '\u{2906}' => CaElementKind::SuddenStop,
        _ => unreachable!("not a CA element char"),
    }
}

fn char_to_ca_delimiter(ch: char) -> CaDelimiterKind {
    match ch {
        '\u{2047}' => CaDelimiterKind::Unsure,
        '\u{00A7}' => CaDelimiterKind::Precise,
        '\u{204E}' => CaDelimiterKind::Creaky,
        '\u{00B0}' => CaDelimiterKind::Softer,
        '\u{21AB}' => CaDelimiterKind::SegmentRepetition,
        '\u{2206}' => CaDelimiterKind::Faster,
        '\u{2207}' => CaDelimiterKind::Slower,
        '\u{222C}' => CaDelimiterKind::Whisper,
        '\u{222E}' => CaDelimiterKind::Singing,
        '\u{2581}' => CaDelimiterKind::LowPitch,
        '\u{2594}' => CaDelimiterKind::HighPitch,
        '\u{25C9}' => CaDelimiterKind::Louder,
        '\u{263A}' => CaDelimiterKind::SmileVoice,
        '\u{264B}' => CaDelimiterKind::BreathyVoice,
        '\u{03AB}' => CaDelimiterKind::Yawn,
        _ => unreachable!("not a CA delimiter char"),
    }
}
