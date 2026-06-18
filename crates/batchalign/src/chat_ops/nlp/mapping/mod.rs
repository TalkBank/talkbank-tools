//! UD-to-CHAT morphosyntax mapping. Re-exports
//! `batchalign_transform::morphosyntax`. New consumers should import from
//! `batchalign_transform` directly.

#[cfg(test)]
mod tests;

pub use batchalign_transform::morphosyntax::*;
