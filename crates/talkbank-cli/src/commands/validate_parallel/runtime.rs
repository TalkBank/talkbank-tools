//! Standard streamed validation runtime with text, JSON, and TUI frontends.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::commands::validate::cache::initialize_validation_cache;
use crate::commands::validate_parallel::renderer::create_renderer;
use crate::commands::validate_parallel::shared::empty_stats;
use crate::commands::validate_parallel::{
    ValidateDirectoryOptions, ValidationPresentation, ValidationTraversalMode,
};
use crate::ui::{TuiAction, run_validation_tui_streaming};
use talkbank_transform::validation_runner::{
    CacheMode, DirectoryMode, ValidationConfig, ValidationEvent, ValidationStatsSnapshot,
    validate_directory_streaming,
};

/// Run the standard validation flow for one directory or file target.
pub fn run_validation_runtime(
    path: &Path,
    options: ValidateDirectoryOptions,
) -> ValidationStatsSnapshot {
    let ValidateDirectoryOptions {
        rules,
        traversal,
        execution,
        presentation,
        suppress,
    } = options;
    let ValidationPresentation::Streaming(output) = presentation else {
        unreachable!("audit presentation is handled before the runtime entrypoint");
    };

    let suppress_set: std::collections::HashSet<String> = suppress.into_iter().collect();

    let config = ValidationConfig {
        check_alignment: rules.alignment.enabled(),
        jobs: execution.jobs,
        cache: CacheMode::Enabled,
        directory: match traversal {
            ValidationTraversalMode::Recursive => DirectoryMode::Recursive,
            ValidationTraversalMode::SingleFile => DirectoryMode::SingleFile,
        },
        roundtrip: rules.roundtrip.enabled(),
        parser_kind: rules.parser_kind,
    };

    let cache = initialize_validation_cache(path, execution.cache_refresh);

    if output.interface.uses_tui() {
        return run_tui_loop(path, &config, cache, output.theme);
    }

    let (events_rx, cancel_tx) = validate_directory_streaming(path, &config, cache);
    install_ctrlc_handler(&cancel_tx);

    let json_mode = matches!(output.format, crate::cli::OutputFormat::Json);
    let mut renderer = create_renderer(json_mode, output.quiet);
    let mut final_stats = None;
    let mut error_count = 0usize;
    let mut files_completed = 0usize;
    // Track files whose errors were entirely suppressed (for exit code adjustment)
    let mut files_fully_suppressed = 0usize;

    for event in events_rx {
        match event {
            ValidationEvent::Discovering => renderer.handle_discovering(),
            ValidationEvent::Started { total_files } => renderer.handle_started(total_files),
            ValidationEvent::Errors(mut error_event) => {
                // Filter suppressed error codes before rendering
                if !suppress_set.is_empty() {
                    let pre_count = error_event.errors.len();
                    error_event
                        .errors
                        .retain(|e| !suppress_set.contains(e.code.as_str()));
                    if error_event.errors.is_empty() && pre_count > 0 {
                        files_fully_suppressed += 1;
                    }
                }
                if !error_event.errors.is_empty() {
                    error_count = error_count.saturating_add(renderer.handle_errors(&error_event));
                    cancel_if_error_limit_reached(&cancel_tx, execution.max_errors, error_count);
                }
            }
            ValidationEvent::RoundtripComplete(rt_event) => {
                error_count =
                    error_count.saturating_add(renderer.handle_roundtrip_complete(&rt_event));
                cancel_if_error_limit_reached(&cancel_tx, execution.max_errors, error_count);
            }
            ValidationEvent::FileComplete(file_event) => {
                files_completed += 1;
                renderer.handle_file_complete(&file_event, files_completed);
            }
            ValidationEvent::Finished(snapshot) => {
                final_stats = Some(snapshot);
            }
        }
    }

    let stats = match final_stats {
        Some(stats) => stats,
        None => {
            eprintln!("Error: No validation stats received");
            std::process::exit(1);
        }
    };

    // Adjust stats before rendering: files whose errors were entirely
    // suppressed should not count as invalid in the summary or exit code.
    let mut stats = stats;
    if files_fully_suppressed > 0 {
        stats.invalid_files = stats.invalid_files.saturating_sub(files_fully_suppressed);
    }

    renderer.handle_finished(&stats, files_completed, execution.max_errors, error_count);
    renderer.print_summary(path, &stats, rules.roundtrip.enabled());
    stats
}

/// Drive the interactive TUI, supporting reruns until the user exits.
fn run_tui_loop(
    path: &Path,
    config: &ValidationConfig,
    cache: Option<Arc<talkbank_transform::CachePool>>,
    theme: crate::ui::Theme,
) -> ValidationStatsSnapshot {
    loop {
        let (events_rx, cancel_tx) = validate_directory_streaming(path, config, cache.clone());
        match run_validation_tui_streaming(events_rx, cancel_tx, theme.clone()) {
            Ok(TuiAction::Quit) => return empty_stats(false),
            Ok(TuiAction::ForceQuit) => std::process::exit(130),
            Ok(TuiAction::Rerun) => {
                eprintln!("Re-running validation...");
            }
            Err(error) => {
                eprintln!("TUI error: {}", error);
                return empty_stats(true);
            }
        }
    }
}

/// Install the Ctrl+C handler used by non-interactive validation modes.
fn install_ctrlc_handler(cancel_tx: &crossbeam_channel::Sender<()>) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let cancel_count = Arc::new(AtomicUsize::new(0));
    let cancelled_clone = Arc::clone(&cancelled);
    let cancel_count_clone = Arc::clone(&cancel_count);
    let cancel_tx_clone = cancel_tx.clone();

    if let Err(error) = ctrlc::set_handler(move || {
        let count = cancel_count_clone.fetch_add(1, Ordering::SeqCst);
        if count == 0 {
            let was_cancelled = cancelled_clone.swap(true, Ordering::SeqCst);
            cancel_tx_clone.send(()).ok();
            if !was_cancelled {
                eprintln!("\nCancelling validation... (press Ctrl+C again to force quit)");
            }
        } else {
            eprintln!("\nForce quitting.");
            std::process::exit(130);
        }
    }) {
        eprintln!("Error setting Ctrl+C handler: {}", error);
    }
}

/// Cancel the run when the configured error limit has been reached.
fn cancel_if_error_limit_reached(
    cancel_tx: &crossbeam_channel::Sender<()>,
    max_errors: Option<usize>,
    error_count: usize,
) {
    if let Some(limit) = max_errors
        && error_count >= limit
    {
        cancel_tx.send(()).ok();
    }
}
