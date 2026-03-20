//! Complex header parsers — headers with structured or multi-field content.
//!
//! These parsers handle headers that require more than simple text extraction:
//! comma-separated values, pipe-delimited fields, participant entries, etc.

use chumsky::prelude::*;
use talkbank_model::model::{
    ActivityType, AgeValue, BirthplaceDescription, ChatDate, ChatOptionFlag, CustomIdField,
    DesignType, EducationDescription, GemLabel, GroupName, GroupType, Header, IDHeader,
    LanguageCode, LanguageCodes, LanguageName, MediaHeader, MediaStatus, MediaType, Number,
    ParticipantEntry, RecordingQuality, SesValue, Sex, SpeakerCode, TimeDurationValue,
    TimeStartValue, Transcription, TypesHeader,
};

use super::helpers::{
    header_content_normalized_trimmed, header_content_trimmed, unknown_header_with_reason,
};

// ============================================================================
// Language and Media Headers
// ============================================================================

/// Parse @Languages header.
///
/// Grammar:
/// ```text
/// languages_header: $ => seq(
///     $.languages_prefix,  // "@Languages"
///     $.header_sep,        // ":\t"
///     $.languages_contents,
///     $.newline
/// )
/// ```
///
/// Example: `@Languages:\teng, spa, fra`
pub(super) fn languages_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    use talkbank_model::model::{LanguageCode, LanguageCodes};
    let codes_parser = any()
        .filter(|c: &char| c.is_alphabetic())
        .repeated()
        .at_least(1)
        .to_slice()
        .separated_by(just(',').then(just(' ').or_not()))
        .at_least(1)
        .collect::<Vec<_>>();

    let with_codes = just("@Languages:")
        .then_ignore(just('\t'))
        .ignore_then(codes_parser)
        .then_ignore(just('\n').or_not())
        .map(|codes: Vec<&str>| {
            let lang_codes: Vec<LanguageCode> =
                codes.iter().map(|s| LanguageCode::new(*s)).collect();

            Header::Languages {
                codes: LanguageCodes::new(lang_codes),
            }
        });

    let empty = just("@Languages:")
        .then_ignore(just('\t').or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|_, extra| {
            unknown_header_with_reason(
                extra.slice(),
                "Missing @Languages value".to_string(),
                Some("Provide @Languages:\t<code>[, <code>...]"),
            )
        });

    with_codes.or(empty)
}

/// Parse @Media header.
pub(super) fn media_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    let token = none_of(",\n\r \t").repeated().at_least(1).to_slice();
    let contents = token
        .then_ignore(just(','))
        .then_ignore(just(' ').or_not())
        .then(token)
        .then(
            just(',')
                .ignore_then(just(' ').or_not())
                .ignore_then(token)
                .or_not(),
        );

    let with_contents = just("@Media:")
        .then_ignore(just('\t').or_not())
        .ignore_then(contents)
        .then_ignore(just('\n').or_not())
        .map_with(
            |((filename, media_type), status): ((&str, &str), Option<&str>), extra| {
                let _raw = extra.slice();
                // All values accepted via from_text(); unsupported ones flagged by the validator.
                let media_type = MediaType::from_text(media_type);
                let status = status.map(MediaStatus::from_text);

                let mut header = MediaHeader::new(filename, media_type);
                if let Some(status) = status {
                    header = header.with_status(status);
                }
                Header::Media(header)
            },
        );

    let empty = just("@Media:")
        .then_ignore(just('\t').or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|_, extra| {
            unknown_header_with_reason(
                extra.slice(),
                "Empty @Media header".to_string(),
                Some("Provide @Media:\tfilename, audio|video[, status]"),
            )
        });

    with_contents.or(empty)
}

// ============================================================================
// Date, Birth, and Person-Scoped Headers
// ============================================================================

/// Parse @Date header.
pub(super) fn date_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Date:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed().or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|text: Option<String>, extra| {
            let raw = extra.slice();
            match text {
                Some(value) => Header::Date {
                    date: ChatDate::new(value.as_str()),
                },
                None => unknown_header_with_reason(
                    raw,
                    "Missing @Date value".to_string(),
                    Some("Provide @Date:\\tDD-MMM-YYYY"),
                ),
            }
        })
}

