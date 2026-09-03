use talkbank_model::model::{ChatFile, LanguageCode};

use crate::asr_postprocess;

use super::{ParticipantDesc, TranscriptDescription, UtteranceDesc, WordDesc};

/// Build a CHAT file from a JSON transcript description string.
///
/// This is the entry point used by the PyO3 bridge (`build_chat_inner`).
pub fn build_chat_from_json(json: &str) -> Result<ChatFile, String> {
    let desc: TranscriptDescription =
        serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {e}"))?;
    // The PyO3 edge speaks strings; the typed error is flattened HERE, at the
    // boundary, rather than by everything upstream of it.
    super::build_chat(&desc).map_err(|e| e.to_string())
}

/// Domain errors from building a `TranscriptDescription`.
///
/// Exposes structured failure information, the offending word's
/// position, text, declared language, and the full
/// `Vec<talkbank_model::ParseError>` from `ChatWordText::try_from_lang`
///, so upstream callers can render diagnostics or branch on failure
/// class without re-parsing a string.
#[derive(Debug, thiserror::Error)]
pub enum TranscriptBuildError {
    /// A word failed CHAT-legality validation under its utterance's
    /// language. Normalization upstream in `process_raw_asr` should
    /// have rewritten reporter-class tokens (`%`, digit-hyphen compounds)
    /// before this gate; any failure surfacing here is a residual case
    /// the normalizer hasn't been taught yet.
    #[error(
        "word {word_idx} ({word_text:?}) in utterance {utt_idx} \
         (speaker *{speaker_id}:, lang {lang}) failed CHAT validation: {}",
        parse_errors.iter()
            .map(|e| format!("[{}] {}", e.code.as_str(), e.message))
            .collect::<Vec<_>>()
            .join("; ")
    )]
    WordFailedValidation {
        /// Zero-based index of the utterance containing the bad word.
        utt_idx: usize,
        /// Zero-based index of the word within its utterance.
        word_idx: usize,
        /// Speaker code for the enclosing utterance (e.g. `"PAR0"`).
        speaker_id: String,
        /// Original ASR token text (before any attempted normalization).
        word_text: String,
        /// ISO 639-3 language code the word was validated under.
        lang: String,
        /// Structured parse/validation errors from
        /// [`ChatWordText::try_from_lang`].
        parse_errors: Vec<talkbank_model::ParseError>,
    },

    /// A language code supplied to the bridge (transcript-level or
    /// per-utterance) is not a valid CHAT language code. chatter 0.3.x
    /// made [`LanguageCode`] construction fallible; the bridge parses
    /// every code at this boundary so downstream code only sees typed
    /// values. (chatter v0.3.1 re-exports the error type, so the source
    /// is typed.)
    #[error("invalid language code {lang:?}")]
    InvalidLanguageCode {
        /// The offending raw code as supplied by the caller.
        lang: String,
        /// The upstream construction error.
        #[source]
        source: talkbank_model::model::LanguageCodeError,
    },
}

/// A word the language gate refused, emitted anyway for human review.
///
/// This type exists because the refusal used to go to `tracing::warn!` and
/// nowhere else. Shape C in the workspace's list, in its commonest disguise: a
/// return type too weak to hold what the stage learned sends the fact to a log
/// line, where it looks handled. `WordDesc` cannot say "this one is unproved",
/// so the fact travels beside the description instead.
///
/// Emitting the token verbatim is a deliberate, unchanged POLICY: the surface
/// is the provider's observation and what it should have been is a human's
/// call, not the pipeline's. Reporting it is the other half of that policy,
/// which was never built.
#[derive(Debug, Clone)]
pub struct LanguageInvalidWord {
    /// Zero-based index of the utterance the word is in.
    pub utt_idx: usize,
    /// Zero-based index of the word within its utterance.
    pub word_idx: usize,
    /// Speaker code for the enclosing utterance (e.g. `"PAR1"`).
    pub speaker_id: String,
    /// The provider's surface, verbatim, exactly as emitted.
    pub text: String,
    /// ISO 639-3 language the word was judged under.
    pub lang: String,
    /// Why the language gate refused it (E220 digits, E241 reserved
    /// untranscribed marker, and anything else `Word::validate` reports).
    pub parse_errors: Vec<talkbank_model::ParseError>,
}

