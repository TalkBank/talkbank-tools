//! Scoped annotation parsing for CHAT utterance content.
//!
//! Scoped annotations include retracings, error markers, explanations, additions,
//! postcodes, duration markers, stressing markers, overlap markers, and more.
//!
//! These parsers are used by both `words.rs` and `groups.rs`.

use chumsky::{error::Rich, prelude::*};
use talkbank_model::model::{Action, UtteranceContent};

/// Parse a scoped annotation: [/], [//], [* m], [= explanation], [=! paralinguistic], etc.
///
/// Uses pure chumsky combinators - no string inspection in closures.
pub(super) fn scoped_annotation_parser<'a>()
-> impl Parser<'a, &'a str, talkbank_model::model::ScopedAnnotation, extra::Err<Rich<'a, char>>> {
    use talkbank_model::model::*;

    // Helper: parse text content until ]
    let text_content = none_of(']').repeated().to_slice();

    // Retracing markers (atomic, no content)
    let retracing = choice((
        just("[///]").to(ScopedAnnotation::MultipleRetracing),
        just("[//]").to(ScopedAnnotation::Retracing),
        just("[/-]").to(ScopedAnnotation::Reformulation),
        just("[/?]").to(ScopedAnnotation::UncertainRetracing),
        just("[/]").to(ScopedAnnotation::PartialRetracing),
    ));

    // Error markers: [*] or [* code]
    let error_marker = just("[*").ignore_then(choice((
        // [* code] - space followed by non-empty code
        just(' ')
            .ignore_then(none_of(']').repeated().at_least(1).to_slice())
            .then_ignore(just(']'))
            .map(|code: &str| {
                ScopedAnnotation::Error(ScopedError {
                    code: Some(code.into()),
                })
            }),
        // [*] - just asterisk, no code
        just(']').to(ScopedAnnotation::Error(ScopedError { code: None })),
    )));

    // Explanation variants: [= text], [=! text], [=? text]
    let explanation_variants = just("[=").ignore_then(choice((
        just("! ")
            .ignore_then(text_content)
            .then_ignore(just(']'))
            .map(|text: &str| {
                ScopedAnnotation::Paralinguistic(ScopedParalinguistic { text: text.into() })
            }),
        just("? ")
            .ignore_then(text_content)
            .then_ignore(just(']'))
            .map(|text: &str| {
                ScopedAnnotation::Alternative(ScopedAlternative { text: text.into() })
            }),
        just(' ')
            .ignore_then(text_content)
            .then_ignore(just(']'))
            .map(|text: &str| {
                ScopedAnnotation::Explanation(ScopedExplanation { text: text.into() })
            }),
    )));

    // Addition: [+ text]
    let addition = just("[+ ")
        .ignore_then(text_content)
        .then_ignore(just(']'))
        .map(|text: &str| ScopedAnnotation::Addition(ScopedAddition { text: text.into() }));

    // Percent comment: [% text]
    let percent_comment = just("[% ")
        .ignore_then(text_content)
        .then_ignore(just(']'))
        .map(|text: &str| {
            ScopedAnnotation::PercentComment(ScopedPercentComment { text: text.into() })
        });

    // Duration: [# time]
    let duration = just("[# ")
        .ignore_then(text_content)
        .then_ignore(just(']'))
        .map(|time: &str| ScopedAnnotation::Duration(ScopedDuration { time: time.into() }));

    // Exclude marker: [e]
    let exclude = just("[e]").to(ScopedAnnotation::ExcludeMarker);

    // Stressing markers
    let stressing = choice((
        just("[!!]").to(ScopedAnnotation::ScopedContrastiveStressing),
        just("[!*]").to(ScopedAnnotation::ScopedBestGuess),
        just("[!]").to(ScopedAnnotation::ScopedStressing),
        just("[?]").to(ScopedAnnotation::ScopedUncertain),
    ));

    // Overlap markers: [<], [>], [<1], [>2], etc.
    let overlap = choice((
        // [<N] - overlap begin with index
        just("[<")
            .ignore_then(one_of("0123456789").repeated().at_least(1).to_slice())
            .then_ignore(just(']'))
            .try_map(|idx: &str, span| {
                let index = idx
                    .parse::<u8>()
                    .map(OverlapMarkerIndex::new)
                    .map_err(|_| Rich::custom(span, "Invalid overlap index"))?;
                Ok(ScopedAnnotation::OverlapBegin(ScopedOverlapBegin {
                    index: Some(index),
                }))
            }),
        // [>N] - overlap end with index
        just("[>")
            .ignore_then(one_of("0123456789").repeated().at_least(1).to_slice())
            .then_ignore(just(']'))
            .try_map(|idx: &str, span| {
                let index = idx
                    .parse::<u8>()
                    .map(OverlapMarkerIndex::new)
                    .map_err(|_| Rich::custom(span, "Invalid overlap index"))?;
                Ok(ScopedAnnotation::OverlapEnd(ScopedOverlapEnd {
                    index: Some(index),
                }))
            }),
        // [<] - overlap begin without index
        just("[<]").to(ScopedAnnotation::OverlapBegin(ScopedOverlapBegin {
            index: None,
        })),
        // [>] - overlap end without index
        just("[>]").to(ScopedAnnotation::OverlapEnd(ScopedOverlapEnd {
            index: None,
        })),
    ));

    // Fallback: unknown annotation as explanation [anything]
    // BUT exclude [: ...] (replacement) and [^ ...] (freecode) - those are separate constructs
    let fallback = just('[')
        .ignore_then(
            // First char must NOT be : or ^ (those are replacements/freecodes)
            none_of(":^]")
                .then(none_of(']').repeated().to_slice())
                .to_slice(),
        )
        .then_ignore(just(']'))
        .map(|text: &str| ScopedAnnotation::Explanation(ScopedExplanation { text: text.into() }));

    // Combine all in order (specific patterns first)
    choice((
        retracing,
        error_marker,
        explanation_variants,
        addition,
        percent_comment,
        duration,
        exclude,
        stressing,
        overlap,
        fallback,
    ))
}

/// Parse standalone scoped annotations as freecode content.
pub(super) fn scoped_annotation_content_parser<'a>()
-> impl Parser<'a, &'a str, UtteranceContent, extra::Err<Rich<'a, char>>> {
    scoped_annotation_parser().map(|annotation| {
        UtteranceContent::AnnotatedAction(
            talkbank_model::model::Annotated::new(Action::new())
                .with_scoped_annotations(vec![annotation]),
        )
    })
}
