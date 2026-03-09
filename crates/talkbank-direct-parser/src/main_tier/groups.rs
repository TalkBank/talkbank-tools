//! Group-level parsers for CHAT main-tier utterances.
//!
//! This module handles angle-bracket groups (`<...>`), phonological groups (`‹...›`),
//! sign/gesture groups (`〔...〕`), quotations, nonvocal markers, and long feature markers.
//! It also contains the `bracketed_item_parser` which handles recursive content inside groups.

use super::annotations::scoped_annotation_parser;
use crate::whitespace::ws_parser;
use chumsky::{error::Rich, prelude::*};
use talkbank_model::Span;
use talkbank_model::model::UtteranceContent;

/// Parse a bracketed item (word, event, pause, etc. inside groups)
/// Supports nested groups, annotations, and all content types
pub(super) fn bracketed_item_parser<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, talkbank_model::model::BracketedItem, extra::Err<Rich<'a, char>>> {
    recursive(move |bracketed_item| {
        choice((
            // Nested pho groups: ‹...›
            just('‹')
                .ignore_then(
                    bracketed_item
                        .clone()
                        .separated_by(ws_parser())
                        .allow_leading()
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then_ignore(just('›'))
                .map(|items| {
                    let bracketed_content = talkbank_model::model::BracketedContent::new(items);
                    talkbank_model::model::BracketedItem::PhoGroup(
                        talkbank_model::model::PhoGroup::new(bracketed_content),
                    )
                })
                .boxed(),
            // Nested sin groups: 〔...〕
            just('〔')
                .ignore_then(
                    bracketed_item
                        .clone()
                        .separated_by(ws_parser())
                        .allow_leading()
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then_ignore(just('〕'))
                .map(|items| {
                    let bracketed_content = talkbank_model::model::BracketedContent::new(items);
                    talkbank_model::model::BracketedItem::SinGroup(
                        talkbank_model::model::SinGroup::new(bracketed_content),
                    )
                })
                .boxed(),
            // Nested annotated groups: <...> [/] (only annotated version allowed in bracketed content)
            just('<')
                .ignore_then(
                    bracketed_item
                        .clone()
                        .separated_by(ws_parser())
                        .allow_leading()
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then_ignore(just('>'))
                .then_ignore(ws_parser())
                .then(
                    scoped_annotation_parser()
                        .boxed()
                        .repeated()
                        .at_least(1)
                        .collect::<Vec<_>>(),
                )
                .map(|(items, annotations)| {
                    let bracketed_content = talkbank_model::model::BracketedContent::new(items);
                    let group = talkbank_model::model::Group::new(bracketed_content);
                    talkbank_model::model::BracketedItem::AnnotatedGroup(
                        talkbank_model::model::Annotated::new(group)
                            .with_scoped_annotations(annotations),
                    )
                })
                .boxed(),
            // Nested quotations (straight quotes): "..."
            just('"')
                .ignore_then(
                    bracketed_item
                        .clone()
                        .separated_by(ws_parser())
                        .allow_leading()
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then_ignore(just('"'))
                .map(|items| {
                    let bracketed_content = talkbank_model::model::BracketedContent::new(items);
                    talkbank_model::model::BracketedItem::Quotation(
                        talkbank_model::model::Quotation::new(bracketed_content),
                    )
                })
                .boxed(),
            // Nested quotations (curly quotes): \u{201C}...\u{201D} (U+201C and U+201D)
            just('\u{201C}')
                .ignore_then(
                    bracketed_item
                        .clone()
                        .separated_by(ws_parser())
                        .allow_leading()
                        .allow_trailing()
                        .collect::<Vec<_>>(),
                )
                .then_ignore(just('\u{201D}'))
                .map(|items| {
                    let bracketed_content = talkbank_model::model::BracketedContent::new(items);
                    talkbank_model::model::BracketedItem::Quotation(
                        talkbank_model::model::Quotation::new(bracketed_content),
                    )
                })
                .boxed(),
            // Annotated events: &=action [annotation]
            just("&=")
                .ignore_then(
                    none_of(" \t\n\r\u{15}>›〔〕\"\u{201C}\u{201D}[")
                        .repeated()
                        .at_least(1)
                        .to_slice(),
                )
                .then(
                    ws_parser()
                        .ignore_then(scoped_annotation_parser().boxed())
                        .repeated()
                        .at_least(1)
                        .collect::<Vec<_>>()
                        .or_not(),
                )
                .map(|(action, annotations_opt)| {
                    let event = talkbank_model::model::Event::new(action);
                    if let Some(annotations) = annotations_opt {
                        talkbank_model::model::BracketedItem::AnnotatedEvent(
                            talkbank_model::model::Annotated::new(event)
                                .with_scoped_annotations(annotations),
                        )
                    } else {
                        talkbank_model::model::BracketedItem::Event(event)
                    }
                })
                .boxed(),
            // Pauses: (.), (..), (...), (1.5), (7:1.5) - pure chumsky, no string inspection
            // NOTE: .to_slice() must be INSIDE ignore_then to exclude the '(' from the slice
            {
                let digits = || one_of("0123456789").repeated().at_least(1);

                // Timed with colon: (7:1.5) - slice captures "7:1.5"
                let colon_timed = just('(')
                    .ignore_then(
                        digits()
                            .then_ignore(just(':'))
                            .then(one_of("0123456789.").repeated().at_least(1))
                            .to_slice(),
                    )
                    .then_ignore(just(')'))
                    .map(|text: &str| {
                        talkbank_model::model::BracketedItem::Pause(
                            talkbank_model::model::Pause::new(
                                talkbank_model::model::PauseDuration::Timed(
                                    talkbank_model::model::PauseTimedDuration::new(text),
                                ),
                            ),
                        )
                    });

                // Timed decimal: (1.5) - slice captures "1.5"
                let decimal_timed = just('(')
                    .ignore_then(digits().then_ignore(just('.')).then(digits()).to_slice())
                    .then_ignore(just(')'))
                    .map(|text: &str| {
                        talkbank_model::model::BracketedItem::Pause(
                            talkbank_model::model::Pause::new(
                                talkbank_model::model::PauseDuration::Timed(
                                    talkbank_model::model::PauseTimedDuration::new(text),
                                ),
                            ),
                        )
                    });

                // Timed integer: (2) - slice captures "2"
                let integer_timed = just('(')
                    .ignore_then(digits().to_slice())
                    .then_ignore(just(')'))
                    .map(|text: &str| {
                        talkbank_model::model::BracketedItem::Pause(
                            talkbank_model::model::Pause::new(
                                talkbank_model::model::PauseDuration::Timed(
                                    talkbank_model::model::PauseTimedDuration::new(text),
                                ),
                            ),
                        )
                    });

                // Symbolic: (...), (..), (.)
                let symbolic = choice((
                    just("(...)").to(talkbank_model::model::BracketedItem::Pause(
                        talkbank_model::model::Pause::new(
                            talkbank_model::model::PauseDuration::Long,
                        ),
                    )),
                    just("(..)").to(talkbank_model::model::BracketedItem::Pause(
                        talkbank_model::model::Pause::new(
                            talkbank_model::model::PauseDuration::Medium,
                        ),
                    )),
                    just("(.)").to(talkbank_model::model::BracketedItem::Pause(
                        talkbank_model::model::Pause::new(
                            talkbank_model::model::PauseDuration::Short,
                        ),
                    )),
                ));

                choice((colon_timed, decimal_timed, integer_timed, symbolic))
            }
            .boxed(),
            // Internal bullets (appear inside groups)
            just('\u{15}')
                .ignore_then(
                    one_of("0123456789")
                        .repeated()
                        .at_least(1)
                        .to_slice()
                        .then_ignore(just('_'))
                        .then(one_of("0123456789").repeated().at_least(1).to_slice()),
                )
                .then_ignore(just('\u{15}'))
                .try_map(|(start_str, end_str): (&str, &str), span: SimpleSpan| {
                    let start: u32 = start_str
                        .parse()
                        .map_err(|_| Rich::custom(span, "Invalid bullet start"))?;
                    let end: u32 = end_str
                        .parse()
                        .map_err(|_| Rich::custom(span, "Invalid bullet end"))?;
                    Ok(talkbank_model::model::BracketedItem::InternalBullet(
                        talkbank_model::model::Bullet::new(start.into(), end.into()),
                    ))
                })
                .boxed(),
            // Freecodes: [^ text]
            just("[^ ")
                .ignore_then(none_of(']').repeated().at_least(1).to_slice())
                .then_ignore(just(']'))
                .map(|text| {
                    talkbank_model::model::BracketedItem::Freecode(
                        talkbank_model::model::Freecode::new(text),
                    )
                })
                .boxed(),
            // Replaced words: word [: replacement] (must come before annotated words)
            // Use proper word parser for both original and replacement words
            crate::word::word_parser_combinator(offset)
                .then_ignore(ws_parser())
                .then_ignore(just("[: "))
                .then(
                    // Parse replacement words inside [: ...]
                    crate::word::word_parser_combinator(offset)
                        .separated_by(one_of(" \t"))
                        .at_least(1)
                        .collect::<Vec<_>>(),
                )
                .then_ignore(just(']'))
                .then(
                    // Optional scoped annotations after replacement
                    ws_parser()
                        .ignore_then(scoped_annotation_parser().boxed())
                        .repeated()
                        .at_least(1)
                        .collect::<Vec<_>>()
                        .or_not(),
                )
                .map(|((original, replacement_words), annotations_opt)| {
                    let replacement = talkbank_model::model::Replacement::new(replacement_words);
                    let mut replaced_word =
                        talkbank_model::model::ReplacedWord::new(original, replacement);
                    if let Some(annotations) = annotations_opt {
                        replaced_word = replaced_word.with_scoped_annotations(annotations);
                    }
                    talkbank_model::model::BracketedItem::ReplacedWord(Box::new(replaced_word))
                })
                .boxed(),
            // NOTE: Overlap markers inside groups (⌈⌉⌊⌋) are treated as whitespace by group_parser
            // and do NOT appear in the bracketed content. TreeSitterParser completely ignores them.
            // They are consumed by the group_separator() parser, not by bracketed_item_parser.
            //
            // Separators (MUST come before word parser to avoid parsing them as words)
            // Tag: \u{201E} (U+201E)
            just('\u{201E}')
                .to(talkbank_model::model::BracketedItem::Separator(
                    talkbank_model::model::Separator::Tag { span: Span::DUMMY },
                ))
                .boxed(),
            // Vocative: \u{2021} (U+2021)
            just('\u{2021}')
                .to(talkbank_model::model::BracketedItem::Separator(
                    talkbank_model::model::Separator::Vocative { span: Span::DUMMY },
                ))
                .boxed(),
            // Comma
            just(',')
                .to(talkbank_model::model::BracketedItem::Separator(
                    talkbank_model::model::Separator::Comma { span: Span::DUMMY },
                ))
                .boxed(),
            // Semicolon
            just(';')
                .to(talkbank_model::model::BracketedItem::Separator(
                    talkbank_model::model::Separator::Semicolon { span: Span::DUMMY },
                ))
                .boxed(),
            // Colon (single : not part of a word)
            just(':')
                .to(talkbank_model::model::BracketedItem::Separator(
                    talkbank_model::model::Separator::Colon { span: Span::DUMMY },
                ))
                .boxed(),
            // Action markers inside groups: 0 [annotation]* or bare 0.
            // word_parser_combinator cannot parse bare '0' (it requires at least one word body
            // character), so these must be handled explicitly.
            //
            // Annotated action: 0 followed by whitespace + at least one scoped annotation
            // (e.g., <0 [= ! whining]>).  The whitespace requirement prevents false-matching
            // of omission words like `0word`.
            just('0')
                .then(
                    ws_parser()
                        .ignore_then(scoped_annotation_parser().boxed())
                        .repeated()
                        .at_least(1)
                        .collect::<Vec<_>>(),
                )
                .map(|(_, annotations)| {
                    talkbank_model::model::BracketedItem::AnnotatedAction(
                        talkbank_model::model::Annotated::new(talkbank_model::model::Action::new())
                            .with_scoped_annotations(annotations),
                    )
                })
                .boxed(),
            // Annotated words: word [annotation]
            // Use proper word parser to handle compound markers and other word features
            crate::word::word_parser_combinator(offset)
                .then(
                    ws_parser()
                        .ignore_then(scoped_annotation_parser().boxed())
                        .repeated()
                        .at_least(1)
                        .collect::<Vec<_>>()
                        .or_not(),
                )
                .map(|(word, annotations_opt)| {
                    if let Some(annotations) = annotations_opt {
                        talkbank_model::model::BracketedItem::AnnotatedWord(Box::new(
                            talkbank_model::model::Annotated::new(word)
                                .with_scoped_annotations(annotations),
                        ))
                    } else {
                        talkbank_model::model::BracketedItem::Word(Box::new(word))
                    }
                })
                .boxed(),
            // Bare action: lone '0' with no following word body or annotations.
            // Must come after the word parsers so omission words (`0word`) are parsed as
            // words rather than (action + unconsumed leftover that breaks the group).
            just('0')
                .map(|_| {
                    talkbank_model::model::BracketedItem::Action(
                        talkbank_model::model::Action::new(),
                    )
                })
                .boxed(),
        ))
    })
}

/// Parse a group: <I want> or <I want> [/]
/// Returns either a bare Group or an AnnotatedGroup
pub(super) fn group_parser<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, UtteranceContent, extra::Err<Rich<'a, char>>> {
    // Inside groups, overlap markers (⌈⌉⌊⌋) with optional index are treated as whitespace
    // and do NOT appear in the output. TreeSitterParser completely ignores them.
    let group_separator = || {
        ws_parser()
            .or(
                // Consume overlap markers with optional index as if they were whitespace
                choice((
                    just('⌈').then(one_of("123456789").or_not()).ignored(),
                    just('⌉').then(one_of("123456789").or_not()).ignored(),
                    just('⌊').then(one_of("123456789").or_not()).ignored(),
                    just('⌋').then(one_of("123456789").or_not()).ignored(),
                )),
            )
            .repeated()
            .at_least(1)
            .ignored()
    };

    let group = just('<')
        .ignore_then(
            bracketed_item_parser(offset)
                .separated_by(group_separator())
                .allow_leading()
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(just('>'))
        .map(|items| {
            let bracketed_content = talkbank_model::model::BracketedContent::new(items);
            UtteranceContent::Group(talkbank_model::model::Group::new(bracketed_content))
        });

    let annotated = just('<')
        .ignore_then(
            bracketed_item_parser(offset)
                .separated_by(group_separator())
                .allow_leading()
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(just('>'))
        .then(
            ws_parser()
                .ignore_then(scoped_annotation_parser())
                .repeated()
                .at_least(1)
                .collect::<Vec<_>>(),
        )
        .map(|(items, annotations)| {
            let bracketed_content = talkbank_model::model::BracketedContent::new(items);
            let group = talkbank_model::model::Group::new(bracketed_content);
            UtteranceContent::AnnotatedGroup(
                talkbank_model::model::Annotated::new(group).with_scoped_annotations(annotations),
            )
        });

    annotated.or(group)
}

/// Parse a phonological group: ‹hello›
pub(super) fn pho_group_parser<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, talkbank_model::model::PhoGroup, extra::Err<Rich<'a, char>>> {
    just('‹')
        .ignore_then(
            bracketed_item_parser(offset)
                .separated_by(ws_parser())
                .allow_leading()
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(just('›'))
        .map(|bracketed_items| {
            let bracketed_content = talkbank_model::model::BracketedContent::new(bracketed_items);
            talkbank_model::model::PhoGroup::new(bracketed_content)
        })
}

/// Parse a sign/gesture group: 〔wave〕
pub(super) fn sin_group_parser<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, talkbank_model::model::SinGroup, extra::Err<Rich<'a, char>>> {
    just('〔')
        .ignore_then(
            bracketed_item_parser(offset)
                .separated_by(ws_parser())
                .allow_leading()
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(just('〕'))
        .map(|bracketed_items| {
            let bracketed_content = talkbank_model::model::BracketedContent::new(bracketed_items);
            talkbank_model::model::SinGroup::new(bracketed_content)
        })
}

/// Parse curly quotation: \u{201C}hello there\u{201D} (U+201C and U+201D)
pub(super) fn curly_quotation_parser<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, talkbank_model::model::Quotation, extra::Err<Rich<'a, char>>> {
    just('\u{201C}') // Left curly double quote
        .ignore_then(
            bracketed_item_parser(offset)
                .separated_by(ws_parser())
                .allow_leading()
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(just('\u{201D}')) // Right curly double quote
        .map(|bracketed_items| {
            let bracketed_content = talkbank_model::model::BracketedContent::new(bracketed_items);
            talkbank_model::model::Quotation::new(bracketed_content)
        })
}

/// Parse straight quotation: "hello there"
pub(super) fn straight_quotation_parser<'a>(
    offset: usize,
) -> impl Parser<'a, &'a str, talkbank_model::model::Quotation, extra::Err<Rich<'a, char>>> {
    just('"')
        .ignore_then(
            bracketed_item_parser(offset)
                .separated_by(ws_parser())
                .allow_leading()
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(just('"'))
        .map(|bracketed_items| {
            let bracketed_content = talkbank_model::model::BracketedContent::new(bracketed_items);
            talkbank_model::model::Quotation::new(bracketed_content)
        })
}

/// Parse a long feature begin marker: &{l=LABEL
pub(super) fn long_feature_begin_parser<'a>()
-> impl Parser<'a, &'a str, talkbank_model::model::LongFeatureBegin, extra::Err<Rich<'a, char>>> {
    just("&{l=")
        .ignore_then(
            none_of(" \t\n\r\u{15}") // Handle various whitespace
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .map(talkbank_model::model::LongFeatureBegin::new)
}

/// Parse a long feature end marker: &}l=LABEL
pub(super) fn long_feature_end_parser<'a>()
-> impl Parser<'a, &'a str, talkbank_model::model::LongFeatureEnd, extra::Err<Rich<'a, char>>> {
    just("&}l=")
        .ignore_then(
            none_of(" \t\n\r\u{15}") // Handle various whitespace
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .map(talkbank_model::model::LongFeatureEnd::new)
}

/// Parse nonvocal markers: &{n=LABEL, &{n=LABEL}, &}n=LABEL
pub(super) fn nonvocal_parser<'a>()
-> impl Parser<'a, &'a str, UtteranceContent, extra::Err<Rich<'a, char>>> {
    // Try to parse nonvocal end first: &}n=LABEL
    let nonvocal_end = just("&}n=")
        .ignore_then(
            none_of(" \t\n\r\u{15}") // Handle various whitespace
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .map(|label: &str| {
            UtteranceContent::NonvocalEnd(talkbank_model::model::NonvocalEnd::new(label))
        });

    // Try to parse nonvocal simple: &{n=LABEL}
    let nonvocal_simple = just("&{n=")
        .ignore_then(
            none_of("} \t\n\r\u{15}") // Handle various whitespace
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .then_ignore(just('}'))
        .map(|label: &str| {
            UtteranceContent::NonvocalSimple(talkbank_model::model::NonvocalSimple::new(label))
        });

    // Try to parse nonvocal begin: &{n=LABEL (no closing brace)
    let nonvocal_begin = just("&{n=")
        .ignore_then(
            none_of(" \t\n\r\u{15}") // Handle various whitespace
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .map(|label: &str| {
            UtteranceContent::NonvocalBegin(talkbank_model::model::NonvocalBegin::new(label))
        });

    choice((nonvocal_end, nonvocal_simple, nonvocal_begin))
}
