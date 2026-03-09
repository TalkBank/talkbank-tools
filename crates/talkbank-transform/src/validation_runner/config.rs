//! Validation-runner configuration types.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

/// Which parser backend to use for validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserKind {
    /// Tree-sitter parser (default, canonical)
    TreeSitter,
    /// Direct chumsky parser (experimental)
    Direct,
}

impl ParserKind {
    /// Label used for cache keys (must be stable across runs).
    pub fn cache_label(self) -> &'static str {
        match self {
            ParserKind::TreeSitter => "tree-sitter",
            ParserKind::Direct => "direct",
        }
    }
}

/// Whether the validation cache is enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CacheMode {
    /// Cache validation and roundtrip results (default).
    #[default]
    Enabled,
    /// Skip all cache lookups and writes.
    Disabled,
}

/// Whether to recurse into subdirectories when collecting .cha files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DirectoryMode {
    /// Process only the immediate directory.
    SingleFile,
    /// Recurse into subdirectories (default).
    #[default]
    Recursive,
}

/// Configuration for validation runner
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Check tier alignment (more thorough, slower)
    pub check_alignment: bool,

    /// Number of parallel jobs (None = use all CPUs)
    pub jobs: Option<usize>,

    /// Whether to use the validation cache
    pub cache: CacheMode,

    /// How to traverse directories when collecting .cha files
    pub directory: DirectoryMode,

    /// Run roundtrip test (serialize -> re-parse -> compare) after validation
    pub roundtrip: bool,

    /// Which parser backend to use
    pub parser_kind: ParserKind,
}

impl Default for ValidationConfig {
    /// Create the default validation-runner configuration.
    fn default() -> Self {
        Self {
            check_alignment: true,
            jobs: None,
            cache: CacheMode::Enabled,
            directory: DirectoryMode::Recursive,
            roundtrip: false,
            parser_kind: ParserKind::TreeSitter,
        }
    }
}
