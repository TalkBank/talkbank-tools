//! Main-tier parsing for CHAT utterance lines.
//!
//! This module parses the `*SPK:\t...` tier, including linkers, language markers,
//! content items, terminators, postcodes, and media bullets.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Words>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Terminators>
//! - <https://talkbank.org/0info/manuals/CHAT.html#CA_Overlaps>
//! - <https://talkbank.org/0info/manuals/CHAT.html#CA_Delimiters>

mod annotations;
mod groups;
mod words;

use crate::dependent_tier::classify_dependent_tier_parse_health;
use crate::whitespace::ws_parser;
use chumsky::{error::Rich, prelude::*};
use talkbank_model::ParseOutcome;
use talkbank_model::model::{
    Bullet, LanguageCode, Linker, MainTier, Postcode, SpeakerCode, Terminator, TierContent,
    Utterance, UtteranceContent,
};
use talkbank_model::{ErrorCode, ErrorSink, ParseError, Severity, Span};

use annotations::scoped_annotation_content_parser;
use groups::{
    curly_quotation_parser, group_parser, long_feature_begin_parser, long_feature_end_parser,
    nonvocal_parser, pho_group_parser, sin_group_parser, straight_quotation_parser,
};
use words::{
    action_parser, annotated_event_parser, annotated_word_parser, event_parser, freecode_parser,
    internal_bullet_parser, other_spoken_event_parser, pause_parser, replaced_word_parser,
    standalone_overlap_point_parser, word_content_parser,
};

