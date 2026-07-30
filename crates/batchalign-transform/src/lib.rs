#![warn(missing_docs)]
// Test code is exempt from this crate's `deny`-level panic lints: assertion
// macros panic by design.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented
    )
)]
//! Focused transform building blocks for CHAT file processing.
//!
//! This crate exposes many leaf modules, but the crate root keeps a smaller
//! convenience surface for the most common pipeline entry points. Specialized
//! behavior continues to live in its owning module namespace (`json`,
//! `corpus`, `validation_runner`, `xml`, and so on).
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//!
//! ## Top-level entry points
//!
//! - Root re-exports such as [`parse_and_validate`] and [`normalize_chat`] are
//!   the common one-shot pipeline helpers.
//! - [`json`] and [`xml`] own the format-conversion surfaces.
//! - [`corpus`], [`unified_cache`], and [`validation_runner`] own discovery,
//!   caching, and directory-scale validation workflows.
//!
//! # Design Principles
//!
//! - Streaming entry points require `ErrorSink` for diagnostics
//! - Cache paths are shared across tools for consistency
//!
//! # Examples
//!
//! ```no_run
//! use talkbank_transform::{parse_and_validate, PipelineError};
//! use talkbank_model::ParseValidateOptions;
//!
//! let content = "@UTF8\n@Begin\n@Languages:\teng\n@Participants:\tCHI Child\n\
//!     @ID:\teng|corpus|CHI|||||Child|||\n*CHI:\thello .\n@End\n";
//! let options = ParseValidateOptions::default().with_validation();
//! let chat_file = parse_and_validate(content, options).unwrap();
//! assert_eq!(chat_file.utterances().count(), 1);
//! ```

// Generic CHAT transform surface: single home is chatter's talkbank-transform,
// re-exported so the batchalign-specific modules below (which reach generic
// helpers via `crate::parse`, `crate::extract`, `crate::dependent_tiers`, ...)
// and downstream consumers reach the whole generic surface through one root.
pub use talkbank_transform::*;

// The `unified_cache` compatibility shim was deleted on 2026-07-30. It existed
// to keep historical `batchalign_transform::unified_cache::...` references
// resolving after chatter moved caching into `talkbank-cache`, and a search of
// the whole workspace found ZERO such references: it had no consumers at any
// point after it was written.
//
// It was also the one thing here that would have broken if chatter put its
// cache surface behind a feature, because it names those four types EXPLICITLY.
// The blanket `pub use talkbank_transform::*` above is a glob and simply
// re-exports fewer names when a feature is off, so with the shim gone this
// crate compiles against chatter either way.

// Batchalign-specific transforms. These need ML-pipeline context (ASR output,
// neural morphotag, forced-alignment decisions, utterance segmentation), so
// they live on the Batchalign side, NOT in the generic talkbank-transform crate.
pub mod asr_postprocess;
pub mod benchmark;
pub mod build_chat;
pub mod compare;
pub mod constituency;
pub mod coref;
pub mod decisions;
pub mod diff;
pub mod dp_align;
pub mod inject;
pub mod merge_abbrev;
pub mod morphosyntax;
pub mod retokenize;
pub mod tokenizer_realign;
pub mod translate;
pub mod utseg;
pub mod utseg_compute;
pub mod wer_conform;

// The generic convenience re-exports (corpus, json, pipeline, rendering,
// caching, validation_runner) now come from chatter via the `pub use
// talkbank_transform::*` glob above. Only the batchalign-specific convenience
// re-export stays here.
pub use self::merge_abbrev::{merge_abbreviations, merge_abbreviations_in_chat_text};
