//! Participant data handlers for the LSP.
//!
//! Provides `talkbank/getParticipants` and `talkbank/formatIdLine` execute-command
//! handlers. These delegate CHAT parsing and serialization to the Rust model so the
//! TypeScript side remains a thin UI layer.

use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;
use talkbank_model::ParseErrors;
use talkbank_model::model::{
    AgeValue, ChatFile, CorpusName, CustomIdField, EducationDescription, GroupName, Header,
    IDHeader, LanguageCode, Line, ParticipantRole, SesValue, Sex, SpeakerCode,
};
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::Url;

use super::documents;
use super::execute_commands::{DocumentUriRequest, ExecuteCommandRequest, IdLineFieldsRequest};
use super::state::Backend;

/// Feature-oriented execute-command service for participant commands.
pub(crate) struct ParticipantCommandService;

impl ParticipantCommandService {
    /// Dispatch one participant-family execute-command request.
    pub(crate) fn dispatch(
        &self,
        backend: &Backend,
        request: ExecuteCommandRequest,
    ) -> LspResult<Option<Value>> {
        match request {
            ExecuteCommandRequest::GetParticipants(request) => command_response(
                handle_get_participants(backend, &request),
                "Participant error",
            ),
            ExecuteCommandRequest::FormatIdLine(request) => {
                command_response(handle_format_id_line(&request), "Format error")
            }
            _ => unreachable!("participant service received unsupported execute-command request"),
        }
    }
}

fn command_response(result: Result<Value, String>, prefix: &str) -> LspResult<Option<Value>> {
    match result {
        Ok(json) => Ok(Some(json)),
        Err(error) => Ok(Some(Value::String(format!("{prefix}: {error}")))),
    }
}

/// JSON-serializable participant entry returned by `talkbank/getParticipants`.
#[derive(Serialize)]
struct ParticipantEntry {
    /// 0-based line number of the `@ID` header in the document.
    line: usize,
    /// The ten pipe-delimited `@ID` fields as plain strings.
    fields: ParticipantFields,
}

/// The ten fields of an `@ID` line, all as plain strings for the webview.
#[derive(Serialize)]
struct ParticipantFields {
    /// Language code field.
    language: String,
    /// Corpus name field.
    corpus: String,
    /// Speaker code field.
    speaker: String,
    /// Age field.
    age: String,
    /// Sex field.
    sex: String,
    /// Group field.
    group: String,
    /// SES field.
    ses: String,
    /// Role field.
    role: String,
    /// Education field.
    education: String,
    /// Custom field.
    custom: String,
}

/// Handle `talkbank/getParticipants` — extract participant `@ID` data from a parsed document.
///
/// Returns: JSON array of `ParticipantEntry`.
pub(crate) fn handle_get_participants(
    backend: &Backend,
    request: &DocumentUriRequest,
) -> Result<Value, String> {
    let text = documents::get_document_text(backend, &request.uri)
        .ok_or_else(|| "Document not found".to_string())?;

    let chat_file = get_chat_file(backend, &request.uri, &text)?;

    let entries = extract_participant_entries(&chat_file, &text);

    serde_json::to_value(&entries).map_err(|e| format!("Serialization error: {e}"))
}

/// Handle `talkbank/formatIdLine` — construct a canonical `@ID` line from field values.
///
/// Returns: JSON string containing the formatted `@ID` line.
pub(crate) fn handle_format_id_line(request: &IdLineFieldsRequest) -> Result<Value, String> {
    let id_header = build_id_header(request);
    let formatted = id_header.to_string();

    Ok(Value::String(formatted))
}

