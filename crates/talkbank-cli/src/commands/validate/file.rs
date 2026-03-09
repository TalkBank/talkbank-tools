//! Single-file validation flow with error-sink wiring.
//!
//! Implements `chatter validate <file>` by composing the cache, parser, validator,
//! and output layers. Three output modes are supported: streaming terminal (text),
//! structured JSON, and interactive TUI with rerun capability.
//!
//! # Error-sink architecture
//!
//! In text mode, a [`TeeErrorSink`] mirrors each error to both a [`TerminalErrorSink`]
//! (for immediate display) and an [`ErrorCollector`] (for caching and exit-code logic).
//! JSON and TUI modes collect into an [`ErrorCollector`] alone and format after the parse
//! completes.

use std::fs;
use std::path::PathBuf;

use talkbank_model::{ErrorCollector, ParseValidateOptions, TeeErrorSink};
use talkbank_transform::parse_and_validate_streaming;

use crate::cli::OutputFormat;
use crate::commands::{AlignmentValidationMode, CacheRefreshMode, ValidationInterface};
use crate::output::TerminalErrorSink;
use crate::ui::{FileErrors, Theme, TuiAction, run_validation_tui};

use super::cache::{get_cached_validation, initialize_validation_cache, set_cached_validation};
use super::output::output_validation_result;

/// Validate a single CHAT file with optional alignment and caching behavior.
///
/// This routine encapsulates the CLI behavior for the `validate` subcommand when the target
/// path is a single file. It manages the shared `UnifiedCache`, optionally purges entries when
/// `--force` is provided, reads the CHAT content, and builds `ParseValidateOptions` that align
/// with the Main Tier and Dependent Tier rules described in the CHAT manual. Errors are streamed
/// through the appropriate sinks (JSON, TUI, or terminal).
pub fn validate_file(
    path: &PathBuf,
    format: OutputFormat,
    alignment: AlignmentValidationMode,
    cache_refresh: CacheRefreshMode,
    quiet: bool,
    interface: ValidationInterface,
    theme: Theme,
) {
    let check_alignment = alignment.enabled();

    let cache = initialize_validation_cache(path, cache_refresh);

    // Try to get cached results.
    // On Some(true): cached valid — skip revalidation.
    // On Some(false) or None: revalidate.
    if get_cached_validation(cache.as_ref(), path, check_alignment) == Some(true) {
        // Cached success: output and return without revalidating.
        output_validation_result(path, &[], None, format, true, quiet);
        return;
    }

    // Not in cache or cache disabled - validate file
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file {:?}: {}", path, e);
            std::process::exit(1);
        }
    };

    // Build pipeline options (alignment adds `%wor`/`%pho` checks while validation-only skips them).
    // Alignment is on by default; use --skip-alignment to disable
    let options = if check_alignment {
        ParseValidateOptions::default().with_alignment()
    } else {
        ParseValidateOptions::default().with_validation()
    };

    // Use different error sinks based on output format and TUI mode
    // JSON/TUI needs structured values, interactive CLI streams to terminal plus collecting.
    let mut errors = if matches!(format, OutputFormat::Json) || interface.uses_tui() {
        // JSON mode or TUI mode: collect errors for structured output or TUI display
        let error_sink = ErrorCollector::new();

        match parse_and_validate_streaming(&content, options.clone(), &error_sink) {
            Ok(_) => error_sink.into_vec(),
            Err(e) => {
                if matches!(format, OutputFormat::Json) {
                    let json_output = serde_json::json!({
                        "file": path.to_string_lossy(),
                        "status": "error",
                        "error": format!("{}", e)
                    });
                    match serde_json::to_string_pretty(&json_output) {
                        Ok(serialized) => println!("{}", serialized),
                        Err(err) => {
                            eprintln!("Error serializing JSON output: {}", err);
                        }
                    }
                } else {
                    eprintln!("Error: {}", e);
                }
                std::process::exit(1);
            }
        }
    } else {
        // Interactive mode (not TUI): stream errors immediately to terminal AND collect for caching/output
        let terminal_sink = TerminalErrorSink::new(path, &content);
        let collecting_sink = ErrorCollector::new();
        let tee_sink = TeeErrorSink::new(&terminal_sink, &collecting_sink);

        match parse_and_validate_streaming(&content, options.clone(), &tee_sink) {
            Ok(_) => collecting_sink.into_vec(),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    };

    // Enhance errors with source context for proper miette display (TUI, JSON output, etc.)
    talkbank_model::enhance_errors_with_source(&mut errors, &content);

    // Cache the results (pass/fail only)
    set_cached_validation(cache.as_ref(), path, check_alignment, errors.is_empty());

    // TUI mode: Launch interactive error browser with rerun support (uses `termion` UI state).
    if interface.uses_tui() {
        loop {
            if !errors.is_empty() {
                let file_errors = FileErrors {
                    path: path.clone(),
                    errors: errors.clone(),
                    source: content.clone().into(),
                };

                match run_validation_tui(vec![file_errors], theme.clone()) {
                    Ok(TuiAction::Quit) => {
                        if !errors.is_empty() {
                            std::process::exit(1);
                        }
                        return;
                    }
                    Ok(TuiAction::ForceQuit) => {
                        std::process::exit(130);
                    }
                    Ok(TuiAction::Rerun) => {
                        // Re-run validation to get fresh results
                        println!("Re-running validation...");

                        // Re-read file
                        let content = match fs::read_to_string(path) {
                            Ok(c) => c,
                            Err(e) => {
                                eprintln!("Error reading file {:?}: {}", path, e);
                                std::process::exit(1);
                            }
                        };

                        // Re-validate
                        let error_sink = ErrorCollector::new();
                        match parse_and_validate_streaming(&content, options.clone(), &error_sink) {
                            Ok(_) => {
                                errors = error_sink.into_vec();
                                // Enhance errors with source context
                                talkbank_model::enhance_errors_with_source(&mut errors, &content);
                            }
                            Err(e) => {
                                eprintln!("Error: {}", e);
                                std::process::exit(1);
                            }
                        }

                        // Continue loop to show updated TUI
                    }
                    Err(e) => {
                        eprintln!("TUI error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                println!("✓ No errors found in {}", path.display());
                return;
            }
        }
    }

    // Regular output mode
    let source_for_print = if matches!(format, OutputFormat::Text) {
        Some(content.as_str())
    } else {
        None
    };

    // If we are in text mode, we already streamed errors via TerminalErrorSink.
    // If there are errors, we don't need to print them again via output_validation_result.
    if matches!(format, OutputFormat::Text) && !errors.is_empty() {
        std::process::exit(1);
    }

    output_validation_result(path, &errors, source_for_print, format, false, quiet);
    if !errors.is_empty() {
        std::process::exit(1);
    }
}
