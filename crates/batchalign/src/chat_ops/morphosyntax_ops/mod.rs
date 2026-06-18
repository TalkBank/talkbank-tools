//! Re-exports `batchalign_transform::morphosyntax`. New consumers should
//! import from `batchalign_transform` directly.

pub mod l2;

#[cfg(test)]
mod tests;

pub use batchalign_transform::morphosyntax::*;
