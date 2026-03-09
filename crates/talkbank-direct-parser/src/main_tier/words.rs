//! Word-level and word-adjacent content parsers for main-tier CHAT utterances.
//!
//! This module handles individual words, events, pauses, overlap points,
//! replaced words, freecodes, internal bullets, and annotated variants.

use super::annotations::scoped_annotation_parser;
use crate::whitespace::ws_parser;
use chumsky::{error::Rich, prelude::*};
use talkbank_model::ParseOutcome;
use talkbank_model::Span;
use talkbank_model::model::{
    Action, OverlapIndex, OverlapPoint, OverlapPointKind, PauseDuration, PauseTimedDuration,
    UtteranceContent, Word,
};

/// Parse a word (delegate to word parser, with special cases)
///
/// NOTE: Overlap markers (⌈⌉⌊⌋) are NOT excluded here because they can appear
/// INSIDE words (e.g., "ye⌉2⌊3:s"). The word parser handles them as WordContent.
/// Only standalone overlap markers (not part of a word) are parsed separately
/// by overlap_point_parser().
pub(super) fn word_content_parser<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, Word, extra::Err<Rich<'a, char>>> {
    let token_char = |c: char| {
        !matches!(
            c,
            ' ' | '\t'
                | '\n'
                | '\r'
                | '\u{15}'
                | '['
                | '"'
                | '\u{201C}'
                | '\u{201D}'
                | ','
                // Terminators - MUST be excluded so words stop before terminators
                | '.'
                | '?'
                | '!'
                // CA intonation markers and terminators
                | '⇗'
                | '↗'
                | '→'
                | '↘'
                | '⇘'
                | '≋'
                | '≈'
        )
    };
    let omit_word = just('0')
        .then(any().filter(move |&c: &char| token_char(c)).repeated())
        .to_slice();
    let word_start_char = |c: char| {
        if matches!(c, '.' | '?' | '!') {
            return false;
        }
        crate::word::is_word_start_char(c)
            // Category-prefixed words (&~, &-, &+) must be routable from main-tier tokenization.
            // The word parser validates the full marker sequence; this only enables dispatch.
            || c == '&'
            // CA markers are registry-governed; keep dispatch aligned with generated symbol sets.
            || crate::word::is_ca_marker_char(c)
            || matches!(
                c,
                'ˈ'
                    | 'ˌ'
                    // CRITICAL: Overlap markers MUST be allowed to start words!
                    // Words like ⌉world should be parsed as a single Word, not split.
                    | '⌈'
                    | '⌉'
                    | '⌊'
                    | '⌋'
                    // NOTE: „ ‡ ∞ are SEPARATORS, not word characters - handled by separator_parser
                    // CRITICAL: ( can start a word for standalone shortenings like (parens)
                    // or word-initial shortenings like (t)a. The word parser handles
                    // shortenings as WordContent::Shortening.
                    | '('
            )
            || c == '\u{0002}'
    };
    omit_word
        .or(any()
            .filter(move |&c: &char| word_start_char(c))
            .then(any().filter(move |&c: &char| token_char(c)).repeated())
            .to_slice())
        .try_map(move |word_text: &str, span: chumsky::span::SimpleSpan| {
            if word_text.is_empty() {
                return Err(Rich::custom(span, String::from("Cannot parse empty word")));
            }

            // Reject patterns that are terminators, not words
            // NOTE: ⇗ ↗ → ↘ ⇘ are SEPARATORS, not terminators - don't reject them here
            let is_terminator = matches!(
                word_text,
                "+" | "+/"
                    | "+\""
                    | "+/."
                    | "+/?"
                    | "+//."
                    | "+//?."
                    | "+\"."
                    | "+\"/."
                    | "+..."
                    | "+..?"
                    | "+."
                    | "+//"
                    | "+\"/"
                    | "+//?"
                    | "+≋"
                    | "+≈"
                    | "."
                    | "?"
                    | "!"
                    | "≋"
                    | "≈"
            );
            if is_terminator {
                return Err(Rich::custom(
                    span,
                    format!("'{}' is a terminator pattern, not a word", word_text),
                ));
            }

            // Create span with offset applied
            let _word_span = Span::from_usize(span.start + offset, span.end + offset);

            if word_text == "0" {
                return Err(Rich::custom(span, "Action marker parsed separately"));
            }

            // NOTE: „ ‡ ∞ are now handled as Separators by separator_parser, not as words

            let inner_errors = talkbank_model::ErrorCollector::new();
            match crate::word::parse_word_impl(word_text, offset + span.start, &inner_errors) {
                ParseOutcome::Parsed(word) => Ok(word),
                ParseOutcome::Rejected => {
                    // Recovery: preserve raw text as a Word instead of killing the file.
                    // Mirrors tree-sitter parser's recover_error_as_word() pattern.
                    // Word-level parse errors are logged; validation can still flag the word.
                    tracing::debug!(
                        word = word_text,
                        "Word parse recovery: preserving as raw text"
                    );
                    Ok(Word::new_unchecked(word_text, word_text)
                        .with_span(Span::from_usize(offset + span.start, offset + span.end)))
                }
            }
        })
}

