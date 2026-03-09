use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::backend::state::Backend;

use super::context::{document_text, get_chat_file};

pub(super) async fn handle_formatting(
    backend: &Backend,
    params: DocumentFormattingParams,
) -> Result<Option<Vec<TextEdit>>> {
    let uri = params.text_document.uri;
    let doc = match document_text(backend, &uri) {
        Some(doc) => doc,
        None => return Ok(None),
    };
    let chat_file = match get_chat_file(backend, &uri, &doc) {
        Some(file) => file,
        None => return Ok(None),
    };

    let formatted = chat_file.to_chat();
    if formatted == doc {
        return Ok(None);
    }

    let line_count = doc.lines().count();
    let last_line = doc.lines().last().unwrap_or_default();
    let last_char = last_line.len();

    Ok(Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: line_count as u32 - 1,
                character: last_char as u32,
            },
        },
        new_text: formatted,
    }]))
}
