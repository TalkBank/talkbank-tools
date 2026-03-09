//! Public API for TalkBank parsing
//!
//! This module provides the main user-facing parsing functions organized by level:
//! - File-level: `parse_chat_file()`, `parse_utterance()`, `parse_utterance_with_context()`
//! - Tier-level: `parse_header()`, `parse_main_tier()`, `parse_main_tier_with_context()`, `parse_word()`
//! - Granular tier parsing: `tiers::*` modules organized by linguistic category
//!
//! CHAT reference anchors:
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//!
//! # Examples
//!
//! Parse a complete CHAT file:
//! ```
//! use talkbank_parser::parse_chat_file;
//!
//! let input = "@Begin\n*CHI:\thello .\n@End";
//! let result = parse_chat_file(input);
//! # let _ = result;
//! ```
//!
//! Parse a specific tier type using the parse_dependent_tier function:
//! ```
//! use talkbank_parser::parse_dependent_tier;
//! use talkbank_model::ErrorCollector;
//! use talkbank_model::ParseOutcome;
//!
//! let errors = ErrorCollector::new();
//! let result = parse_dependent_tier("%mor:\tn|hello det|the .", &errors);
//! assert!(matches!(result, ParseOutcome::Parsed(_)));
//! # let _ = result;
//! ```

pub mod dependent_tier;
pub mod file;
pub mod header;
pub mod main_tier;
mod parser_api;
mod parser_impl;
pub mod tiers;

// Re-export main parsing functions at module level
pub use dependent_tier::parse_dependent_tier;
pub use file::{parse_chat_file, parse_utterance, parse_utterance_with_context};
pub use header::parse_header;
pub use main_tier::{parse_main_tier, parse_main_tier_with_context, parse_word};
pub use tiers::*;