/// Parse action marker (bare `0`) as AnnotatedAction (no annotations).
pub(super) fn action_parser<'a>()
-> impl Parser<'a, &'a str, talkbank_model::model::Annotated<Action>, extra::Err<Rich<'a, char>>> {
    just('0')
        .then(
            ws_parser()
                .ignore_then(scoped_annotation_parser())
                .repeated()
                .collect::<Vec<_>>()
                .or_not(),
        )
        .map(|(_, annotations_opt)| {
            let action = talkbank_model::model::Annotated::new(Action::new());
            if let Some(annotations) = annotations_opt {
                action.with_scoped_annotations(annotations)
            } else {
                action
            }
        })
}

/// Parse an annotated word: word followed by scoped annotations (with intervening whitespace)
pub(super) fn annotated_word_parser<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, talkbank_model::model::Annotated<Word>, extra::Err<Rich<'a, char>>> {
    word_content_parser(offset)
        .then(
            // Annotations follow word with whitespace in between
            // (whitespace is part of the annotation unit, not consumed by separated_by)
            ws_parser()
                .ignore_then(scoped_annotation_parser())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|(word, annotations)| {
            talkbank_model::model::Annotated::new(word).with_scoped_annotations(annotations)
        })
}

/// Parse an annotated event: event followed by scoped annotations (with intervening whitespace)
pub(super) fn annotated_event_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    talkbank_model::model::Annotated<talkbank_model::model::Event>,
    extra::Err<Rich<'a, char>>,
> {
    event_parser()
        .then(
            // Annotations follow event with whitespace in between
            ws_parser()
                .ignore_then(scoped_annotation_parser())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|(event, annotations)| {
            talkbank_model::model::Annotated::new(event).with_scoped_annotations(annotations)
        })
}

/// Parse other spoken event: &*SPEAKER:text
pub(super) fn other_spoken_event_parser<'a>()
-> impl Parser<'a, &'a str, talkbank_model::model::OtherSpokenEvent, extra::Err<Rich<'a, char>>> {
    just("&*")
        .ignore_then(
            // Parse speaker code (letters/digits until colon)
            none_of(": \t\n\r\u{15}").repeated().at_least(1).to_slice(),
        )
        .then_ignore(just(':'))
        .then(
            // Parse text (until whitespace or end)
            none_of(" \t\n\r\u{15}").repeated().at_least(1).to_slice(),
        )
        .map(|(speaker, text)| talkbank_model::model::OtherSpokenEvent::new(speaker, text))
}

/// Parse an event: &=action
pub(super) fn event_parser<'a>()
-> impl Parser<'a, &'a str, talkbank_model::model::Event, extra::Err<Rich<'a, char>>> {
    just("&=")
        .ignore_then(
            none_of(" \t\n\r\u{15}") // Handle various whitespace
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .map(talkbank_model::model::Event::new)
}

