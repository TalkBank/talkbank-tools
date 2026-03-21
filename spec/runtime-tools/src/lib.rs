//! Runtime-aware specification tooling.
//!
//! This crate contains the post-generation tooling that needs the live Rust
//! parser/model crates. These tools are intentionally separate from
//! `spec/tools`, which should stay usable without pulling runtime parser/model
//! dependencies into ordinary spec generation workflows.

pub mod bootstrap;
pub mod description;