/// Parse a complete utterance (main tier line, possibly with dependent tiers).
///
/// Utterance in CHAT can be:
/// - Single main tier: "*CHI:\thello world ."
/// - Main tier with dependents: "*CHI:\thello .\n%mor:\tpro:sub|I"
///
/// Parses the main tier (first line starting with *) and all dependent tier lines
/// (subsequent lines starting with %).
pub fn parse_utterance_impl(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<Utterance> {
    // Use split_inclusive so offsets remain exact for both LF and CRLF input.
    let mut lines = input.split_inclusive('\n');
    let main_tier_raw = match lines.next() {
        Some(line) => line,
        None => input,
    };
    let main_tier_line = main_tier_raw.trim_end_matches('\n').trim_end_matches('\r');
    let main_tier = match parse_main_tier_impl(main_tier_line, offset, errors) {
        ParseOutcome::Parsed(main_tier) => main_tier,
        ParseOutcome::Rejected => return ParseOutcome::rejected(),
    };

    let mut utterance = Utterance::new(main_tier);
    let mut current_offset = offset + main_tier_raw.len();

    for raw_line in lines {
        let line = raw_line.trim_end_matches('\n').trim_end_matches('\r');

        if line.trim().is_empty() {
            current_offset += raw_line.len();
            continue;
        }

        if matches!(line.as_bytes().first(), Some(b'%')) {
            if let talkbank_model::ParseOutcome::Parsed(tier) =
                crate::dependent_tier::parse_dependent_tier_impl(line, current_offset, errors)
            {
                utterance.dependent_tiers.push(tier);
            } else if let Some(tier) = classify_dependent_tier_parse_health(line) {
                utterance.mark_parse_taint(tier);
            } else {
                utterance.mark_all_dependent_alignment_taint();
            }
        }

        current_offset += raw_line.len();
    }

    ParseOutcome::parsed(utterance)
}

/// Parse a main tier line using chumsky combinators.
///
/// This is the entry point that integrates with ErrorSink.
///
/// Offset is threaded through all parsers to create document-absolute spans.
pub fn parse_main_tier_impl(
    input: &str,
    offset: usize,
    errors: &impl ErrorSink,
) -> ParseOutcome<MainTier> {
    let input_len = input.len();
    let parser = main_tier_parser(offset, input_len);
    match parser.parse(input).into_result() {
        Ok(main_tier) => ParseOutcome::parsed(main_tier),
        Err(parse_errors) => {
            // Report chumsky errors to ErrorSink
            for err in parse_errors {
                let span = err.span();
                let msg = format!("Parse error: {}", err.reason());
                errors.report(ParseError::from_source_span(
                    ErrorCode::UnparsableContent,
                    Severity::Error,
                    Span::from_usize(span.start + offset, span.end + offset),
                    input,
                    input,
                    msg,
                ));
            }
            ParseOutcome::rejected()
        }
    }
}

// ============================================================================
// Main Tier Parser - Sequential Structure
// ============================================================================
//
// Main tier structure (strict order):
//   *SPEAKER:\t linkers language_code content terminator postcodes bullet
//
// Examples:
//   *CHI:\thello .
//   *CHI:\t++ hello .
//   *CHI:\t[- spa] hola .
//   *CHI:\t++ [- spa] <I want> [/] the dog &=barks (.) fast , okay ? [+ note] 12345_23456
//

/// Main tier parser - parses full sequential structure.
///
/// Structure: *SPEAKER:\t linkers language_code content terminator postcodes bullet
///
/// Offset is threaded through to all sub-parsers for document-absolute spans.
fn main_tier_parser<'a>(
    offset: usize,
    input_len: usize,
) -> impl Parser<'a, &'a str, MainTier, extra::Err<Rich<'a, char>>> {
    // Parse speaker: *CODE:
    let speaker = just('*')
        .ignore_then(none_of(":\t\n\r").repeated().at_least(1).to_slice())
        .then_ignore(just(':'))
        .then_ignore(just('\t'));

    // Parse linkers (++, +<, +^, +", +,, +≋, +≈)
    let linkers = linker_parser()
        .then_ignore(one_of(" \t\u{15}").repeated())
        .repeated()
        .collect::<Vec<_>>();

    // Parse language code: [- spa]
    let language = language_code_parser()
        .then_ignore(one_of(" \t\u{15}").repeated())
        .or_not();

    // Parse content (words, events, pauses, groups, etc.)
    //
    // CA MODE SUPPORT: Overlap markers can be adjacent to any content (no whitespace required).
    // Other content items require whitespace between them.
    // This matches tree-sitter behavior where overlap_point is a lexical token.
    let content = content_with_adjacent_overlaps(offset);

    // Parse terminator (. ? ! +... etc.)
    let terminator = ws_parser()
        .or_not()
        .ignore_then(terminator_parser().or_not());

    // Parse postcodes: [+ note]
    let postcodes = ws_parser()
        .ignore_then(postcode_parser())
        .repeated()
        .collect::<Vec<_>>();

    // Parse bullet: 12345_23456
    let bullet = ws_parser().ignore_then(bullet_parser()).or_not();

    // Combine all parts sequentially
    speaker
        .then(linkers)
        .then(language)
        .then(content)
        .then(terminator)
        .then(postcodes)
        .then(bullet)
        .map_with(
            move |((((((speaker_code, linkers), lang), content), term), postcodes), bullet),
                  _extra| {
                let tier_content =
                    TierContent::with_all(linkers, lang, content, term, postcodes, bullet);

                // CRITICAL: Apply offset to create document-absolute spans
                // Use input_len for the end to match TreeSitterParser behavior
                let source_span = Span::from_usize(offset, offset + input_len);

                // For speaker_span, calculate where the speaker code is
                // Format: *CODE:
                // Speaker code starts after '*' (position 1) and ends before ':'
                let speaker_text = speaker_code;
                let speaker_start = offset + 1; // After the '*'
                let speaker_end = speaker_start + speaker_text.len();
                let speaker_span = Span::from_usize(speaker_start, speaker_end);

                MainTier {
                    speaker: SpeakerCode::new(speaker_code),
                    content: tier_content,
                    span: source_span,
                    speaker_span,
                }
            },
        )
}

// ============================================================================
// Component Parsers
// ============================================================================

