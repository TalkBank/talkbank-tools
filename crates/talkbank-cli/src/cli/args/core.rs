use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use super::clan_commands::ClanCommands;

pub use crate::ui::ThemePreset;

/// TalkBank utilities for CHAT format validation and transformation
#[derive(Parser)]
#[command(name = "talkbank", version, long_version = concat!(env!("CARGO_PKG_VERSION"), " (build ", env!("BUILD_HASH"), ")"))]
#[command(about = "Tools for validating and transforming TalkBank CHAT files", long_about = None)]
pub struct Cli {
    /// Logging verbosity level (can be repeated: -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Logging output format
    #[arg(long, value_enum, default_value = "text", global = true)]
    pub log_format: LogFormat,

    /// TUI mode: auto (detect terminal), force (always), disable (never)
    #[arg(long, value_enum, default_value_t, global = true)]
    pub tui_mode: TuiMode,

    /// Color theme for TUI mode
    #[arg(long, value_enum, global = true)]
    pub theme: Option<ThemePreset>,

    #[command(subcommand)]
    pub command: Commands,
}

/// Supported formats for tracing output.
#[derive(Debug, Clone, ValueEnum)]
pub enum LogFormat {
    /// Human-readable text format
    Text,
    /// JSON format for observability/telemetry tools
    Json,
}

/// Controls whether the interactive TUI is used for validation output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum TuiMode {
    /// Automatically detect terminal capability (TUI when stdout is a TTY)
    #[default]
    Auto,
    /// Force TUI mode regardless of terminal detection
    Force,
    /// Disable TUI mode even in interactive terminals
    Disable,
}

impl TuiMode {
    /// Resolve the mode into a concrete decision, consulting the terminal when `Auto`.
    pub fn should_use_tui(self) -> bool {
        match self {
            Self::Force => true,
            Self::Disable => false,
            Self::Auto => atty::is(atty::Stream::Stdout),
        }
    }
}

/// Output encodings for command results.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Human-readable validation output
    Text,
    /// Structured JSON output
    Json,
}

/// Dependent tiers that `show-alignment` can filter on.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum AlignmentTier {
    /// Morphology tier (%mor)
    Mor,
    /// Grammar tier (%gra)
    Gra,
    /// Phonology tier (%pho)
    Pho,
    /// Syntax tier (%sin)
    Sin,
}

