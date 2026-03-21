//! Shared document-loading helpers for execute-command handlers.

use std::sync::Arc;

use talkbank_model::ParseErrors;
use talkbank_model::model::ChatFile;
use tower_lsp::lsp_types::Url;

use crate::backend::documents;
use crate::backend::state::Backend;

/// Return a parsed `ChatFile` either from cache or by reparsing the document text.
pub(super) fn get_chat_file(
    backend: &Backend,
    uri: &Url,
    doc: &str,
) -> Result<Arc<ChatFile>, String> {
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

/// Load both the current document text and its parsed `ChatFile`.
pub(super) fn get_document_and_chat_file(
    backend: &Backend,
    uri: &Url,
) -> Result<(String, Arc<ChatFile>), String> {
    let text = documents::get_document_text(backend, uri)
        .ok_or_else(|| "Document not found".to_string())?;
    let chat_file = get_chat_file(backend, uri, &text)?;
    Ok((text, chat_file))
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