/// Parse content sequence where overlap markers can be adjacent to ANY following content.
///
/// Key insight: `.separated_by()` with flexible separator that accepts no whitespace
/// Parse content sequence allowing overlap markers to be adjacent to any content.
///
/// CA corpora frequently have patterns like `⌊mais` (overlap marker immediately adjacent to word).
/// Rule: Overlap markers can be adjacent to anything, but non-overlap content
/// requires whitespace between items (except when adjacent to overlap markers).
///
/// IMPORTANT: \u{15} is used as bullet delimiter, not as whitespace in content.
/// Only space and tab are whitespace for content separation.
fn content_with_adjacent_overlaps<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, Vec<UtteranceContent>, extra::Err<Rich<'a, char>>> {
    let core = utterance_content_parser(offset).boxed();

    // CRITICAL CHANGE: Overlap markers should ONLY appear within words, not standalone!
    // The word parser handles overlap markers at any position (start, middle, end).
    // We only need separator_then_core for CA separators that can appear standalone.
    let separator_then_core = separator_parser()
        .map(UtteranceContent::Separator)
        .then(ws_parser().or_not())
        .then(core.clone())
        .map(|((sep, _), item)| vec![sep, item])
        .boxed();
    let core_only = core.clone().map(|item| vec![item]);
    let separator_only = separator_parser()
        .map(UtteranceContent::Separator)
        .map(|sep| vec![sep])
        .boxed();

    // Segment choice: try core first (which includes word parser that handles overlap markers)
    let segment = choice((
        separator_then_core.clone(),
        core_only.clone(),
        separator_only.clone(),
    ))
    .boxed();
    let segment_no_ws = choice((separator_then_core, core_only, separator_only));

    ws_parser()
        .or_not()
        .ignore_then(segment.clone())
        .then(
            choice((ws_parser().ignore_then(segment), segment_no_ws))
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(|(first, rest)| {
            let mut items = Vec::new();
            items.extend(first);
            for chunk in rest {
                items.extend(chunk);
            }
            items
        })
}

