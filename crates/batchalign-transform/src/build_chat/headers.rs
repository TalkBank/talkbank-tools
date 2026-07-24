use std::path::Path;

use indexmap::IndexMap;
use talkbank_model::model::{
    Header, IDHeader, LanguageCode, LanguageCodes, Line, MediaHeader, MediaType, Participant,
    ParticipantEntries, ParticipantEntry, ParticipantName, ParticipantRole, SpeakerCode,
};

use super::TranscriptDescription;

pub(super) fn build_header_lines(
    desc: &TranscriptDescription,
    langs: &[LanguageCode],
) -> Vec<Line> {
    let participant_entries = build_participant_entries(desc);
    let id_headers = build_id_headers(desc, langs);
    let mut lines: Vec<Line> = vec![
        Line::header(Header::Utf8),
        Line::header(Header::Begin),
        Line::header(Header::Languages {
            codes: LanguageCodes::new(langs.to_vec()),
        }),
        Line::header(Header::Participants {
            entries: ParticipantEntries::new(participant_entries),
        }),
    ];

    for id in id_headers {
        lines.push(Line::header(Header::ID(id)));
    }

    if let Some(media_header) = build_media_header(desc) {
        lines.push(Line::header(Header::Media(media_header)));
    }

    lines
}

/// Assemble the typed participant map the built [`ChatFile`] must carry.
///
/// `ChatFile::new` deliberately leaves its derived participant map empty
/// (parser-intermediate semantics), but programmatically assembled CHAT
/// must expose the same metadata as parsed CHAT: consumers of the
/// in-memory file consult this map rather than re-scanning header lines.
/// Built from the SAME entry + `@ID` values the header lines carry, so
/// map and headers cannot disagree.
///
/// [`ChatFile`]: talkbank_model::model::ChatFile
pub(super) fn build_participant_map(
    desc: &TranscriptDescription,
    langs: &[LanguageCode],
) -> IndexMap<SpeakerCode, Participant> {
    build_participant_entries(desc)
        .into_iter()
        .zip(build_id_headers(desc, langs))
        .map(|(entry, id)| (entry.speaker_code.clone(), Participant::new(entry, id)))
        .collect()
}

fn build_participant_entries(desc: &TranscriptDescription) -> Vec<ParticipantEntry> {
    desc.participants
        .iter()
        .map(|participant| ParticipantEntry {
            speaker_code: SpeakerCode::new(participant.id.as_str()),
            name: participant.name.as_ref().map(ParticipantName::new),
            role: ParticipantRole::new(participant.role.as_str()),
        })
        .collect()
}

fn build_id_headers(desc: &TranscriptDescription, langs: &[LanguageCode]) -> Vec<IDHeader> {
    // BuildChatContext guarantees a non-empty, already-validated list;
    // @ID headers carry the primary (first) language.
    let Some(lang_code) = langs.first() else {
        return Vec::new();
    };

    desc.participants
        .iter()
        .map(|participant| {
            let corpus = if participant.corpus.is_empty() {
                "corpus_name"
            } else {
                participant.corpus.as_str()
            };
            IDHeader::new(
                lang_code.clone(),
                participant.id.as_str(),
                participant.role.as_str(),
            )
            .with_corpus(corpus)
        })
        .collect()
}

fn build_media_header(desc: &TranscriptDescription) -> Option<MediaHeader> {
    let media_name = desc.media_name.as_ref()?;
    let normalized_media_name = normalize_media_name(media_name);
    let media_type = match desc.media_type.as_deref() {
        Some("video") => MediaType::Video,
        Some("audio") | None => MediaType::Audio,
        other => {
            tracing::warn!(media_type = ?other, "unrecognized media_type, defaulting to audio");
            MediaType::Audio
        }
    };

    Some(MediaHeader::new(normalized_media_name.as_str(), media_type))
}

fn normalize_media_name(raw: &str) -> String {
    let candidate = Path::new(raw);
    candidate
        .file_stem()
        .filter(|stem| !stem.is_empty())
        .or_else(|| candidate.file_name())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| raw.to_string())
}
