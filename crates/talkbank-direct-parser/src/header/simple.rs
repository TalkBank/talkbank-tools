//! Simple header parsers — fixed headers and single-value text headers.
//!
//! These parsers handle headers that either have no content (@UTF8, @Begin,
//! @End, @Blank, @New Episode) or a single text value parsed with the standard
//! `header_content_trimmed()` helpers.

use chumsky::prelude::*;
use talkbank_model::model::{
    ActivitiesDescription, BackgroundDescription, ColorWordList, FontSpec, Header,
    LocationDescription, PageNumber, PidValue, RoomLayoutDescription, SituationDescription,
    TDescription, TapeLocationDescription, TranscriberName, VideoSpec, WarningText, WindowGeometry,
};

use super::helpers::{
    header_content, header_content_trimmed, header_content_trimmed_end, unknown_header_with_reason,
};
use crate::text_tier::parse_bullet_content_text;

// ============================================================================
// Fixed Headers (no content)
// ============================================================================

/// Parse @UTF8 header.
///
/// Grammar: `utf8_header: $ => seq(token('@UTF8'), $.newline)`
pub(super) fn utf8_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@UTF8")
        .then_ignore(just('\n').or_not())
        .to(Header::Utf8)
}

/// Parse @Begin header.
///
/// Grammar: `begin_header: $ => seq(token('@Begin'), $.newline)`
pub(super) fn begin_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Begin")
        .then_ignore(just('\n').or_not())
        .to(Header::Begin)
}

/// Parse @End header.
///
/// Grammar: `end_header: $ => seq(token('@End'), $.newline)`
pub(super) fn end_header_parser<'a>() -> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>>
{
    just("@End")
        .then_ignore(just('\n').or_not())
        .to(Header::End)
}

/// Parse @Blank header.
pub(super) fn blank_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Blank")
        .then_ignore(just('\n').or_not())
        .to(Header::Blank)
}

/// Parse @New Episode header.
pub(super) fn new_episode_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@New Episode")
        .then_ignore(just('\n').or_not())
        .to(Header::NewEpisode)
}

// ============================================================================
// Single-Value Text Headers
// ============================================================================

/// Parse @Comment header (supports continuation lines).
pub(super) fn comment_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Comment:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed_end())
        .map(|text: String| Header::Comment {
            content: parse_bullet_content_text(&text),
        })
}

/// Parse @PID header.
pub(super) fn pid_header_parser<'a>() -> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>>
{
    just("@PID:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed().or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|text: Option<String>, extra| {
            let raw = extra.slice();
            match text {
                Some(value) => Header::Pid {
                    pid: PidValue::new(value.as_str()),
                },
                None => unknown_header_with_reason(
                    raw,
                    "Missing @PID value".to_string(),
                    Some("Provide @PID:\\t<value>"),
                ),
            }
        })
}

/// Parse @Situation header.
pub(super) fn situation_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Situation:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed().or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|text: Option<String>, extra| {
            let raw = extra.slice();
            match text {
                Some(value) => Header::Situation {
                    text: SituationDescription::new(value.as_str()),
                },
                None => unknown_header_with_reason(
                    raw,
                    "Missing @Situation value".to_string(),
                    Some("Provide @Situation:\\t<description>"),
                ),
            }
        })
}

/// Parse @Warning header.
pub(super) fn warning_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Warning:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::Warning {
            text: WarningText::new(text.as_str()),
        })
}

/// Parse @Activities header.
pub(super) fn activities_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Activities:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::Activities {
            activities: ActivitiesDescription::new(text.as_str()),
        })
}

/// Parse @Page header.
pub(super) fn page_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Page:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed().or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|text: Option<String>, extra| {
            let raw = extra.slice();
            match text {
                Some(value) => Header::Page {
                    page: PageNumber::new(value.as_str()),
                },
                None => unknown_header_with_reason(
                    raw,
                    "Missing @Page value".to_string(),
                    Some("Provide @Page:\\t<page-number>"),
                ),
            }
        })
}

/// Parse @Font header.
pub(super) fn font_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Font:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::Font {
            font: FontSpec::new(text.as_str()),
        })
}

/// Parse @Window header.
pub(super) fn window_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Window:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::Window {
            geometry: WindowGeometry::new(text.as_str()),
        })
}

/// Parse @Color words header.
pub(super) fn color_words_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Color words:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::ColorWords {
            colors: ColorWordList::new(text.as_str()),
        })
}

/// Parse @Bck header.
pub(super) fn bck_header_parser<'a>() -> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>>
{
    just("@Bck:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::Bck {
            bck: BackgroundDescription::new(text.as_str()),
        })
}

/// Parse @T header (inline thumbnail).
pub(super) fn thumbnail_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@T:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::T {
            text: TDescription::new(text.as_str()),
        })
}

/// Parse @Location header.
pub(super) fn location_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Location:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::Location {
            location: LocationDescription::new(text.as_str()),
        })
}

/// Parse @Room Layout header.
pub(super) fn room_layout_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Room Layout:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::RoomLayout {
            layout: RoomLayoutDescription::new(text.as_str()),
        })
}

/// Parse @Transcriber header.
pub(super) fn transcriber_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Transcriber:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::Transcriber {
            transcriber: TranscriberName::new(text.as_str()),
        })
}

/// Parse @Videos header.
pub(super) fn videos_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Videos:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::Videos {
            videos: VideoSpec::new(text.as_str()),
        })
}

/// Parse @Tape Location header.
pub(super) fn tape_location_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Tape Location:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::TapeLocation {
            location: TapeLocationDescription::new(text.as_str()),
        })
}

/// Parse unknown/generic header as fallback.
pub(super) fn unknown_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just('@')
        .ignore_then(header_content())
        .then_ignore(just('\n').or_not())
        .map(|text: String| {
            use talkbank_model::model::WarningText;
            // Reconstruct full header text with @
            let full_text = format!("@{}", text);
            Header::Unknown {
                text: WarningText::new(full_text),
                parse_reason: Some("Unrecognized header type".to_string()),
                suggested_fix: None,
            }
        })
}