/// Parse utterance content (words, events, pauses, groups, etc.)
/// This is the complex part - 24 UtteranceContent types
///
/// CRITICAL ORDER: Words MUST be parsed before standalone overlap markers!
/// Words can contain overlap markers (e.g., "ye⌉2⌊3:s"), so we need to try
/// the word parser first. Only if it fails should we try standalone overlap markers.
fn utterance_content_parser<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, UtteranceContent, extra::Err<Rich<'a, char>>> {
    choice((
        // Internal bullets (must come before pause parser which also uses parentheses)
        internal_bullet_parser().map(UtteranceContent::InternalBullet),
        // Annotated variants MUST come before bare variants (try annotated first, then bare)
        scoped_annotation_content_parser(),
        annotated_event_parser().map(UtteranceContent::AnnotatedEvent),
        annotated_word_parser(offset).map(|aw| UtteranceContent::AnnotatedWord(Box::new(aw))),
        // Standalone underline markers - match ONLY when followed by whitespace
        // When attached to word (e.g., \u{2}\u{1}text), they're parsed as WordContent by word parser
        // When standalone (e.g., ` \u{2}\u{1} `), they're UtteranceContent::UnderlineBegin/End
        //
        // Lookahead: peek at next char, succeed only if it's whitespace
        just("\u{0002}\u{0001}")
            .then(one_of(" \t").rewind()) // lookahead: must be followed by space/tab
            .map_with(move |_, extra| {
                let span: chumsky::span::SimpleSpan = extra.span();
                let source_span = Span::from_usize(span.start + offset, span.end + offset);
                UtteranceContent::UnderlineBegin(talkbank_model::UnderlineMarker::from_span(
                    source_span,
                ))
            }),
        just("\u{0002}\u{0002}")
            .then(one_of(" \t.?!\n\r").rewind()) // lookahead: must be followed by ws/terminator
            .map_with(move |_, extra| {
                let span: chumsky::span::SimpleSpan = extra.span();
                let source_span = Span::from_usize(span.start + offset, span.end + offset);
                UtteranceContent::UnderlineEnd(talkbank_model::UnderlineMarker::from_span(
                    source_span,
                ))
            }),
        // Phase 2: Core content types
        other_spoken_event_parser().map(UtteranceContent::OtherSpokenEvent), // &*SPEAKER:text (before &=)
        event_parser().map(UtteranceContent::Event), // Must come before word (starts with &)
        pause_parser().map(UtteranceContent::Pause), // Must come before word (starts with ()
        // Phase 3: Groups and annotations
        group_parser(offset), // Must come before word (starts with <), handles both bare and annotated
        // Phase 4: Special groups
        pho_group_parser(offset).map(UtteranceContent::PhoGroup), // ‹...›
        sin_group_parser(offset).map(UtteranceContent::SinGroup), // 〔...〕
        // Quotations - both curly and straight quotes
        curly_quotation_parser(offset).map(UtteranceContent::Quotation), // \u{201C}...\u{201D}
        straight_quotation_parser(offset).map(UtteranceContent::Quotation), // "..."
        // Phase 5: Scope markers
        long_feature_begin_parser().map(UtteranceContent::LongFeatureBegin), // &{l=LABEL
        long_feature_end_parser().map(UtteranceContent::LongFeatureEnd),     // &}l=LABEL
        nonvocal_parser(), // &{n=LABEL or &{n=LABEL}
        // Phase 6: Remaining types
        freecode_parser().map(UtteranceContent::Freecode), // [^ text]
        replaced_word_parser(offset).map(UtteranceContent::ReplacedWord), // word [: replacement]
        // Phase 7: Standalone overlap points - must come BEFORE word parser!
        // Pattern: ⌈ word (overlap marker followed by whitespace) or ⌈2 word (with index)
        // These are standalone utterance content, not part of a word.
        // Word-embedded overlap markers (like "ye⌉2⌊3:s") are handled by word parser.
        standalone_overlap_point_parser(),
        // Phase 8: Words - try word parser after standalone overlap points
        // Words can contain overlap markers (e.g., "ye⌉2⌊3:s"), so word parser handles them
        word_content_parser(offset).map(|word| UtteranceContent::Word(Box::new(word))),
        action_parser()
            .boxed()
            .map(UtteranceContent::AnnotatedAction),
        // Fix 3: Catch-all — preserve any unparseable token as a raw Word.
        //
        // This arm fires only when all parsers above fail (e.g., an unrecognized
        // character sequence that is not whitespace, a terminator, or a known
        // content type).
        //
        // EXCLUSION RATIONALE:
        // - Whitespace (' ' '\t' '\n' '\r' '\u{15}'): content item delimiters
        // - Terminator chars (. ? ! ≋ ≈): handled by terminator_parser after content
        // - CA intonation separators (⇗ ↗ → ↘ ⇘): handled by separator_parser
        // - Bracket/quote delimiters ([ " \u{201C} \u{201D} ,): handled by bracket/annotation parsers
        // - '+': starts compound terminators (+... +/. +//. etc.); must NOT be consumed
        //   as content, or the terminator parser can't match the full +xxx sequence
        // - ';' ':': separator characters handled by separator_only in
        //   content_with_adjacent_overlaps; if catch-all consumed them, separator_only
        //   would never fire (core_only catches first in the choice ordering)
        // - '∞' '„' '‡' '≡': CA separator characters in WORD_SEGMENT_FORBIDDEN_COMMON;
        //   not in token_char exclusion but must be left for separator_only
        //
        // This is defense-in-depth: Fixes 1+2 handle malformed words that DO start
        // with a valid word-start character. This catch-all handles tokens whose
        // FIRST character fails word_start_char (e.g., standalone digits 1-9,
        // or unrecognized Unicode sequences).
        any()
            .filter(|&c: &char| {
                !matches!(
                    c,
                    // Whitespace and special delimiters
                    ' ' | '\t'
                        | '\n'
                        | '\r'
                        | '\u{0015}'
                        // Bracket/quote delimiters (bracket parsers, annotation parsers)
                        | '['
                        | '"'
                        | '\u{201C}'
                        | '\u{201D}'
                        | ','
                        // Terminator characters
                        | '.'
                        | '?'
                        | '!'
                        | '≋'
                        | '≈'
                        // CA intonation separators (separator_parser)
                        | '⇗'
                        | '↗'
                        | '→'
                        | '↘'
                        | '⇘'
                        // Terminator prefix: + starts +... +/. +//. +!? etc.
                        // Must NOT be consumed so terminator_parser can match the full sequence.
                        | '+'
                        // Separator characters handled by separator_only in
                        // content_with_adjacent_overlaps (not in token_char exclusion
                        // but must not shadow separator_only via core_only catch-all)
                        | ';'
                        | ':'
                        | '∞'  // U+221E — UnmarkedEnding separator
                        | '„'  // U+201E — Tag separator
                        | '‡'  // U+2021 — Vocative separator
                        | '≡' // U+2261 — Uptake separator
                )
            })
            .repeated()
            .at_least(1)
            .to_slice()
            .map_with(move |raw: &str, extra| {
                let span: chumsky::span::SimpleSpan = extra.span();
                let source_span = Span::from_usize(span.start + offset, span.end + offset);
                tracing::debug!(token = raw, "Content item fallback: preserving as raw word");
                UtteranceContent::Word(Box::new(
                    talkbank_model::model::Word::new_unchecked(raw, raw).with_span(source_span),
                ))
            }),
    ))
}

