//! Shared backend state for document caches, language services, and validation bookkeeping.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tower_lsp::Client;
use tower_lsp::lsp_types::Url;
use tree_sitter::Tree;

use talkbank_model::model::ChatFile;

use super::language_services::LanguageServices;
use crate::backend::validation_cache::ValidationCache;

/// Initialization failures for backend subsystems.
#[derive(Clone, Debug, thiserror::Error)]
pub enum BackendInitError {
    /// Tree-sitter parser failed to initialize.
    #[error("failed to initialize tree-sitter parser: {0}")]
    Parser(String),
    /// Semantic tokens provider failed to initialize.
    #[error("failed to initialize semantic tokens provider: {0}")]
    SemanticTokens(String),
}

/// Mutable server-wide state shared across LSP request/notification handlers.
#[derive(Clone)]
pub struct Backend {
    /// LSP client for sending notifications
    pub client: Client,

    /// Cache of document contents (URI -> text)
    pub documents: Arc<DashMap<Url, String>>,

    /// Thread-local parser and semantic-token services.
    pub(crate) language_services: LanguageServices,

    /// Pending validation IDs per document (for debouncing)
    pub pending_validations: Arc<DashMap<Url, u64>>,

    /// Counter for generating unique validation IDs
    pub validation_counter: Arc<AtomicU64>,

    /// Cache of parse trees per document (for incremental parsing)
    pub parse_trees: Arc<DashMap<Url, Tree>>,

    /// Cache of parsed ChatFiles per document (for incremental features)
    ///
    /// This is updated alongside parse_trees and provides quick access
    /// to the parsed model for hover, completion, and other features
    /// without needing to re-parse.
    pub chat_files: Arc<DashMap<Url, Arc<ChatFile>>>,

    /// Track whether the last parse had no syntax errors (per document).
    pub parse_clean: Arc<DashMap<Url, bool>>,

    /// Cache of validation errors per document (for incremental validation).
    pub validation_cache: Arc<DashMap<Url, ValidationCache>>,

    /// Cache of last-published LSP diagnostics per document (for pull model).
    pub last_diagnostics: Arc<DashMap<Url, Vec<tower_lsp::lsp_types::Diagnostic>>>,
}

impl Backend {
    /// Create a backend with initialized language services and empty caches.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(DashMap::new()),
            language_services: LanguageServices::new(),
            pending_validations: Arc::new(DashMap::new()),
            validation_counter: Arc::new(AtomicU64::new(0)),
            parse_trees: Arc::new(DashMap::new()),
            chat_files: Arc::new(DashMap::new()),
            parse_clean: Arc::new(DashMap::new()),
            validation_cache: Arc::new(DashMap::new()),
            last_diagnostics: Arc::new(DashMap::new()),
        }
    }
}
