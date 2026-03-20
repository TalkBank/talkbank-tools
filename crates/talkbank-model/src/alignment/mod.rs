//! Tier alignment module
//!
//! This module provides 1-1 alignment between tiers:
//! - Main tier → %mor tier items
//! - Main tier → %pho/%mod tier tokens (PhoAlignment)
//! - Main tier → %sin tier tokens
//! - %mor tier chunks → %gra tier relations
//!
//! # Alignment Rules
//!
//! ## Main → %mor Alignment
//! - Each alignable unit in main tier corresponds to exactly one %mor item
//! - Alignable content: words, tag markers, groups (recursively), replacements
//! - Excluded: retraces, repeats, scoped annotations themselves
//!
//! ## Main → %pho/%mod Alignment
//! - Each alignable unit in main tier corresponds to exactly one token
//! - Alignable content: same as %mor alignment (words, tag markers, groups, replacements)
//! - %pho = actual pronunciation, %mod = target pronunciation
//! - Both use PhoAlignment type and align_main_to_pho function
//!
//! ## Main → %wor Alignment
//! - Each alignable unit in main tier corresponds to exactly one %wor token
//! - Uses Wor domain (includes retraced words — they were spoken)
//! - Uses its own WorAlignment type (not PhoAlignment)
//!
//! ## Main → %sin Alignment
//! - Each alignable unit in main tier corresponds to exactly one gesture/sign token
//! - Alignable content: same as %pho alignment
//!
//! ## %mor → %gra Alignment
//! - Each %mor chunk corresponds to exactly one %gra relation
//! - Chunks != Items: clitics create additional chunks!
//! - Example: `pro|it~v|be&PRES` creates 2 chunks (pre-clitic + main)
//!
//! # Example
//!
//! ```ignore
//! use talkbank_model::{MainTier, MorTier, Terminator, UtteranceContent, Word};
//! use talkbank_model::Span;
//! use talkbank_parser::parse_mor_tier;
//! use talkbank_model::alignment::align_main_to_mor;
//!
//! let main = MainTier::new(
//!     "CHI",
//!     vec![
//!         UtteranceContent::Word(Word::new_unchecked("hello", "hello")),
//!         UtteranceContent::Word(Word::new_unchecked("world", "world")),
//!     ],
//!     Terminator::Period { span: Span::DUMMY },
//! );
//!
//! let mor = match parse_mor_tier("v|hello n|world .") {
//!     Ok(mor) => mor,
//!     Err(_) => return,
//! };
//!
//! let alignment = align_main_to_mor(&main, &mor);
//! assert_eq!(alignment.pairs.len(), 3); // 2 words + 1 terminator
//! assert!(alignment.errors.is_empty());
//! ```
//!
//! # Implementation Notes
//!
//! - Alignment counts alignable content recursively
//! - %wor alignment mirrors %pho alignment rules
//! - New tiers must update `AlignmentSet` units and tests
//!
//! # Parse-Health Gating Rules
//!
//! - Alignment must consult utterance parse-health before reporting mismatch errors
//! - If a dependent tier domain is parse-tainted, suppress mismatch diagnostics for that domain pair
//! - Main-tier taint blocks main-dependent alignments but not unrelated dependent-dependent checks
//!   (e.g., %mor <-> %gra)
//! - Add targeted tests whenever alignment gating semantics change
//!
//! References:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Word_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Morphological_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#GrammaticalRelations_Tier>

mod format;
mod gra;
pub mod helpers;
mod mor;
mod pho;
mod sin;
pub mod traits;
mod types;
mod wor;

#[cfg(test)]
mod location_tests;

// Re-export public API
pub use gra::{GraAlignment, GraAlignmentPair, align_mor_to_gra};
pub use helpers::{TierDomain, count_tier_positions_until};
pub use mor::{MorAlignment, align_main_to_mor};
pub use pho::{PhoAlignment, align_main_to_pho};
pub use sin::{SinAlignment, align_main_to_sin};
pub use traits::{
    TierCountable, AlignableTier, IndexPair, MismatchFormat, TierAlignmentResult,
    positional_align,
};
pub use types::AlignmentPair;
pub use wor::{WorAlignment, align_main_to_wor};
