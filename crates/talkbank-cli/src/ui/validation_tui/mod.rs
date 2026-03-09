//! Interactive TUI for validation error browsing with CLAN integration.
//!
//! Displays validation errors in a two-pane layout:
//! - Left: File list with error counts
//! - Right: Error details for selected file with source context
//!
//! Keyboard controls:
//! - Tab: Switch between file list and error list
//! - j/k or ↑/↓: Navigate within pane
//! - Enter: Open selected error in CLAN (via send2clan)
//! - r: Re-run validation
//! - q or Esc: Quit
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

mod models;
mod rendering;
mod state;
mod text_processing;

use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, poll},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::Style,
    widgets::{Block, Borders, Paragraph},
};
use std::io;
use std::time::Duration;
use talkbank_transform::validation_runner::ValidationEvent;

use crate::ui::Theme;

/// Return value from TUI indicating user action.
#[derive(Debug)]
pub enum TuiAction {
    /// User quit normally
    Quit,
    /// User requested immediate process termination
    ForceQuit,
    /// User requested rerun validation
    Rerun,
}

pub use models::FileErrors;

use rendering::{
    render_error_details, render_file_list, render_footer, render_footer_streaming, render_header,
    render_header_streaming,
};
use state::TuiState;

/// Launch the validation TUI.
pub fn run_validation_tui(mut files: Vec<FileErrors>, theme: Theme) -> Result<TuiAction> {
    if files.is_empty() {
        println!("✓ No errors found!");
        return Ok(TuiAction::Quit);
    }

    // Ensure all errors have line/column information
    for file in &mut files {
        file.ensure_line_columns();
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut state = TuiState::new(files, theme);

    // Main event loop
    let result = run_static_app(&mut terminal, &mut state);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

/// Launch the validation TUI with streaming error display.
///
/// Errors appear in real-time as validation progresses. User can cancel validation
/// by pressing 'c' or Ctrl+C. Files are kept sorted alphabetically.
pub fn run_validation_tui_streaming(
    events_rx: crossbeam_channel::Receiver<ValidationEvent>,
    cancel_tx: crossbeam_channel::Sender<()>,
    theme: Theme,
) -> Result<TuiAction> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state with empty file list (will populate as errors arrive)
    let mut state = TuiState::new(Vec::new(), theme);
    let mut validation_complete = false;
    let mut ctrl_c_count = 0usize;

    // Main event loop with non-blocking polls
    let result = loop {
        // Draw UI
        terminal.draw(|f| ui_streaming(f, &mut state, validation_complete))?;

        // Poll for keyboard input (non-blocking, 50ms timeout)
        if poll(Duration::from_millis(50))?
            && let Event::Key(key) = event::read()?
        {
            match (key.code, key.modifiers) {
                // Cancel validation
                (KeyCode::Char('c'), KeyModifiers::NONE) => {
                    cancel_tx.send(()).ok();
                }
                // Quit immediately on Ctrl+C
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    ctrl_c_count += 1;
                    cancel_tx.send(()).ok();
                    if ctrl_c_count >= 2 {
                        break Ok(TuiAction::ForceQuit);
                    }
                }
                // Quit
                (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Esc, _) => {
                    cancel_tx.send(()).ok();
                    break Ok(TuiAction::Quit);
                }
                // Navigate up
                (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, _) => {
                    state.move_up();
                }
                // Navigate down
                (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, _) => {
                    state.move_down();
                }
                // Toggle focus
                (KeyCode::Tab, _) => {
                    state.toggle_focus();
                }
                // Open in CLAN
                (KeyCode::Enter, _) => {
                    if let Err(e) = state.open_in_clan() {
                        let _ = e;
                    }
                }
                // Rerun validation (only after completion)
                (KeyCode::Char('r'), KeyModifiers::NONE) if validation_complete => {
                    break Ok(TuiAction::Rerun);
                }
                _ => {}
            }
        }

        // Drain all pending validation events (non-blocking)
        loop {
            match events_rx.try_recv() {
                Ok(ValidationEvent::Errors(mut error_event)) => {
                    // Enhance errors with full line context for miette display
                    talkbank_model::enhance_errors_with_source(
                        &mut error_event.errors,
                        &error_event.source,
                    );

                    // Check if this file already exists in the list
                    if let Some(existing) =
                        state.files.iter_mut().find(|f| f.path == error_event.path)
                    {
                        // File already exists - merge errors
                        existing.errors.extend(error_event.errors);
                    } else {
                        // New file with errors - add to list
                        let mut file_errors = FileErrors {
                            path: error_event.path,
                            errors: error_event.errors,
                            source: error_event.source,
                        };

                        // Ensure line/column information
                        file_errors.ensure_line_columns();

                        // Add to state
                        state.files.push(file_errors);

                        // Keep files sorted alphabetically
                        state.files.sort_by(|a, b| a.path.cmp(&b.path));

                        // Update selection if this is the first file
                        if state.files.len() == 1 {
                            state.file_list_state.select(Some(0));
                            if !state.files[0].errors.is_empty() {
                                state.error_list_state.select(Some(0));
                            }
                        }
                    }
                }
                Ok(ValidationEvent::Discovering) => {
                    state.discovering = true;
                }
                Ok(ValidationEvent::Started { total_files }) => {
                    state.total_files = total_files;
                    state.discovering = false;
                }
                Ok(ValidationEvent::RoundtripComplete(_)) => {
                    // Roundtrip events handled via FileComplete status
                }
                Ok(ValidationEvent::FileComplete(_)) => {
                    state.files_processed += 1;
                    state.update_progress_display(false);
                }
                Ok(ValidationEvent::Finished(snapshot)) => {
                    validation_complete = true;
                    state.total_files = snapshot.total_files;
                    state.files_processed = snapshot.total_files;
                    state.update_progress_display(true);
                    state.final_valid_files = Some(snapshot.valid_files);
                    state.final_invalid_files = Some(snapshot.invalid_files);
                    state.final_cache_hits = Some(snapshot.cache_hits);
                    state.final_cache_misses = Some(snapshot.cache_misses);
                }
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    validation_complete = true;
                    break;
                }
            }
        }
    };

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

