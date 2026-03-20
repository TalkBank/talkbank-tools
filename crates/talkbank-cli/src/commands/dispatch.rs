//! Feature-oriented CLI command routing.
//!
//! This keeps `cli/run.rs` as a small composition root and lets each command family own
//! its own dispatch rules and shared runtime context.

use crate::cli::Commands;
use crate::ui::Theme;

use super::cache::run_cache_command;
use super::run_lsp;
use super::validate::{
    ValidateCommandExecution, ValidateCommandOptions, ValidateCommandPresentation,
    ValidateCommandRules, run_validate_command,
};
use super::{
    AlignmentValidationMode, CacheRefreshMode, RoundtripValidationMode, ValidationInterface,
    chat_to_json, clean_file, create_new_file, json_to_chat, lint_files, normalize_chat, run_clan,
    run_schema, show_alignment, watch_files,
};

/// Runtime context shared across top-level CLI command families.
#[derive(Clone)]
pub struct CommandContext {
    /// Whether the current invocation should prefer the interactive TUI surface.
    pub should_use_tui: bool,
    /// Loaded TUI color theme.
    pub theme: Theme,
}

/// One feature family that owns a subset of top-level CLI commands.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommandFamily {
    Validation,
    Utility,
    Cache,
    Lsp,
    Clan,
    Debug,
}

trait CommandFamilyService {
    fn dispatch(&self, command: Commands, context: &CommandContext);
}

struct CommandServices {
    validation: ValidationCommandService,
    utility: UtilityCommandService,
    cache: CacheCommandService,
    lsp: LspCommandService,
    clan: ClanCommandService,
    debug: DebugCommandService,
}

impl CommandServices {
    const fn new() -> Self {
        Self {
            validation: ValidationCommandService,
            utility: UtilityCommandService,
            cache: CacheCommandService,
            lsp: LspCommandService,
            clan: ClanCommandService,
            debug: DebugCommandService,
        }
    }

    fn dispatch(&self, command: Commands, context: &CommandContext) {
        match command.family() {
            CommandFamily::Validation => self.validation.dispatch(command, context),
            CommandFamily::Utility => self.utility.dispatch(command, context),
            CommandFamily::Cache => self.cache.dispatch(command, context),
            CommandFamily::Lsp => self.lsp.dispatch(command, context),
            CommandFamily::Clan => self.clan.dispatch(command, context),
            CommandFamily::Debug => self.debug.dispatch(command, context),
        }
    }
}

impl Commands {
    const fn family(&self) -> CommandFamily {
        match self {
            Self::Validate { .. }
            | Self::ShowAlignment { .. }
            | Self::Watch { .. }
            | Self::Lint { .. } => CommandFamily::Validation,
            Self::Normalize { .. }
            | Self::ToJson { .. }
            | Self::FromJson { .. }
            | Self::Clean { .. }
            | Self::NewFile { .. }
            | Self::Schema { .. } => CommandFamily::Utility,
            Self::Cache { .. } => CommandFamily::Cache,
            Self::Lsp => CommandFamily::Lsp,
            Self::Clan { .. } => CommandFamily::Clan,
            Self::Debug { .. } => CommandFamily::Debug,
        }
    }
}

/// Dispatch one parsed top-level CLI command to its owning feature family.
pub fn dispatch_command(command: Commands, context: &CommandContext) {
    CommandServices::new().dispatch(command, context);
}

struct ValidationCommandService;