/// Top-level `talkbank` subcommands.
#[derive(Subcommand)]
pub enum Commands {
    /// Validate CHAT file(s)
    Validate {
        /// Path to CHAT file or directory
        path: PathBuf,

        /// Output format: text (default) or json
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Text, help = "Validation output style (text|json)")]
        format: OutputFormat,

        /// Skip tier alignment (validation includes alignment by default)
        #[arg(
            long = "skip-alignment",
            help = "Disable dependent tier alignment checks (alignment is on by default)"
        )]
        skip_alignment: bool,

        /// Force fresh validation, clearing and updating cache
        #[arg(
            long,
            help = "Force fresh validation (clears and updates cache for specified path)"
        )]
        force: bool,

        /// Number of parallel jobs (default: number of CPUs)
        #[arg(short, long)]
        jobs: Option<usize>,

        /// Suppress success output (errors still print)
        #[arg(long, help = "Quiet mode (only emit errors, rely on exit codes)")]
        quiet: bool,

        /// Stop after this many errors (across all files)
        #[arg(long)]
        max_errors: Option<usize>,

        /// Run roundtrip test (serialize → re-parse → compare) after validation.
        /// Tests serialization idempotency. Developer tool for parser/serializer testing.
        #[arg(long, help = "Test serialization idempotency (developer tool)")]
        roundtrip: bool,

        /// Parser backend to use for validation.
        /// "tree-sitter" is the canonical parser (default); "direct" is experimental.
        #[arg(long, value_enum, default_value_t = ParserKindArg::TreeSitter)]
        parser: ParserKindArg,

        /// Audit mode: stream errors to JSONL file without caching (for bulk corpus validation).
        /// Reads from cache to skip clean files (fast), but doesn't write new errors to cache (avoids OOM).
        /// Generates summary statistics at the end.
        #[arg(
            long,
            help = "Stream errors to JSONL file (bulk audit mode)",
            value_name = "OUTPUT_FILE"
        )]
        audit: Option<PathBuf>,
    },

    /// Normalize CHAT file to canonical format
    Normalize {
        /// Input CHAT file path
        input: PathBuf,

        /// Output CHAT file path (if not specified, prints to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Validate (includes alignment) before normalization
        #[arg(long, help = "Validate and check alignment before writing output")]
        validate: bool,

        /// Skip alignment when validating
        #[arg(
            long = "skip-alignment",
            help = "Skip alignment checks when --validate is supplied"
        )]
        skip_alignment: bool,
    },

    /// Convert CHAT file to JSON
    #[command(long_about = "Convert a CHAT transcript to JSON.\n\n\
        Output conforms to the TalkBank CHAT JSON Schema:\n\
        https://talkbank.org/schemas/v0.1/chat-file.json\n\n\
        Validation and alignment are enabled by default. Use --skip-alignment \
        to disable alignment, or --skip-validation to disable validation.\n\n\
        By default the output is also validated against the schema. Use \
        --skip-schema-validation to disable this check.")]
    ToJson {
        /// Input CHAT file path
        input: PathBuf,

        /// Output JSON file path (if not specified, prints to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Pretty-print JSON (default: true)
        #[arg(short, long, default_value = "true")]
        pretty: bool,

        /// [Deprecated] Validation is now on by default. This flag is ignored.
        #[arg(long, hide = true)]
        validate: bool,

        /// [Deprecated] Alignment is now on by default. Use --skip-alignment to disable.
        #[arg(short, long, hide = true)]
        alignment: bool,

        /// Skip tier alignment checks
        #[arg(
            long = "skip-alignment",
            help = "Disable tier alignment validation during conversion"
        )]
        skip_alignment: bool,

        /// Skip data model validation (parse only)
        #[arg(
            long = "skip-validation",
            help = "Skip validation of the CHAT data model (parse only, no alignment)"
        )]
        skip_validation: bool,

        /// Skip validation against the CHAT JSON Schema
        #[arg(
            long,
            help = "Skip validation against the CHAT JSON Schema \
            (https://talkbank.org/schemas/v0.1/chat-file.json). \
            Useful for faster output when you trust the data model."
        )]
        skip_schema_validation: bool,
    },

    /// Convert JSON file to CHAT
    #[command(long_about = "Convert a JSON file back to CHAT format.\n\n\
        The input should conform to the TalkBank CHAT JSON Schema:\n\
        https://talkbank.org/schemas/v0.1/chat-file.json\n\n\
        Use `chatter schema` to print the full schema.")]
    FromJson {
        /// Input JSON file path
        input: PathBuf,

        /// Output CHAT file path (if not specified, prints to stdout)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Show alignment visualization for debugging
    ShowAlignment {
        /// Input CHAT file path
        input: PathBuf,

        /// Show alignment for specific tier types (mor, gra, pho, sin)
        /// If not specified, shows all available alignments
        #[arg(short, long, value_enum)]
        tier: Option<AlignmentTier>,

        /// Compact output (one line per alignment)
        #[arg(short, long)]
        compact: bool,
    },

    /// Watch CHAT file(s) for changes and continuously validate
    Watch {
        /// Path to CHAT file or directory to watch
        path: PathBuf,

        /// Skip tier alignment checks
        #[arg(long)]
        skip_alignment: bool,

        /// Clear terminal before each validation run
        #[arg(short, long)]
        clear: bool,
    },

    /// Lint CHAT file(s) and optionally auto-fix issues
    Lint {
        /// Path to CHAT file or directory
        path: PathBuf,

        /// Automatically apply fixes
        #[arg(long)]
        fix: bool,

        /// Show what would be fixed without modifying files
        #[arg(long, requires = "fix")]
        dry_run: bool,

        /// Skip tier alignment checks
        #[arg(long)]
        skip_alignment: bool,
    },

    /// Show cleaned text for each word in utterances (debugging aid)
    Clean {
        /// Input CHAT file path
        path: PathBuf,

        /// Only show words where raw text differs from cleaned text
        #[arg(long)]
        diff_only: bool,

        /// Output format
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },

    /// Create a new minimal valid CHAT file
    NewFile {
        /// Output file path (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Speaker code (default: CHI)
        #[arg(short, long, default_value = "CHI")]
        speaker: String,

        /// ISO 639-3 language code (default: eng)
        #[arg(short, long, default_value = "eng")]
        language: String,

        /// Participant role (default: Target_Child)
        #[arg(short, long, default_value = "Target_Child")]
        role: String,

        /// Corpus identifier (default: corpus)
        #[arg(short, long, default_value = "corpus")]
        corpus: String,

        /// Initial utterance content (optional)
        #[arg(short, long)]
        utterance: Option<String>,
    },

    /// Cache management operations
    Cache {
        #[command(subcommand)]
        command: CacheCommands,
    },

    /// Print the CHAT JSON Schema
    #[command(long_about = "Print the CHAT JSON Schema to stdout.\n\n\
        The schema describes the structure of CHAT transcripts serialized to JSON \
        by `chatter to-json`. It is auto-generated from the Rust data model \
        and conforms to JSON Schema 2020-12.\n\n\
        Canonical URL: https://talkbank.org/schemas/v0.1/chat-file.json")]
    Schema {
        /// Print only the canonical schema URL instead of the full schema
        #[arg(
            long,
            help = "Print only the canonical URL (https://talkbank.org/schemas/v0.1/chat-file.json)"
        )]
        url: bool,
    },

    /// Run the CHAT language server over stdio
    Lsp,

    /// CLAN analysis and transform commands
    #[command(
        about = "CLAN analysis and transform commands for CHAT transcripts",
        long_about = "Run CLAN analysis commands (freq, mlu, mlt, etc.) and transform commands \
            (flo, lowcase, etc.) on CHAT files.\n\n\
            Analysis commands compute statistics and produce text/JSON/CSV output.\n\
            Transform commands modify CHAT files in place or to a new file."
    )]
    Clan {
        #[command(subcommand)]
        command: ClanCommands,
    },

    /// Developer/debugging tools for CHAT analysis
    #[command(about = "Developer tools for inspecting and debugging CHAT files")]
    Debug {
        #[command(subcommand)]
        command: DebugCommands,
    },
}