/// Parse a linker: ++, +<, +^, +", +,
fn linker_parser<'a>() -> impl Parser<'a, &'a str, Linker, extra::Err<Rich<'a, char>>> {
    choice((
        just("++").to(Linker::OtherCompletion),
        just("+<").to(Linker::LazyOverlapPrecedes),
        just("+^").to(Linker::QuickUptakeOverlap),
        just("+\"").to(Linker::QuotationFollows),
        just("+,").to(Linker::SelfCompletion),
        just("+≋").to(Linker::TcuContinuation),
        just("+≈").to(Linker::NoBreakTcuContinuation),
    ))
}

/// Parse a language code: [- spa]
fn language_code_parser<'a>() -> impl Parser<'a, &'a str, LanguageCode, extra::Err<Rich<'a, char>>>
{
    just("[- ")
        .ignore_then(none_of(']').repeated().at_least(1).to_slice())
        .then_ignore(just(']'))
        .map(LanguageCode::new)
}

/// Parse a terminator: . ? ! +... +/. +//. and CA break markers (≋ ≈)
///
/// NOTE: CA intonation markers (⇗ ↗ → ↘ ⇘) are parsed as SEPARATORS, not terminators.
/// Only ≋ and ≈ are actual terminators.
fn terminator_parser<'a>() -> impl Parser<'a, &'a str, Terminator, extra::Err<Rich<'a, char>>> {
    choice((
        // Interruption terminators (order matters - longest first!)
        just("+//.").to(Terminator::SelfInterruption { span: Span::DUMMY }),
        just("+//?").to(Terminator::SelfInterruptedQuestion { span: Span::DUMMY }),
        just("+\"/.").to(Terminator::QuotedNewLine { span: Span::DUMMY }),
        just("+/.").to(Terminator::Interruption { span: Span::DUMMY }),
        just("+/?").to(Terminator::InterruptedQuestion { span: Span::DUMMY }),
        just("+/??").to(Terminator::BrokenQuestion { span: Span::DUMMY }),
        just("+...").to(Terminator::TrailingOff { span: Span::DUMMY }),
        just("+..!").to(Terminator::TrailingOffQuestion { span: Span::DUMMY }),
        just("+..?").to(Terminator::TrailingOffQuestion { span: Span::DUMMY }),
        just("+..").to(Terminator::TrailingOff { span: Span::DUMMY }),
        just("+!?").to(Terminator::BrokenQuestion { span: Span::DUMMY }),
        just("+\".").to(Terminator::QuotedPeriodSimple { span: Span::DUMMY }),
        just("+.").to(Terminator::BreakForCoding { span: Span::DUMMY }),
        // CA mode terminators with linkers (+ prefix)
        just("+≋").to(Terminator::CaTechnicalBreakLinker { span: Span::DUMMY }),
        just("+≈").to(Terminator::CaNoBreakLinker { span: Span::DUMMY }),
        // CA mode break terminators (bare symbols)
        // NOTE: ⇗ ↗ → ↘ ⇘ are SEPARATORS, not terminators
        just("≋").to(Terminator::CaTechnicalBreak { span: Span::DUMMY }), // U+224B
        just("≈").to(Terminator::CaNoBreak { span: Span::DUMMY }),        // U+2248
        // Standard terminators
        just('.').to(Terminator::Period { span: Span::DUMMY }),
        just('?').to(Terminator::Question { span: Span::DUMMY }),
        just('!').to(Terminator::Exclamation { span: Span::DUMMY }),
    ))
}

/// Parse a postcode: [+ note]
fn postcode_parser<'a>() -> impl Parser<'a, &'a str, Postcode, extra::Err<Rich<'a, char>>> {
    just("[+ ")
        .ignore_then(none_of(']').repeated().at_least(1).to_slice())
        .then_ignore(just(']'))
        .map(Postcode::new)
}