/// A transcript description together with every word that reached it unproved.
///
/// `#[must_use]`, and the two fields are deliberately not collapsed into one:
/// a caller that wants only the description has to say so, in a line a reader
/// can see, rather than by never being offered the rest.
#[must_use]
#[derive(Debug, Clone)]
pub struct AsrTranscript {
    /// The pre-serialization transcript, ready for `build_chat`.
    pub description: TranscriptDescription,
    /// Words emitted despite failing language-level validation, in emission
    /// order. Empty means every word carried a full language-level proof.
    pub language_invalid: Vec<LanguageInvalidWord>,
}

/// Convert post-processed ASR utterances into a pre-serialization
/// `TranscriptDescription`.
///
/// Each word's text is validated at construction via
/// [`ChatWordText::try_from_lang`][try_lang] under the utterance's declared
/// language (falling back to the primary `langs[0]` or `"eng"`). Fails
/// with [`TranscriptBuildError`] at the first offending word. This is
/// the "loud guard" half of strategy 4c: normalization runs upstream
/// in `process_raw_asr`'s stages; this gate is the belt after the
/// braces.
///
/// A word that is structurally legal but fails the LANGUAGE-level rules is
/// still emitted verbatim, deliberately, and is now also returned in
/// [`AsrTranscript::language_invalid`] so a caller can act on it.
///
/// [try_lang]: asr_postprocess::ChatWordText::try_from_lang
pub fn transcript_from_asr_utterances(
    utterances: &[asr_postprocess::Utterance],
    participant_ids: &[String],
    langs: &[String],
    media_name: Option<&str>,
    write_wor: bool,
) -> Result<AsrTranscript, TranscriptBuildError> {
    if let Ok(path) = std::env::var("BA3_DUMP_UTTERANCES")
        && let Ok(json) = serde_json::to_string_pretty(utterances)
    {
        let _ = std::fs::write(&path, json);
        tracing::warn!(path = %path, "BA3_DUMP_UTTERANCES wrote post-processed utterances");
    }

    let participants = build_asr_participants(utterances, participant_ids);
    let primary_lang_raw = langs.first().map(String::as_str).unwrap_or("eng");
    let primary_lang_code = LanguageCode::new(primary_lang_raw).map_err(|source| {
        TranscriptBuildError::InvalidLanguageCode {
            lang: primary_lang_raw.to_string(),
            source,
        }
    })?;

    let mut utterance_descs = Vec::with_capacity(utterances.len());
    let mut language_invalid: Vec<LanguageInvalidWord> = Vec::new();
    for (utt_idx, utterance) in utterances.iter().enumerate() {
        let speaker_id = resolve_speaker_id(utterance.speaker, participant_ids);
        let utterance_lang = match utterance.lang.as_deref() {
            Some(raw) => LanguageCode::new(raw).map_err(|source| {
                TranscriptBuildError::InvalidLanguageCode {
                    lang: raw.to_string(),
                    source,
                }
            })?,
            None => primary_lang_code.clone(),
        };

        let mut words = Vec::with_capacity(utterance.words.len());
        for (word_idx, word) in utterance.words.iter().enumerate() {
            let admitted =
                validate_asr_word(word, &speaker_id, &utterance_lang, utt_idx, word_idx)?;
            // Exhaustive: a future admission outcome must state what the
            // caller owes it rather than falling through this match.
            match admitted {
                WordAdmission::LanguageProved(desc) => words.push(desc),
                WordAdmission::StructuralOnly { desc, refusal } => {
                    words.push(desc);
                    language_invalid.push(refusal);
                }
            }
        }

        utterance_descs.push(UtteranceDesc {
            speaker: speaker_id,
            words: Some(words),
            text: None,
            start_ms: None,
            end_ms: None,
            lang: utterance.lang.clone(),
        });
    }

    Ok(AsrTranscript {
        description: TranscriptDescription {
            langs: if langs.is_empty() {
                vec!["eng".to_string()]
            } else {
                langs.to_vec()
            },
            participants,
            media_name: media_name.map(String::from),
            media_type: Some("audio".to_string()),
            media_status: None,
            utterances: utterance_descs,
            write_wor,
        },
        language_invalid,
    })
}

fn build_asr_participants(
    utterances: &[asr_postprocess::Utterance],
    participant_ids: &[String],
) -> Vec<ParticipantDesc> {
    let mut seen_speakers: Vec<asr_postprocess::SpeakerIndex> = Vec::new();
    for utterance in utterances {
        if !seen_speakers.contains(&utterance.speaker) {
            seen_speakers.push(utterance.speaker);
        }
    }
    seen_speakers.sort_unstable();

    seen_speakers
        .iter()
        .map(|&speaker| {
            let id = resolve_speaker_id(speaker, participant_ids);
            let (_name, role) = role_for_speaker_code(&id);
            ParticipantDesc {
                id,
                name: None,
                role,
                corpus: String::new(),
            }
        })
        .collect()
}