impl CommandFamilyService for ValidationCommandService {
    fn dispatch(&self, command: Commands, context: &CommandContext) {
        match command {
            Commands::Validate {
                path,
                format,
                skip_alignment,
                force,
                jobs,
                quiet,
                max_errors,
                roundtrip,
                parser,
                audit,
            } => run_validate_command(
                path,
                ValidateCommandOptions {
                    rules: ValidateCommandRules {
                        alignment: AlignmentValidationMode::from_enabled(!skip_alignment),
                        roundtrip: RoundtripValidationMode::from_enabled(roundtrip),
                        parser_kind: parser.into(),
                    },
                    execution: ValidateCommandExecution {
                        cache_refresh: CacheRefreshMode::from_force(force),
                        jobs,
                        max_errors,
                    },
                    presentation: ValidateCommandPresentation {
                        format,
                        quiet,
                        audit_output: audit,
                        interface: ValidationInterface::from_tui(context.should_use_tui),
                        theme: context.theme.clone(),
                    },
                },
            ),
            Commands::ShowAlignment {
                input,
                tier,
                compact,
            } => show_alignment(&input, tier, compact),
            Commands::Watch {
                path,
                skip_alignment,
                clear,
            } => {
                if let Err(err) = watch_files(&path, !skip_alignment, true, clear) {
                    eprintln!("Error: {}", err);
                    std::process::exit(1);
                }
            }
            Commands::Lint {
                path,
                fix,
                dry_run,
                skip_alignment,
            } => lint_files(&path, fix, dry_run, true, !skip_alignment),
            _ => unreachable!("validation service received unsupported command"),
        }
    }
}

struct UtilityCommandService;

impl CommandFamilyService for UtilityCommandService {
    fn dispatch(&self, command: Commands, _context: &CommandContext) {
        match command {
            Commands::Normalize {
                input,
                output,
                validate,
                skip_alignment,
            } => normalize_chat(&input, output.as_ref(), validate, skip_alignment),
            Commands::ToJson {
                input,
                output,
                pretty,
                validate: _,
                alignment: _,
                skip_alignment,
                skip_validation,
                skip_schema_validation,
            } => {
                let do_validate = !skip_validation;
                let run_alignment = !skip_alignment && !skip_validation;
                chat_to_json(
                    &input,
                    output.as_ref(),
                    pretty,
                    do_validate,
                    run_alignment,
                    skip_schema_validation,
                );
            }
            Commands::FromJson { input, output } => json_to_chat(&input, output.as_ref()),
            Commands::Clean {
                path,
                diff_only,
                format,
            } => clean_file(&path, diff_only, format),
            Commands::NewFile {
                output,
                speaker,
                language,
                role,
                corpus,
                utterance,
            } => create_new_file(
                output.as_deref(),
                &speaker,
                &language,
                &role,
                &corpus,
                utterance.as_deref(),
            ),
            Commands::Schema { url } => run_schema(url),
            _ => unreachable!("utility service received unsupported command"),
        }
    }
}

struct CacheCommandService;

impl CommandFamilyService for CacheCommandService {
    fn dispatch(&self, command: Commands, _context: &CommandContext) {
        match command {
            Commands::Cache { command } => run_cache_command(command),
            _ => unreachable!("cache service received unsupported command"),
        }
    }
}

struct LspCommandService;

impl CommandFamilyService for LspCommandService {
    fn dispatch(&self, command: Commands, _context: &CommandContext) {
        match command {
            Commands::Lsp => run_lsp(),
            _ => unreachable!("lsp service received unsupported command"),
        }
    }
}

struct ClanCommandService;

impl CommandFamilyService for ClanCommandService {
    fn dispatch(&self, command: Commands, _context: &CommandContext) {
        match command {
            Commands::Clan { command } => run_clan(command),
            _ => unreachable!("clan service received unsupported command"),
        }
    }
}

struct DebugCommandService;

impl CommandFamilyService for DebugCommandService {
    fn dispatch(&self, command: Commands, _context: &CommandContext) {
        match command {
            Commands::Debug { command } => run_debug(command),
            _ => unreachable!("debug service received unsupported command"),
        }
    }
}

fn run_debug(command: crate::cli::DebugCommands) {
    use crate::cli::DebugCommands;
    match command {
        DebugCommands::OverlapAudit {
            path,
            format: _,
            database,
        } => {
            super::debug::run_overlap_audit(&path, database.as_deref());
        }
    }
}
