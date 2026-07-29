//! The recipe-driven command architecture.
//!
//! No longer a spike, and no longer parallel to `workflow/`, which is gone:
//! this is the load-bearing execution path, reached both by `execution/` and
//! (for now) by the legacy `runner/dispatch/` stack, which calls into it.
//! Retiring that last caller is step 4 of the phase-2 sequence.
//!
//! `catalog.rs` holds the one declaration table for released commands;
//! `recipes.rs` holds the stage recipes it points at.

pub(crate) mod catalog;
pub(crate) mod command_spec;
pub(crate) mod materialize;
pub(crate) mod planner;
pub(crate) mod recipe;
pub(crate) mod recipes;
pub(crate) mod runtime;
pub(crate) mod work_unit;