/// Parse a separator: , ; : and CA intonation markers (⇗ ↗ → ↘ ⇘)
///
/// CA intonation markers appear mid-tier before terminators like ≋ or ≈.
/// Example: `*CHI: rising ⇗≋` has separator(⇗) + terminator(≋)
fn separator_parser<'a>()
-> impl Parser<'a, &'a str, talkbank_model::model::Separator, extra::Err<Rich<'a, char>>> {
    choice((
        // Basic separators
        just(',').to(talkbank_model::model::Separator::Comma { span: Span::DUMMY }),
        just(';').to(talkbank_model::model::Separator::Semicolon { span: Span::DUMMY }),
        just(':').to(talkbank_model::model::Separator::Colon { span: Span::DUMMY }),
        // CA continuation marker [^c]
        just("[^c]").to(talkbank_model::model::Separator::CaContinuation { span: Span::DUMMY }),
        // CA intonation markers (tree-sitter treats these as separators, not terminators)
        just("⇗").to(talkbank_model::model::Separator::RisingToHigh { span: Span::DUMMY }),
        just("↗").to(talkbank_model::model::Separator::RisingToMid { span: Span::DUMMY }),
        just("→").to(talkbank_model::model::Separator::Level { span: Span::DUMMY }),
        just("↘").to(talkbank_model::model::Separator::FallingToMid { span: Span::DUMMY }),
        just("⇘").to(talkbank_model::model::Separator::FallingToLow { span: Span::DUMMY }),
        // Other CA separators
        just("„").to(talkbank_model::model::Separator::Tag { span: Span::DUMMY }), // U+201E
        just("‡").to(talkbank_model::model::Separator::Vocative { span: Span::DUMMY }), // U+2021
        just("∞").to(talkbank_model::model::Separator::UnmarkedEnding { span: Span::DUMMY }), // U+221E
        just("≡").to(talkbank_model::model::Separator::Uptake { span: Span::DUMMY }), // U+2261
    ))
}

/// Parse a bullet: 12345_23456
/// Bullets may be wrapped in \u{15} delimiter characters by tree-sitter serialization
fn bullet_parser<'a>() -> impl Parser<'a, &'a str, Bullet, extra::Err<Rich<'a, char>>> {
    // Optional opening delimiter
    just('\u{15}')
        .or_not()
        .ignore_then(one_of("0123456789").repeated().at_least(1).to_slice())
        .then_ignore(just('_'))
        .then(one_of("0123456789").repeated().at_least(1).to_slice())
        .then(just('-').or_not()) // Optional trailing dash for skip marker
        .then_ignore(just('\u{15}').or_not()) // Optional closing delimiter
        .try_map(
            |((start, end), has_dash): ((&str, &str), Option<char>), span: SimpleSpan| {
                let start_ms = start
                    .parse::<u32>()
                    .map_err(|_| Rich::custom(span, "Invalid bullet start time"))?;
                let end_ms = end
                    .parse::<u32>()
                    .map_err(|_| Rich::custom(span, "Invalid bullet end time"))?;
                let mut bullet = Bullet::new(start_ms.into(), end_ms.into());
                if has_dash.is_some() {
                    bullet = bullet.with_skip(true);
                }
                Ok(bullet)
            },
        )
}

#[cfg(test)]
mod tests {
    use super::parse_main_tier_impl;
    use talkbank_model::ErrorCollector;
    use talkbank_model::model::{ScopedAnnotation, UtteranceContent, WordCategory};

    /// Tests preserves duration annotations.
    #[test]
    fn preserves_duration_annotations() -> Result<(), String> {
        let errors = ErrorCollector::new();
        let main = parse_main_tier_impl("*CHI:\t<a b> [# 2:3.4] .", 0, &errors)
            .ok_or_else(|| "main tier should parse".to_string())?;
        let first = main
            .content
            .content
            .first()
            .ok_or_else(|| "first content item exists".to_string())?;
        let UtteranceContent::AnnotatedGroup(group) = first else {
            return Err("Expected first content item to be an annotated group".to_string());
        };
        let duration = group
            .scoped_annotations
            .iter()
            .find_map(|ann| match ann {
                ScopedAnnotation::Duration(d) => Some(d.time.as_str()),
                _ => None,
            })
            .ok_or_else(|| "duration annotation should exist".to_string())?;
        assert_eq!(duration, "2:3.4");
        Ok(())
    }