/// Build an `IDHeader` from user-provided plain string fields.
fn build_id_header(input: &IdLineFieldsRequest) -> IDHeader {
    let mut header = IDHeader::new(
        LanguageCode::new(&input.language),
        SpeakerCode::new(&input.speaker),
        ParticipantRole::new(&input.role),
    );

    if !input.corpus.is_empty() {
        header = header.with_corpus(CorpusName::new(&input.corpus));
    }
    if !input.age.is_empty() {
        header = header.with_age(AgeValue::new(&input.age));
    }
    if !input.sex.is_empty() {
        header = header.with_sex(Sex::from_text(&input.sex));
    }
    if !input.group.is_empty() {
        header = header.with_group(GroupName::new(&input.group));
    }
    if !input.ses.is_empty() {
        header = header.with_ses(SesValue::from_text(&input.ses));
    }
    if !input.education.is_empty() {
        header = header.with_education(EducationDescription::new(&input.education));
    }
    if !input.custom.is_empty() {
        header = header.with_custom_field(CustomIdField::new(&input.custom));
    }

    header
}

/// Extract participant entries with line numbers from the parsed `ChatFile`.
fn extract_participant_entries(chat_file: &ChatFile, text: &str) -> Vec<ParticipantEntry> {
    let line_index = LineIndex::new(text);

    chat_file
        .lines
        .iter()
        .filter_map(|line| match line {
            Line::Header { header, span } => match header.as_ref() {
                Header::ID(id) => {
                    let line_number = if span.is_dummy() {
                        0
                    } else {
                        line_index.byte_to_line(span.start as usize)
                    };
                    Some(ParticipantEntry {
                        line: line_number,
                        fields: id_header_to_fields(id),
                    })
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// Convert an `IDHeader` to plain string fields for the webview.
fn id_header_to_fields(id: &IDHeader) -> ParticipantFields {
    ParticipantFields {
        language: id
            .language
            .iter()
            .map(|c| c.as_str())
            .collect::<Vec<_>>()
            .join(", "),
        corpus: id
            .corpus
            .as_ref()
            .map_or(String::new(), |c| c.as_str().to_string()),
        speaker: id.speaker.as_str().to_string(),
        age: id
            .age
            .as_ref()
            .map_or(String::new(), |a| a.as_str().to_string()),
        sex: id
            .sex
            .as_ref()
            .map_or(String::new(), |s| s.as_str().to_string()),
        group: id
            .group
            .as_ref()
            .map_or(String::new(), |g| g.as_str().to_string()),
        ses: id.ses.as_ref().map_or(String::new(), |s| s.as_str()),
        role: id.role.as_str().to_string(),
        education: id
            .education
            .as_ref()
            .map_or(String::new(), |e| e.as_str().to_string()),
        custom: id
            .custom_field
            .as_ref()
            .map_or(String::new(), |c| c.as_str().to_string()),
    }
}

/// Get ChatFile from cache or parse.
fn get_chat_file(backend: &Backend, uri: &Url, doc: &str) -> Result<Arc<ChatFile>, String> {
    if let Some(cached) = backend.chat_files.get(uri) {
        return Ok(Arc::clone(cached.value()));
    }

    match backend
        .language_services
        .with_parser(|parser| parser.parse_chat_file(doc))
    {
        Ok(Ok(chat_file)) => Ok(Arc::new(chat_file)),
        Ok(Err(errors)) => Err(format_parse_failure(&errors)),
        Err(error) => Err(error.to_string()),
    }
}

fn format_parse_failure(errors: &ParseErrors) -> String {
    let count = errors.errors.len();
    match errors.errors.first() {
        Some(first) => format!(
            "Failed to parse document ({count} diagnostic{}); first: {}",
            if count == 1 { "" } else { "s" },
            first.message
        ),
        None => "Failed to parse document (parser returned no diagnostics)".to_string(),
    }
}

/// Simple byte-offset to line-number index.
struct LineIndex {
    /// Byte offsets for the start of each line.
    line_starts: Vec<usize>,
}

impl LineIndex {
    /// Build a line index for one document string.
    fn new(text: &str) -> Self {
        let mut line_starts = vec![0];
        for (i, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(i + 1);
            }
        }
        Self { line_starts }
    }

    /// Convert a byte offset into a zero-based line number.
    fn byte_to_line(&self, offset: usize) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(line) => line.saturating_sub(1),
        }
    }
}
