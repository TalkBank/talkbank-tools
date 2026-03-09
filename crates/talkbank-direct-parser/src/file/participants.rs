//! Participant building from CHAT headers (`@Participants`, `@ID`, `@Birth`).

use indexmap::IndexMap;
use smallvec::SmallVec;
use talkbank_model::model::{Header, Participant, SpeakerCode};
use talkbank_model::{ErrorCode, ErrorContext, ParseError, Severity, SourceLocation};

/// Builds participants for downstream use.
pub(crate) fn build_participants(
    headers: &[Header],
) -> (
    IndexMap<SpeakerCode, Participant>,
    SmallVec<[ParseError; 2]>,
) {
    let mut errors = SmallVec::new();
    let mut participants = IndexMap::new();

    let Some(entries) = headers.iter().find_map(|header| match header {
        Header::Participants { entries } => Some(entries),
        _ => None,
    }) else {
        return (participants, errors);
    };

    let id_headers: Vec<&Header> = headers
        .iter()
        .filter(|h| matches!(h, Header::ID(_)))
        .collect();

    for entry in entries {
        let speaker_code = entry.speaker_code.clone();
        let speaker_str = speaker_code.as_str();

        let matching_id = id_headers.iter().find_map(|h| match h {
            Header::ID(id) if id.speaker.as_str() == speaker_str => Some(id.clone()),
            _ => None,
        });

        match matching_id {
            Some(id) => {
                let mut participant = Participant::new(entry.clone(), id);

                if let Some(birth_date) = find_birth_header(speaker_str, headers) {
                    participant = participant.with_birth_date(birth_date);
                }

                participants.insert(speaker_code, participant);
            }
            None => {
                errors.push(
                    ParseError::new(
                        ErrorCode::SpeakerNotDefined,
                        Severity::Error,
                        SourceLocation::at_offset(0),
                        ErrorContext::new(speaker_str, 0..speaker_str.len(), speaker_str),
                        format!(
                            "Participant '{}' listed in @Participants but has no @ID header",
                            speaker_str
                        ),
                    )
                    .with_suggestion(format!(
                        "Add @ID header: @ID:\t<lang>|<corpus>|{}|<age>|<sex>|<group>|<ses>|{}|<edu>|<custom>|",
                        speaker_str, entry.role
                    )),
                );
            }
        }
    }

    for id_header in &id_headers {
        if let Header::ID(id) = id_header
            && !participants.contains_key(&id.speaker)
        {
            let speaker_str = id.speaker.to_string();
            errors.push(
                ParseError::new(
                    ErrorCode::OrphanIDHeader,
                    Severity::Error,
                    SourceLocation::at_offset(0),
                    ErrorContext::new(&speaker_str, 0..speaker_str.len(), &speaker_str),
                    format!(
                        "@ID header for '{}' but speaker not in @Participants",
                        speaker_str
                    ),
                )
                .with_suggestion(format!(
                    "Add to @Participants: {} <name> <role>",
                    speaker_str
                )),
            );
        }
    }

    for header in headers {
        if let Header::Birth { participant, .. } = header
            && !participants.contains_key(participant)
        {
            let speaker_str = participant.to_string();
            errors.push(
                ParseError::new(
                    ErrorCode::BirthUnknownParticipant,
                    Severity::Error,
                    SourceLocation::at_offset(0),
                    ErrorContext::new(&speaker_str, 0..speaker_str.len(), &speaker_str),
                    format!(
                        "@Birth header for '{}' but speaker not a declared participant",
                        speaker_str
                    ),
                )
                .with_suggestion(format!(
                    "Add to @Participants: {} <name> <role>, or remove @Birth header",
                    speaker_str
                )),
            );
        }
    }

    (participants, errors)
}

/// Finds birth header.
fn find_birth_header(
    speaker_code: &str,
    headers: &[Header],
) -> Option<talkbank_model::model::ChatDate> {
    for header in headers {
        if let Header::Birth { participant, date } = header
            && participant.as_str() == speaker_code
        {
            return Some(date.clone());
        }
    }
    None
}