    /// Tests parses nonword amp tilde in main tier.
    #[test]
    fn parses_nonword_amp_tilde_in_main_tier() -> Result<(), String> {
        let errors = ErrorCollector::new();
        let main = parse_main_tier_impl(
            "*CHI:\tfoo &~word\u{2191} &~ba\u{2191}r stuff .",
            0,
            &errors,
        )
        .ok_or_else(|| "main tier should parse".to_string())?;
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.to_vec()
        );

        let words: Vec<_> = main
            .content
            .content
            .iter()
            .filter_map(|item| match item {
                UtteranceContent::Word(word) => Some(word),
                _ => None,
            })
            .collect();
        assert_eq!(words.len(), 4);
        assert_eq!(words[1].category, Some(WordCategory::Nonword));
        assert_eq!(words[2].category, Some(WordCategory::Nonword));
        Ok(())
    }

    /// Tests parses amp category variants in main tier.
    #[test]
    fn parses_amp_category_variants_in_main_tier() -> Result<(), String> {
        let errors = ErrorCollector::new();
        let main = parse_main_tier_impl("*CHI:\t&-filled &+frag &~babble .", 0, &errors)
            .ok_or_else(|| "main tier should parse".to_string())?;
        assert!(
            errors.is_empty(),
            "unexpected parse errors: {:?}",
            errors.to_vec()
        );

        let words: Vec<_> = main
            .content
            .content
            .iter()
            .filter_map(|item| match item {
                UtteranceContent::Word(word) => Some(word),
                _ => None,
            })
            .collect();
        assert_eq!(words.len(), 3);
        assert_eq!(words[0].category, Some(WordCategory::Filler));
        assert_eq!(words[1].category, Some(WordCategory::PhonologicalFragment));
        assert_eq!(words[2].category, Some(WordCategory::Nonword));
        Ok(())
    }

    /// Verifies that a malformed word (one that `parse_word_impl` rejects) is
    /// recovered as raw text instead of aborting the entire main tier parse.
    /// This is the core behavior added by Phase 0 Fix 1.
    #[test]
    fn test_malformed_word_recovers_as_raw_text() -> Result<(), String> {
        let errors = ErrorCollector::new();
        // `he(llo` has an unmatched `(` — the word parser tries to start a
        // shortening but never finds `)`, so it only matches `he` and rejects
        // due to leftover input. The fallback should recover it as raw text.
        let main = parse_main_tier_impl("*CHI:\thello he(llo world .", 0, &errors)
            .ok_or_else(|| "main tier should parse despite malformed word".to_string())?;

        let words: Vec<_> = main
            .content
            .content
            .iter()
            .filter_map(|item| match item {
                UtteranceContent::Word(word) => Some(word),
                _ => None,
            })
            .collect();

        // All three words should be present: hello, he(llo, world
        assert_eq!(
            words.len(),
            3,
            "Expected 3 words (including recovered malformed word), got {}",
            words.len()
        );
        // The malformed word should have its raw text preserved
        assert_eq!(words[1].raw_text(), "he(llo");
        Ok(())
    }

    /// Test that a content token whose first character fails word_start_char
    /// is recovered by the Fix 3 catch-all arm instead of aborting the main tier.
    ///
    /// Standalone digits 1-9 fail word_start_char (tree-sitter policy: only 0 is
    /// allowed as a bare digit word). The catch-all preserves them as raw Words
    /// so the utterance continues to parse. This is the core behavior added by
    /// Phase 0 Fix 3.
    #[test]
    fn test_catchall_content_item_recovers_as_raw_word() -> Result<(), String> {
        let errors = ErrorCollector::new();
        // `5` fails word_start_char (digits 1-9 are forbidden), so all specific
        // parsers reject it. The catch-all should preserve it as a raw Word.
        let main = parse_main_tier_impl("*CHI:\thello 5 world .", 0, &errors)
            .ok_or_else(|| "main tier should parse despite digit content token".to_string())?;

        let words: Vec<_> = main
            .content
            .content
            .iter()
            .filter_map(|item| match item {
                UtteranceContent::Word(word) => Some(word),
                _ => None,
            })
            .collect();

        // All three words should be present: hello, 5 (catch-all), world
        assert_eq!(
            words.len(),
            3,
            "Expected 3 words (including catch-all recovered token), got {}",
            words.len()
        );
        assert_eq!(words[1].raw_text(), "5");
        Ok(())
    }
}