/// Parse @Birth of <CODE> header.
pub(super) fn birth_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Birth of ")
        .ignore_then(none_of(":\n\r \t").repeated().at_least(1).to_slice())
        .then_ignore(just(":"))
        .then_ignore(just('\t').or_not())
        .then(header_content_trimmed().or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|(participant, date): (&str, Option<String>), extra| {
            let raw = extra.slice();
            match date {
                Some(value) => Header::Birth {
                    participant: SpeakerCode::new(participant),
                    date: ChatDate::new(value.as_str()),
                },
                None => unknown_header_with_reason(
                    raw,
                    format!("Missing @Birth date for participant '{}'", participant),
                    Some("Provide @Birth of <CODE>:\\tDD-MMM-YYYY"),
                ),
            }
        })
}

/// Parse @Birthplace of <CODE> header.
pub(super) fn birthplace_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Birthplace of ")
        .ignore_then(none_of(":\n\r \t").repeated().at_least(1).to_slice())
        .then_ignore(just(":"))
        .then_ignore(just('\t').or_not())
        .then(header_content_trimmed().or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|(participant, place): (&str, Option<String>), extra| {
            let raw = extra.slice();
            match place {
                Some(value) => Header::Birthplace {
                    participant: SpeakerCode::new(participant),
                    place: BirthplaceDescription::new(value.as_str()),
                },
                None => unknown_header_with_reason(
                    raw,
                    format!(
                        "Missing @Birthplace value for participant '{}'",
                        participant
                    ),
                    Some("Provide @Birthplace of <CODE>:\\t<location>"),
                ),
            }
        })
}

/// Parse @L1 of <CODE> header.
pub(super) fn l1_header_parser<'a>() -> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>>
{
    just("@L1 of ")
        .ignore_then(none_of(":\n\r \t").repeated().at_least(1).to_slice())
        .then_ignore(just(":"))
        .then_ignore(just('\t').or_not())
        .then(header_content_trimmed().or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|(participant, language): (&str, Option<String>), extra| {
            let raw = extra.slice();
            match language {
                Some(value) => Header::L1Of {
                    participant: SpeakerCode::new(participant),
                    language: LanguageName::new(value.as_str()),
                },
                None => unknown_header_with_reason(
                    raw,
                    format!("Missing @L1 value for participant '{}'", participant),
                    Some("Provide @L1 of <CODE>:\\t<language>"),
                ),
            }
        })
}

// ============================================================================
// Enumerated / Option Headers
// ============================================================================

/// Parse @Number header.
pub(super) fn number_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Number:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed().or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|text: Option<String>, extra| {
            let raw = extra.slice();
            let Some(value) = text else {
                return unknown_header_with_reason(
                    raw,
                    "Missing @Number value".to_string(),
                    Some("Use one of: 1, 2, 3, 4, 5, more, audience"),
                );
            };
            // All values accepted via from_text(); unsupported ones flagged by the validator.
            Header::Number {
                number: Number::from_text(&value),
            }
        })
}

/// Parse @Recording Quality header.
pub(super) fn recording_quality_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Recording Quality:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed().or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|text: Option<String>, extra| {
            let raw = extra.slice();
            let Some(value) = text else {
                return unknown_header_with_reason(
                    raw,
                    "Missing @Recording Quality value".to_string(),
                    Some("Use one of: 1, 2, 3, 4, 5"),
                );
            };
            // All values accepted via from_text(); unsupported ones flagged by the validator.
            Header::RecordingQuality {
                quality: RecordingQuality::from_text(&value),
            }
        })
}

/// Parse @Transcription header.
pub(super) fn transcription_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Transcription:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed().or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|text: Option<String>, extra| {
            let raw = extra.slice();
            let Some(value) = text else {
                return unknown_header_with_reason(
                    raw,
                    "Missing @Transcription value".to_string(),
                    Some(
                        "Use one of: eye_dialect, partial, full, detailed, coarse, checked, anonymized",
                    ),
                );
            };
            // All values accepted via from_text(); unsupported ones flagged by the validator.
            Header::Transcription {
                transcription: Transcription::from_text(&value),
            }
        })
}

/// Parse @Options header.
pub(super) fn options_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    let option = none_of(",\n\r \t").repeated().at_least(1).to_slice();
    let options_body = option
        .separated_by(just(',').then(just(' ').or_not()))
        .collect::<Vec<_>>();

    let with_options = just("@Options:")
        .then_ignore(just('\t').or_not())
        .ignore_then(options_body)
        .then_ignore(just('\n').or_not())
        .map(|values: Vec<&str>| {
            let flags = values
                .iter()
                .map(|value| ChatOptionFlag::from_text(value))
                .collect::<Vec<_>>();
            Header::Options {
                options: flags.into(),
            }
        });

    let empty = just("@Options:")
        .then_ignore(just('\t').or_not())
        .then_ignore(just('\n').or_not())
        .to(Header::Options {
            options: Vec::new().into(),
        });

    with_options.or(empty)
}