/// Debug subcommands under `chatter debug`.
#[derive(Subcommand)]
pub enum DebugCommands {
    /// Analyze CA overlap markers (⌈⌉⌊⌋): pairing, temporal consistency, orphans
    OverlapAudit {
        /// Path to CHAT file(s) or directory
        path: Vec<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,

        /// Write JSON lines database to this file (one JSON object per file).
        /// Enables persistent overlap data for downstream analysis.
        #[arg(long, value_name = "PATH")]
        database: Option<PathBuf>,
    },
}

/// Cache maintenance subcommands under `talkbank cache`.
#[derive(Subcommand)]
pub enum CacheCommands {
    /// Display cache statistics
    Stats {
        /// Output JSON format
        #[arg(long)]
        json: bool,
    },

    /// Clear cache entries
    Clear {
        /// Clear all cache entries
        #[arg(long, conflicts_with = "prefix")]
        all: bool,

        /// Clear entries matching this path prefix
        #[arg(long, conflicts_with = "all")]
        prefix: Option<PathBuf>,

        /// Show what would be cleared without actually clearing
        #[arg(long)]
        dry_run: bool,
    },
}

/// Parser backend choice for CLI.
///
/// Maps to `talkbank_transform::ParserKind` via `From` impl.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ParserKindArg {
    /// Tree-sitter parser (default, canonical)
    #[value(name = "tree-sitter")]
    TreeSitter,
    /// Direct chumsky parser (experimental)
    Direct,
}

impl From<ParserKindArg> for talkbank_transform::ParserKind {
    /// Convert the CLI parser selector into the transform-layer parser enum.
    fn from(arg: ParserKindArg) -> Self {
        match arg {
            ParserKindArg::TreeSitter => talkbank_transform::ParserKind::TreeSitter,
            ParserKindArg::Direct => talkbank_transform::ParserKind::Direct,
        }
    }
}
