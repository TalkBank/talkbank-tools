//! Hosts the FA-specific raw-response types (`FaRawToken`,
//! `FaIndexedTiming`, `FaRawResponse`) used by Whisper-style alignment.
//! Everything else re-exports `batchalign_transform::morphosyntax`; new
//! consumers should import UD/mapping items from `batchalign_transform`
//! directly.

pub mod mapping;
mod types;

pub use batchalign_transform::morphosyntax::*;
pub use types::{FaIndexedTiming, FaRawResponse, FaRawToken};