/// Parse @Types header.
pub(super) fn types_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    let token = || {
        none_of(",\n\r \t")
            .repeated()
            .at_least(1)
            .collect::<String>()
    };
    let spaces = one_of(" \t").repeated();

    just("@Types:")
        .then_ignore(just('\t').or_not())
        .then_ignore(spaces)
        .ignore_then(token())
        .then_ignore(spaces)
        .then_ignore(just(','))
        .then_ignore(spaces)
        .then(token())
        .then_ignore(spaces)
        .then_ignore(just(','))
        .then_ignore(spaces)
        .then(token())
        .then_ignore(just('\n').or_not())
        .map(|((design, activity), group): ((String, String), String)| {
            Header::Types(TypesHeader::new(
                DesignType::from_text(design.as_str()),
                ActivityType::from_text(activity.as_str()),
                GroupType::from_text(group.as_str()),
            ))
        })
}

// ============================================================================
// Time Headers
// ============================================================================

/// Parse @Time Duration header.
pub(super) fn time_duration_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Time Duration:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::TimeDuration {
            duration: TimeDurationValue::new(text.as_str()),
        })
}

/// Parse @Time Start header.
pub(super) fn time_start_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@Time Start:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_trimmed())
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::TimeStart {
            start: TimeStartValue::new(text.as_str()),
        })
}

// ============================================================================
// Gem Headers
// ============================================================================

/// Parse @Bg header (begin gem).
pub(super) fn begin_gem_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    let with_label = just("@Bg:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_normalized_trimmed()) // Normalize continuation markers
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::BeginGem {
            label: Some(GemLabel::new(text.as_str())),
        });
    let empty = just("@Bg")
        .then_ignore(just('\n').or_not())
        .to(Header::BeginGem { label: None });
    with_label.or(empty)
}

/// Parse @Eg header (end gem).
pub(super) fn end_gem_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    let with_label = just("@Eg:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_normalized_trimmed()) // Normalize continuation markers
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::EndGem {
            label: Some(GemLabel::new(text.as_str())),
        });
    let empty = just("@Eg")
        .then_ignore(just('\n').or_not())
        .to(Header::EndGem { label: None });
    with_label.or(empty)
}

/// Parse @G header (lazy gem).
pub(super) fn lazy_gem_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    just("@G:")
        .then_ignore(just('\t').or_not())
        .ignore_then(header_content_normalized_trimmed()) // Normalize continuation markers
        .then_ignore(just('\n').or_not())
        .map(|text: String| Header::LazyGem {
            label: Some(GemLabel::new(text.as_str())),
        })
}

// ============================================================================
// Participant and ID Headers
// ============================================================================

/// Parse a single participant entry: CODE [NAME] ROLE
///
/// Format variations:
/// - `CHI Target_Child` (code + role)
/// - `CHI Alex Target_Child` (code + name + role)
pub(super) fn participant_entry_parser<'a>()
-> impl Parser<'a, &'a str, ParticipantEntry, extra::Err<Rich<'a, char>>> {
    use talkbank_model::model::{ParticipantName, ParticipantRole, SpeakerCode};

    let word = none_of(" ,\t\n\r").repeated().at_least(1).to_slice();

    let whitespace = choice((
        just("\r\n\t"), // Continuation CRLF+tab
        just("\n\t"),   // Continuation LF+tab
        just("\t"),     // Tab
        just(" "),      // Space
    ))
    .repeated()
    .at_least(1)
    .ignored();

    // Parse at least "CODE ROLE"; any middle words become participant name.
    word.separated_by(whitespace)
        .at_least(2)
        .collect::<Vec<_>>()
        .map(|parts: Vec<&str>| {
            let code = parts[0];
            let role = parts[parts.len() - 1];
            let name_parts = &parts[1..parts.len() - 1];

            if name_parts.is_empty() {
                ParticipantEntry {
                    speaker_code: SpeakerCode::new(code),
                    name: None,
                    role: ParticipantRole::new(role),
                }
            } else {
                let name = name_parts.join(" ");
                ParticipantEntry {
                    speaker_code: SpeakerCode::new(code),
                    name: Some(ParticipantName::new(name.as_str())),
                    role: ParticipantRole::new(role),
                }
            }
        })
}