/// Parse a pause: (.) (..) (...) (1.5) (7:1.5) (3.)
///
/// IMPORTANT: This parser ONLY matches pause patterns, NOT word shortenings like (t)a
/// or CA uncertain transcriptions like (parens).
/// Word shortenings contain letters inside parentheses and are handled by the word parser.
/// Pause patterns ONLY contain dots and/or digits.
///
/// Key insight: Use lookahead after '(' to ensure the content starts with '.' or digit.
/// This prevents the parser from partially matching and then failing on letter content.
pub(super) fn pause_parser<'a>()
-> impl Parser<'a, &'a str, talkbank_model::model::Pause, extra::Err<Rich<'a, char>>> {
    // Digits parser for timed pauses
    let digits = || one_of("0123456789").repeated().at_least(1).to_slice();

    // Timed pause with colon format: "7:1." or "7:1.5"
    let colon_timed = digits()
        .then_ignore(just(':'))
        .then(digits())
        .then_ignore(just('.'))
        .then(one_of("0123456789").repeated().to_slice().or_not())
        .to_slice()
        .map(|text| {
            talkbank_model::model::Pause::new(PauseDuration::Timed(PauseTimedDuration::new(text)))
        });

    // CA timed pause format: (3.) - digit followed by dot (no fractional part)
    // This is a CA convention for timing in seconds
    let ca_timed = digits().then_ignore(just('.')).to_slice().map(|text| {
        talkbank_model::model::Pause::new(PauseDuration::Timed(PauseTimedDuration::new(text)))
    });

    // Timed pause decimal format: "1.5" or "0.5" (digits, dot, digits)
    let decimal_timed = digits()
        .then_ignore(just('.'))
        .then(digits())
        .to_slice()
        .map(|text| {
            talkbank_model::model::Pause::new(PauseDuration::Timed(PauseTimedDuration::new(text)))
        });

    // Timed pause integer format: just digits like "2" for 2 seconds
    let integer_timed = digits().map(|text| {
        talkbank_model::model::Pause::new(PauseDuration::Timed(PauseTimedDuration::new(text)))
    });

    // Symbolic pauses: ..., .., .
    let symbolic = choice((
        just("...").to(talkbank_model::model::Pause::new(PauseDuration::Long)),
        just("..").to(talkbank_model::model::Pause::new(PauseDuration::Medium)),
        just(".").to(talkbank_model::model::Pause::new(PauseDuration::Short)),
    ));

    // CRITICAL: Use lookahead after '(' to ensure pause content (dots or digits).
    // This prevents partial matching on (letter...) patterns like (parens) or (t)a.
    // The lookahead checks that the char after '(' is either '.' or a digit.
    just('(')
        .then(one_of(".0123456789").rewind()) // Lookahead: must start with . or digit
        .ignore_then(choice((
            colon_timed,
            decimal_timed,
            ca_timed,
            integer_timed,
            symbolic,
        )))
        .then_ignore(just(')'))
}

/// Parse a standalone overlap point: ⌈, ⌉, ⌊, ⌋ with optional index.
///
/// Standalone overlap points appear when the marker is followed by whitespace,
/// e.g., `⌈ word` or `⌈2 word`. When the marker is embedded in a word (like
/// `ye⌉2⌊3:s`), it's parsed by the word parser instead.
///
/// This parser uses lookahead to only match when followed by whitespace.
pub(super) fn standalone_overlap_point_parser<'a>()
-> impl Parser<'a, &'a str, UtteranceContent, extra::Err<Rich<'a, char>>> {
    // Parse optional index (digits only)
    let idx = || {
        one_of("123456789").repeated().to_slice().map(|s: &str| {
            if s.is_empty() {
                None
            } else {
                s.parse::<u32>().ok().map(OverlapIndex::new)
            }
        })
    };

    // Lookahead for whitespace - standalone overlap points must be followed by space/tab
    let ws_lookahead = || one_of(" \t").rewind();

    choice((
        just("⌈")
            .ignore_then(idx())
            .then_ignore(ws_lookahead())
            .map(|index| {
                UtteranceContent::OverlapPoint(OverlapPoint::new(
                    OverlapPointKind::TopOverlapBegin,
                    index,
                ))
            }),
        just("⌉")
            .ignore_then(idx())
            .then_ignore(ws_lookahead())
            .map(|index| {
                UtteranceContent::OverlapPoint(OverlapPoint::new(
                    OverlapPointKind::TopOverlapEnd,
                    index,
                ))
            }),
        just("⌊")
            .ignore_then(idx())
            .then_ignore(ws_lookahead())
            .map(|index| {
                UtteranceContent::OverlapPoint(OverlapPoint::new(
                    OverlapPointKind::BottomOverlapBegin,
                    index,
                ))
            }),
        just("⌋")
            .ignore_then(idx())
            .then_ignore(ws_lookahead())
            .map(|index| {
                UtteranceContent::OverlapPoint(OverlapPoint::new(
                    OverlapPointKind::BottomOverlapEnd,
                    index,
                ))
            }),
    ))
}