fn resolve_speaker_id(
    speaker: asr_postprocess::SpeakerIndex,
    participant_ids: &[String],
) -> String {
    let index = speaker.as_usize();
    if index < participant_ids.len() {
        participant_ids[index].clone()
    } else {
        format!("SP{index}")
    }
}

/// How much proof one ASR token carries into the transcript.
///
/// Two admissions, and they are NOT the same value: the first is a word the
/// language rules accept, the second is a word they refuse that we emit
/// anyway. Before 2026-09-03 both arrived at the caller as a bare
/// `ChatWordText` and the difference lived only in a log line, which is why
/// two invalid tokens could reach a corpus with nothing recording that the
/// gate had already caught them.
enum WordAdmission {
    /// Passed [`ChatWordText::try_from_lang`][try_lang] under the utterance's
    /// language: structurally a word AND legal in that language.
    ///
    /// [try_lang]: asr_postprocess::ChatWordText::try_from_lang
    LanguageProved(WordDesc),
    /// Parses as a word but breaks a language-level rule (E220 digits, E241
    /// reserved untranscribed marker, ...). Emitted verbatim by policy, with
    /// the refusal attached so the caller has to decide what to do about it.
    StructuralOnly {
        /// The word as it will be emitted: the provider's surface, unchanged.
        desc: WordDesc,
        /// What the language gate refused, and why.
        refusal: LanguageInvalidWord,
    },
}

fn validate_asr_word(
    word: &asr_postprocess::AsrWord,
    speaker_id: &str,
    utterance_lang: &LanguageCode,
    utt_idx: usize,
    word_idx: usize,
) -> Result<WordAdmission, TranscriptBuildError> {
    let describe = |text: asr_postprocess::ChatWordText| WordDesc {
        text,
        start_ms: word.start_ms.map(|ms| ms as u64),
        end_ms: word.end_ms.map(|ms| ms as u64),
        kind: word.kind,
    };

    let lang_errors =
        match asr_postprocess::ChatWordText::try_from_lang(word.text.as_str(), utterance_lang) {
            Ok(text) => return Ok(WordAdmission::LanguageProved(describe(text))),
            Err(lang_errors) => lang_errors,
        };

    // Structural legality is a genuinely weaker claim, so it produces a
    // genuinely different admission rather than the same `ChatWordText` the
    // proved path returns.
    match asr_postprocess::ChatWordText::try_from(word.text.as_str()) {
        Ok(structural) => {
            // Logged AND returned. Narration for whoever is watching the run
            // is fine; it stopped being the only record of the fact.
            tracing::warn!(
                utt_idx,
                word_idx,
                speaker_id = %speaker_id,
                word_text = %word.text.as_str(),
                lang = %utterance_lang.as_str(),
                lang_errors = ?lang_errors,
                "ASR token fails language-level validation \
                 (structurally legal CHAT); emitting verbatim \
                 for downstream validator + CHECK to surface",
            );
            Ok(WordAdmission::StructuralOnly {
                desc: describe(structural),
                refusal: LanguageInvalidWord {
                    utt_idx,
                    word_idx,
                    speaker_id: speaker_id.to_owned(),
                    text: word.text.as_str().to_owned(),
                    lang: utterance_lang.as_str().to_owned(),
                    parse_errors: lang_errors,
                },
            })
        }
        Err(parse_errors) => Err(TranscriptBuildError::WordFailedValidation {
            utt_idx,
            word_idx,
            speaker_id: speaker_id.to_owned(),
            word_text: word.text.as_str().to_owned(),
            lang: utterance_lang.as_str().to_owned(),
            parse_errors,
        }),
    }
}

fn role_for_speaker_code(code: &str) -> (String, String) {
    match code {
        "INV" => ("Investigator".into(), "Investigator".into()),
        "CHI" => ("Target_Child".into(), "Target_Child".into()),
        "MOT" => ("Mother".into(), "Mother".into()),
        "FAT" => ("Father".into(), "Father".into()),
        "EXP" => ("Experimenter".into(), "Experimenter".into()),
        "OBS" => ("Observer".into(), "Observer".into()),
        "TEA" => ("Teacher".into(), "Teacher".into()),
        _ => ("Participant".into(), "Participant".into()),
    }
}