/// Parse @Participants header.
pub(super) fn participants_header_parser<'a>()
-> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>> {
    use talkbank_model::model::ParticipantEntries;

    // Separator between participant entries: optional whitespace, comma, optional whitespace
    // Whitespace can include continuation markers (\n\t)
    let separator = choice((
        just("\r\n\t"), // Continuation CRLF+tab
        just("\n\t"),   // Continuation LF+tab
        just(" "),      // Space
    ))
    .repeated()
    .ignore_then(just(','))
    .then_ignore(choice((just("\r\n\t"), just("\n\t"), just(" "))).repeated())
    .ignored();
    let entries_parser = participant_entry_parser()
        .separated_by(separator)
        .at_least(1)
        .allow_trailing()
        .collect::<Vec<_>>();

    let with_entries = just("@Participants:")
        .then_ignore(just('\t'))
        .ignore_then(entries_parser)
        .then_ignore(just('\n').or_not())
        .map(|entries| Header::Participants {
            entries: ParticipantEntries::new(entries),
        });

    let empty = just("@Participants:")
        .then_ignore(just('\t').or_not())
        .then_ignore(just('\n').or_not())
        .map_with(|_, extra| {
            unknown_header_with_reason(
                extra.slice(),
                "Missing @Participants value".to_string(),
                Some("Provide @Participants:\tCODE [NAME] ROLE[, ...]"),
            )
        });

    with_entries.or(empty)
}

/// Parse @ID header.
///
/// Format: `@ID:\tlang|corpus|speaker|age|sex|group|ses|role|education|custom|`
/// Indices:        0     1      2     3   4    5    6   7      8         9
pub(super) fn id_header_parser<'a>() -> impl Parser<'a, &'a str, Header, extra::Err<Rich<'a, char>>>
{
    // Parse a single field (everything until pipe or line break).
    // Spaces are valid in some fields (e.g., "zho, eng"), so do not exclude them.
    let field = none_of("|\n\r\t").repeated().to_slice();

    just("@ID:")
        .then_ignore(just('\t').or_not())
        .ignore_then(
            // Parse pipe-separated fields
            field.separated_by(just('|')).collect::<Vec<_>>(),
        )
        .then_ignore(just('\n').or_not())
        .map_with(|fields: Vec<&str>, extra| {
            let raw = extra.slice();

            // Extract fields by index
            let language = match fields.first() {
                Some(value) => *value,
                None => "",
            };
            let speaker = match fields.get(2) {
                Some(value) => *value,
                None => "",
            };
            let role = match fields.get(7) {
                Some(value) => *value,
                None => "",
            };

            let mut missing_required = Vec::new();
            if language.is_empty() {
                missing_required.push("language");
            }
            if speaker.is_empty() {
                missing_required.push("speaker");
            }
            if role.is_empty() {
                missing_required.push("role");
            }

            if !missing_required.is_empty() {
                return unknown_header_with_reason(
                    raw,
                    format!(
                        "Missing required @ID field(s): {}",
                        missing_required.join(", ")
                    ),
                    Some(
                        "Expected @ID format: @ID:\\tlang|corpus|speaker|age|sex|group|ses|role|education|custom|",
                    ),
                );
            }

            // Split language on comma — multi-language IDs look like "eng, zho".
            let language_codes: Vec<LanguageCode> = language
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(LanguageCode::new)
                .collect();
            let languages = LanguageCodes::new(language_codes);

            // Create with required fields (language, speaker, role)
            let mut id = IDHeader::from_languages(languages, speaker, role);

            // Add optional corpus field
            if let Some(&corpus) = fields.get(1)
                && !corpus.is_empty()
            {
                id = id.with_corpus(corpus);
            }

            if let Some(&age) = fields.get(3)
                && !age.is_empty()
            {
                id = id.with_age(AgeValue::new(age));
            }

            if let Some(&sex) = fields.get(4)
                && !sex.is_empty()
            {
                id = id.with_sex(Sex::from_text(sex));
            }

            if let Some(&group) = fields.get(5)
                && !group.is_empty()
            {
                id = id.with_group(GroupName::new(group));
            }

            if let Some(&ses) = fields.get(6)
                && !ses.is_empty()
            {
                id = id.with_ses(SesValue::from_text(ses));
            }

            if let Some(&education) = fields.get(8)
                && !education.is_empty()
            {
                id = id.with_education(EducationDescription::new(education));
            }

            if let Some(&custom_field) = fields.get(9)
                && !custom_field.is_empty()
            {
                id = id.with_custom_field(CustomIdField::new(custom_field));
            }

            Header::ID(id)
        })
}
