#![warn(missing_docs)]
//! Tree-sitter-backed parser for TalkBank CHAT.
//!
//! This crate provides two layers:
//! - convenience entry points (`parse_*` functions and `api/` modules),
//! - parser internals (`parser/`) that turn tree-sitter CST nodes into model types.
//!
//! Most callers should use [`TreeSitterParser`] or the top-level full-file
//! helpers. Both routes preserve source spans and report diagnostics through
//! `talkbank-model`.
//!
//! **Important:** full-file parsing is the canonical tree-sitter contract. Some
//! isolated fragment helpers still rely on synthetic wrapper files internally.
//! They remain useful for legacy audits and compatibility checks, but they
//! should not be treated as the semantic oracle for fragment parsing. Those
//! helpers now live under [`synthetic_fragments`] so they are not mistaken for
//! normal crate-root parsing APIs. Use `talkbank-direct-parser` for honest
//! fragment semantics.
//!
//! CHAT reference anchors:
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//!
//! # Examples
//!
//! ```rust
//! use talkbank_parser::TreeSitterParser;
//!
//! let parser = match TreeSitterParser::new() {
//!     Ok(parser) => parser,
//!     Err(_) => return,
//! };
//! let _file = match parser.parse_chat_file("@UTF8\n@Begin\n*CHI:\thello .\n@End\n") {
//!     Ok(file) => file,
//!     Err(_) => return,
//! };
//! ```

pub(crate) mod error {
    pub use talkbank_model::*;
}

pub(crate) mod model {
    pub use talkbank_model::model::*;
}

#[cfg(test)]
pub(crate) mod validation {
    pub use talkbank_model::validation::*;
}

use talkbank_model::model::*;
use talkbank_model::{
    ErrorCode, ErrorContext, ErrorSink, ParseError, ParseErrors, ParseResult, Severity,
    SourceLocation,
};
use thiserror::Error;

/// Node type string constants from tree-sitter-talkbank grammar.
pub mod node_types;

/// Public convenience API modules.
pub mod api;
/// Internal parser implementation modules.
pub(crate) mod parser;

/// Main parser type and initialization error.
pub use parser::{ParserInitError, TreeSitterParser};
pub use talkbank_model::FragmentSemanticContext;

/// Legacy synthetic tree-sitter fragment helpers.
pub use api::synthetic_fragments;
/// Convenience re-exports for dependent-tier parsing APIs.
pub use api::{dependent_tier::parse_dependent_tier, tiers};

// =============================================================================
// Thread-Local Parser Pool
// =============================================================================

use std::cell::RefCell;

thread_local! {
    static THREAD_PARSER: RefCell<Option<TreeSitterParser>> = const { RefCell::new(None) };
}

/// Errors from the thread-local parser pool.
#[derive(Debug, Error)]
pub enum ParserPoolError {
    /// Failed to initialize the parser.
    #[error(transparent)]
    Init(#[from] ParserInitError),
    /// Thread-local parser cell was unexpectedly empty after initialization.
    #[error("Thread-local parser unavailable")]
    Unavailable,
}

/// Execute a function with the thread-local parser (pool pattern).
///
/// `tree_sitter::Parser` is mutable and designed for single-threaded use. This
/// helper keeps one parser instance per thread, which avoids repeated parser
/// construction while also avoiding shared mutable cross-thread state.
pub fn with_parser<F, R>(f: F) -> Result<R, ParserPoolError>
where
    F: FnOnce(&TreeSitterParser) -> R,
{
    THREAD_PARSER.with(|cell| {
        let mut parser_opt = cell.borrow_mut();
        if parser_opt.is_none() {
            let parser = TreeSitterParser::new()?;
            *parser_opt = Some(parser);
        }
        let parser_ref = match parser_opt.as_ref() {
            Some(parser) => parser,
            None => return Err(ParserPoolError::Unavailable),
        };
        Ok(f(parser_ref))
    })
}

/// Clear the thread-local parser cache.
///
/// Used by tests that need deterministic parser lifecycle behavior.
#[doc(hidden)]
pub fn clear_thread_parser() {
    THREAD_PARSER.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

// =============================================================================
// Convenience functions (delegate to TreeSitterParser)
// =============================================================================

/// Parse a complete CHAT file via the thread-local parser.
pub fn parse_chat_file(input: &str) -> ParseResult<ChatFile> {
    match with_parser(|parser| parser.parse_chat_file(input)) {
        Ok(result) => result,
        Err(err) => {
            let mut errors = ParseErrors::new();
            errors.push(ParseError::new(
                ErrorCode::TreeParsingError,
                Severity::Error,
                SourceLocation::from_offsets(0, input.len()),
                ErrorContext::new(input, 0..input.len(), "thread_local"),
                format!("Thread-local parser failed: {err}"),
            ));
            Err(errors)
        }
    }
}

/// Parse a complete CHAT file with streaming error output.
///
/// Errors are reported to the `errors` sink as they're discovered, enabling:
/// - Early cancellation when user has seen enough errors
/// - Real-time error display in GUI applications
/// - Memory-efficient processing of large files
///
/// Unlike `parse_chat_file()`, this function always returns a ChatFile (with error recovery),
/// and streams errors via the sink instead of returning them.
///
/// # Example
///
/// ```rust
/// use talkbank_parser::parse_chat_file_streaming;
/// use talkbank_model::{ErrorSink, ErrorCollector};
///
/// let sink = ErrorCollector::new();
/// let chat_file = parse_chat_file_streaming("*CHI:\thello .", &sink);
/// let errors = sink.into_vec();
/// ```
pub fn parse_chat_file_streaming(input: &str, errors: &impl ErrorSink) -> ChatFile {
    match with_parser(|parser| parser.parse_chat_file_streaming(input, errors)) {
        Ok(chat_file) => chat_file,
        Err(err) => {
            errors.report(ParseError::new(
                ErrorCode::TreeParsingError,
                Severity::Error,
                SourceLocation::from_offsets(0, input.len()),
                ErrorContext::new(input, 0..input.len(), "thread_local"),
                format!("Thread-local parser failed: {err}"),
            ));
            ChatFile::new(Vec::new())
        }
    }
}
