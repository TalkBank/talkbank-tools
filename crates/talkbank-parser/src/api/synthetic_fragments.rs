//! Synthetic tree-sitter fragment helpers.
//!
//! These helpers exist only for compatibility and audit workflows that still
//! want the tree-sitter parser to parse isolated fragments by wrapping them in
//! minimal synthetic CHAT context first.
//!
//! They are **not** the semantic oracle for fragment parsing. New fragment
//! semantics and recovery work should target `talkbank-direct-parser`.

pub use super::file::{parse_utterance, parse_utterance_with_context};
pub use super::main_tier::{parse_main_tier, parse_main_tier_with_context, parse_word};