/// Run the main event loop for static validation.
fn run_static_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut TuiState,
) -> Result<TuiAction>
where
    <B as ratatui::backend::Backend>::Error: 'static + std::error::Error + Send + Sync,
{
    loop {
        terminal.draw(|f| ui(f, state))?;

        if let Event::Key(key) = event::read()? {
            match (key.code, key.modifiers) {
                // Quit
                (KeyCode::Char('q'), KeyModifiers::NONE) | (KeyCode::Esc, _) => {
                    return Ok(TuiAction::Quit);
                }
                // Navigate up
                (KeyCode::Char('k'), KeyModifiers::NONE) | (KeyCode::Up, _) => {
                    state.move_up();
                }
                // Navigate down
                (KeyCode::Char('j'), KeyModifiers::NONE) | (KeyCode::Down, _) => {
                    state.move_down();
                }
                // Toggle focus
                (KeyCode::Tab, _) => {
                    state.toggle_focus();
                }
                // Open in CLAN
                (KeyCode::Enter, _) => {
                    if let Err(e) = state.open_in_clan() {
                        eprintln!("Failed to open in CLAN: {}", e);
                    }
                }
                // Rerun validation
                (KeyCode::Char('r'), KeyModifiers::NONE) => {
                    return Ok(TuiAction::Rerun);
                }
                // Quit with Ctrl+C
                (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                    return Ok(TuiAction::Quit);
                }
                _ => {}
            }
        }
    }
}

/// UI rendering for streaming validation (shows validation status).
fn ui_streaming(f: &mut Frame, state: &mut TuiState, validation_complete: bool) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Header with title + gauge
            Constraint::Min(0),    // Main content
            Constraint::Length(4), // Footer (action row + nav row)
        ])
        .split(f.area());

    // Render header with validation status
    render_header_streaming(f, chunks[0], state, validation_complete);

    if state.files.is_empty() {
        // No errors yet - show progress message
        let msg = if validation_complete {
            format!(
                "✓ {} files validated, no errors found! Press 'q' to quit.",
                state.total_files
            )
        } else if state.discovering {
            "Discovering files... (press 'c' to cancel)".to_string()
        } else if state.total_files > 0 {
            "Validating files... (press 'c' to cancel)".to_string()
        } else {
            "Validating... (press 'c' to cancel)".to_string()
        };

        let color = if validation_complete {
            state.theme.header_ok
        } else {
            state.theme.header_progress
        };

        let paragraph = Paragraph::new(msg)
            .style(Style::default().fg(color))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));

        f.render_widget(paragraph, chunks[1]);
    } else {
        // Split main content into two panes
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // File list (left)
                Constraint::Percentage(70), // Error details (right)
            ])
            .split(chunks[1]);

        // Render file list
        render_file_list(f, main_chunks[0], state);

        // Render error details
        render_error_details(f, main_chunks[1], state);
    }

    // Render footer with streaming-specific controls
    render_footer_streaming(f, chunks[2], state, validation_complete);
}

/// UI rendering for static validation.
fn ui(f: &mut Frame, state: &mut TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(4), // Footer (action row + nav row)
        ])
        .split(f.area());

    // Render header
    render_header(f, chunks[0], state);

    // Split main content into two panes
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // File list (left)
            Constraint::Percentage(70), // Error details (right)
        ])
        .split(chunks[1]);

    // Render file list
    render_file_list(f, main_chunks[0], state);

    // Render error details
    render_error_details(f, main_chunks[1], state);

    // Render footer
    render_footer(f, chunks[2], state);
}