/// Parse an internal bullet: 12345_23456 (wrapped in \u{15} delimiters)
/// These appear mid-tier between content items, unlike terminal bullets
pub(super) fn internal_bullet_parser<'a>()
-> impl Parser<'a, &'a str, talkbank_model::model::Bullet, extra::Err<Rich<'a, char>>> {
    just('\u{15}')
        .ignore_then(
            one_of("0123456789")
                .repeated()
                .at_least(1)
                .to_slice()
                .then_ignore(just('_'))
                .then(one_of("0123456789").repeated().at_least(1).to_slice())
                .then(just('-').or_not()), // Optional trailing dash
        )
        .then_ignore(just('\u{15}'))
        .try_map(
            |((start_str, end_str), has_dash): ((&str, &str), Option<char>), span: SimpleSpan| {
                let start: u32 = start_str
                    .parse()
                    .map_err(|_| Rich::custom(span, "Invalid start number"))?;
                let end: u32 = end_str
                    .parse()
                    .map_err(|_| Rich::custom(span, "Invalid end number"))?;

                let mut bullet = talkbank_model::model::Bullet::new(start.into(), end.into());
                if has_dash.is_some() {
                    bullet = bullet.with_skip(true);
                }
                Ok(bullet)
            },
        )
}

/// Parse a freecode: [^ comment]
pub(super) fn freecode_parser<'a>()
-> impl Parser<'a, &'a str, talkbank_model::model::Freecode, extra::Err<Rich<'a, char>>> {
    just("[^ ")
        .ignore_then(none_of(']').repeated().at_least(1).to_slice())
        .then_ignore(just(']'))
        .map(talkbank_model::model::Freecode::new)
}

/// Parse a replaced word: hello [: world]  or  hello [: good morning] [*]
pub(super) fn replaced_word_parser<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, Box<talkbank_model::model::ReplacedWord>, extra::Err<Rich<'a, char>>>
{
    // Parse original word
    word_content_parser(offset)
        .then_ignore(ws_parser()) // Handle various whitespace
        .then_ignore(just("[: "))
        .then(
            // Parse replacement word(s) using pure chumsky - separated by whitespace
            none_of(" \t\n\r]")
                .repeated()
                .at_least(1)
                .to_slice()
                .try_map(|word_text: &str, span: SimpleSpan| {
                    let inner_errors = talkbank_model::ErrorCollector::new();
                    match crate::word::parse_word_impl(word_text, 0, &inner_errors) {
                        ParseOutcome::Parsed(word) => Ok(word),
                        ParseOutcome::Rejected => {
                            // Recovery: preserve raw replacement word as-is.
                            tracing::debug!(
                                word = word_text,
                                "Replacement word parse recovery: preserving as raw text"
                            );
                            Ok(Word::new_unchecked(word_text, word_text)
                                .with_span(Span::from_usize(span.start, span.end)))
                        }
                    }
                })
                .separated_by(one_of(" \t"))
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .then_ignore(just(']'))
        .then(
            // Optional scoped annotations following the replacement: [*], [* code], etc.
            ws_parser()
                .ignore_then(scoped_annotation_parser())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>()
                .or_not(),
        )
        .map(|((original, replacement_words), annotations_opt)| {
            // Replacement wraps a Vec<Word>
            let replacement = talkbank_model::model::Replacement::new(replacement_words);
            let mut replaced_word = talkbank_model::model::ReplacedWord::new(original, replacement);
            if let Some(annotations) = annotations_opt {
                replaced_word = replaced_word.with_scoped_annotations(annotations);
            }
            Box::new(replaced_word)
        })
}
