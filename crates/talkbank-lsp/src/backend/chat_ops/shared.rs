//! Shared document-loading helpers for execute-command handlers.

use std::sync::Arc;

use talkbank_model::model::ChatFile;
use tower_lsp::lsp_types::Url;

use crate::backend::documents;
use crate::backend::state::Backend;

/// Return a parsed `ChatFile` either from cache or by reparsing the document text.
pub(super) fn get_chat_file(backend: &Backend, uri: &Url, doc: &str) -> Option<Arc<ChatFile>> {
    if let Some(cached) = backend.chat_files.get(uri) {
        return Some(Arc::clone(cached.value()));
    }

    backend
        .language_services
        .with_parser(|parser| parser.parse_chat_file(doc).ok().map(Arc::new))
        .unwrap_or_default()
}

/// Load both the current document text and its parsed `ChatFile`.
pub(super) fn get_document_and_chat_file(
    backend: &Backend,
    uri: &Url,
) -> Result<(String, Arc<ChatFile>), String> {
    let text = documents::get_document_text(backend, uri)
        .ok_or_else(|| "Document not found".to_string())?;
    let chat_file =
        get_chat_file(backend, uri, &text).ok_or_else(|| "Failed to parse document".to_string())?;
    Ok((text, chat_file))
}
