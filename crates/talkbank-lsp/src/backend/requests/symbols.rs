use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::backend::features;
use crate::backend::state::Backend;

use super::context::{document_text, get_chat_file};

pub(super) async fn handle_document_symbol(
    backend: &Backend,
    params: DocumentSymbolParams,
) -> Result<Option<DocumentSymbolResponse>> {
    let uri = params.text_document.uri;
    let doc = match document_text(backend, &uri) {
        Some(doc) => doc,
        None => return Ok(None),
    };
    let chat_file = match get_chat_file(backend, &uri, &doc) {
        Some(file) => file,
        None => return Ok(None),
    };
    Ok(features::document_symbol(&chat_file, &doc))
}

pub(super) async fn handle_folding_range(
    backend: &Backend,
    params: FoldingRangeParams,
) -> Result<Option<Vec<FoldingRange>>> {
    let uri = params.text_document.uri;
    let doc = match document_text(backend, &uri) {
        Some(doc) => doc,
        None => return Ok(None),
    };
    let chat_file = match get_chat_file(backend, &uri, &doc) {
        Some(file) => file,
        None => return Ok(None),
    };
    Ok(Some(features::folding_range(&chat_file, &doc)))
}

pub(super) async fn handle_code_lens(
    backend: &Backend,
    params: CodeLensParams,
) -> Result<Option<Vec<CodeLens>>> {
    let uri = params.text_document.uri;
    let doc = match document_text(backend, &uri) {
        Some(doc) => doc,
        None => return Ok(None),
    };
    let chat_file = match get_chat_file(backend, &uri, &doc) {
        Some(file) => file,
        None => return Ok(None),
    };
    Ok(features::code_lens::code_lens(&chat_file, &doc))
}
