//! TUI state management and navigation.
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use ratatui::widgets::ListState;
use talkbank_model::{ClanHiddenLineError, ParseError, resolve_clan_location};
use thiserror::Error;

use super::models::FileErrors;
use crate::ui::Theme;

/// TUI state.
pub struct TuiState {
    /// Color theme for the TUI.
    pub theme: Theme,
    /// All files with errors
    pub files: Vec<FileErrors>,
    /// Currently selected file index
    pub selected_file_idx: usize,
    /// Currently selected error index within the file
    pub selected_error_idx: usize,
    /// Which pane has focus (file list or error list)
    pub focus: Focus,
    /// File list widget state
    pub file_list_state: ListState,
    /// Error list widget state
    pub error_list_state: ListState,
    /// Total files to validate (from Started event)
    pub total_files: usize,
    /// Files processed so far (incremented on each FileComplete)
    pub files_processed: usize,
    /// Throttled display count to reduce redraw noise
    pub files_processed_display: usize,
    /// True until Started event arrives (file discovery phase)
    pub discovering: bool,
    /// Final snapshot counts from Finished event
    pub final_valid_files: Option<usize>,
    pub final_invalid_files: Option<usize>,
    pub final_cache_hits: Option<usize>,
    pub final_cache_misses: Option<usize>,
}

/// Which pane has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    FileList,
    ErrorList,
}

impl TuiState {
    const PROGRESS_DRAW_STRIDE: usize = 50;

    /// Initialize TUI state from the first validation snapshot.
    ///
    /// If the incoming list is non-empty, both panes start with their first
    /// item selected so keyboard navigation is immediately active.
    pub fn new(files: Vec<FileErrors>, theme: Theme) -> Self {
        let mut file_list_state = ListState::default();
        if !files.is_empty() {
            file_list_state.select(Some(0));
        }

        let mut error_list_state = ListState::default();
        if !files.is_empty() && !files[0].errors.is_empty() {
            error_list_state.select(Some(0));
        }

        Self {
            theme,
            files,
            selected_file_idx: 0,
            selected_error_idx: 0,
            focus: Focus::FileList,
            file_list_state,
            error_list_state,
            total_files: 0,
            files_processed: 0,
            files_processed_display: 0,
            discovering: true,
            final_valid_files: None,
            final_invalid_files: None,
            final_cache_hits: None,
            final_cache_misses: None,
        }
    }

    /// Updates progress display.
    pub fn update_progress_display(&mut self, force: bool) {
        if force
            || (self.files_processed - self.files_processed_display) >= Self::PROGRESS_DRAW_STRIDE
        {
            self.files_processed_display = self.files_processed;
        }
    }

    /// Return the currently selected file entry, if any.
    pub fn current_file(&self) -> Option<&FileErrors> {
        self.files.get(self.selected_file_idx)
    }

    /// Return the currently selected error inside the selected file.
    pub fn current_error(&self) -> Option<&ParseError> {
        self.current_file()?.errors.get(self.selected_error_idx)
    }

    /// Count all errors across currently tracked files.
    pub fn total_errors(&self) -> usize {
        self.files.iter().map(|f| f.errors.len()).sum()
    }

    /// Return the number of files currently in the error list.
    pub fn total_files_with_errors(&self) -> usize {
        self.files.len()
    }

    /// Move selection up in the currently focused pane.
    pub fn move_up(&mut self) {
        match self.focus {
            Focus::FileList => {
                if self.selected_file_idx > 0 {
                    self.selected_file_idx -= 1;
                    self.selected_error_idx = 0; // Reset error selection
                    self.file_list_state.select(Some(self.selected_file_idx));
                    self.error_list_state.select(Some(0));
                }
            }
            Focus::ErrorList => {
                if self.selected_error_idx > 0 {
                    self.selected_error_idx -= 1;
                    self.error_list_state.select(Some(self.selected_error_idx));
                }
            }
        }
    }

    /// Move selection down in the currently focused pane.
    pub fn move_down(&mut self) {
        match self.focus {
            Focus::FileList => {
                if self.selected_file_idx < self.files.len().saturating_sub(1) {
                    self.selected_file_idx += 1;
                    self.selected_error_idx = 0; // Reset error selection
                    self.file_list_state.select(Some(self.selected_file_idx));
                    self.error_list_state.select(Some(0));
                }
            }
            Focus::ErrorList => {
                if let Some(file) = self.current_file()
                    && self.selected_error_idx < file.errors.len().saturating_sub(1)
                {
                    self.selected_error_idx += 1;
                    self.error_list_state.select(Some(self.selected_error_idx));
                }
            }
        }
    }

    /// Toggle focus between file list and error list.
    pub fn toggle_focus(&mut self) {
        self.focus = match self.focus {
            Focus::FileList => Focus::ErrorList,
            Focus::ErrorList => Focus::FileList,
        };
    }

    /// Open the currently selected error in CLAN.
    pub fn open_in_clan(&self) -> Result<(), OpenInClanError> {
        let error = self.current_error().ok_or(OpenInClanError::NoError)?;
        let file = self.current_file().ok_or(OpenInClanError::NoFile)?;

        let clan_loc = resolve_clan_location(&error.location, &file.source)?;

        send2clan::send_to_clan(
            30, // 30 second timeout
            file.path.to_str().ok_or(OpenInClanError::InvalidPath)?,
            clan_loc.line as i32,
            clan_loc.column as i32,
            Some(&error.message),
        )?;

        Ok(())
    }
}

/// Errors raised when forwarding the selected error location to CLAN.
#[derive(Debug, Error)]
pub enum OpenInClanError {
    #[error("No error selected")]
    NoError,
    #[error("No file selected")]
    NoFile,
    #[error("Invalid file path")]
    InvalidPath,
    #[error("{0}")]
    HiddenLine(#[from] ClanHiddenLineError),
    #[error("Failed to send to CLAN")]
    Send(#[from] send2clan::Error),
}
