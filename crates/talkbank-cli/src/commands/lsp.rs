//! CLI bridge for the published `chatter lsp` entrypoint.
//!
//! The reusable stdio server loop lives in the `talkbank-lsp` library crate.
//! This module keeps the `chatter` command surface wired to that library without
//! exposing a standalone LSP binary product.

/// Run the TalkBank language server over stdio until the client disconnects.
pub fn run_lsp() {
    if let Err(err) = talkbank_lsp::run_stdio_server() {
        eprintln!("Error: failed to start language server: {err}");
        std::process::exit(1);
    }
}
