//! Shared document, parse-tree, and `ChatFile` lookup helpers for request handlers.

use std::sync::Arc;

use talkbank_model::ParseErrors;
use talkbank_model::model::ChatFile;
use tower_lsp::lsp_types::Url;
use tree_sitter::Tree;

use crate::backend::documents;
use crate::backend::state::Backend;

/// Return cached document text for one URI.
pub(super) fn document_text(backend: &Backend, uri: &Url) -> Option<String> {
    documents::get_document_text(backend, uri)
}

/// Return a parsed `ChatFile`, reparsing the document on cache miss.
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

/// Return a parse tree, reparsing and caching it on cache miss.
pub(super) fn get_parse_tree(backend: &Backend, uri: &Url, doc: &str) -> Option<Tree> {
    if let Some(tree) = backend.parse_trees.get(uri) {
        return Some(tree.clone());
    }

    match backend
        .language_services
        .with_parser(|parser| parser.parse_tree_incremental(doc, None))
    {
        Ok(Ok(tree)) => {
            backend.parse_trees.insert(uri.clone(), tree.clone());
            Some(tree)
        }
        _ => None,
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
