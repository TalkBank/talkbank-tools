//! # CHAT Specification Generators
//!
//! This crate is the engine behind `make test-gen`: it reads the authoritative
//! CHAT specification files in `spec/constructs/` and `spec/errors/`, parses
//! them into structured Rust types ([`spec`]), and generates downstream
//! artifacts through the [`output`] formatters:
//!
//! - **Tree-sitter corpus tests** -- `*.txt` files written to
//!   `tree-sitter-talkbank/test/corpus/`, consumed by `tree-sitter test`.
//! - **Rust validation tests** -- `#[test]` source files for the parser and
//!   validation crates, ensuring every spec example round-trips correctly.
//! - **Error documentation** -- Markdown pages cataloging all error codes with
//!   examples and fix suggestions.
//!
//! The [`templates`] module handles wrapping sub-document fragments (words,
//! tiers) into complete CHAT files so tree-sitter can parse them. The
//! [`bootstrap`] module provides utilities for seeding new spec files from
//! corpus data.
//!
//! # Running the generators
//!
//! The preferred way to invoke all generators at once is through the root Makefile:
//!
//! ```bash
//! # From the talkbank-tools repository root:
//! make test-gen    # regenerate tree-sitter corpus, Rust tests, and error docs
//! make verify      # pre-merge gates (runs test-gen + all test suites)
//! ```
//!
//! Individual generators can be run directly. Note that `spec/tools/` is a
//! **separate Cargo workspace**, so commands must specify `--manifest-path`
//! when run from the repo root, or you must `cd spec/tools` first:
//!
//! ```bash
//! # Tree-sitter corpus test generation (the most common one)
//! cargo run --manifest-path spec/tools/Cargo.toml \
//!     --bin gen_tree_sitter_tests -- \
//!     -o ../tree-sitter-talkbank/test/corpus \
//!     -t spec/tools/templates
//!
//! # Rust validation test generation
//! cargo run --manifest-path spec/tools/Cargo.toml \
//!     --bin gen_rust_tests -- \
//!     -o crates/talkbank-parser-tests/tests/generated
//!
//! # Rust validation-layer-only test generation
//! cargo run --manifest-path spec/tools/Cargo.toml \
//!     --bin gen_validation_tests -- \
//!     -o crates/talkbank-parser-tests/tests/generated
//!
//! # Error documentation (Markdown)
//! cargo run --manifest-path spec/tools/Cargo.toml \
//!     --bin gen_error_docs -- \
//!     -o docs/errors
//!
//! # Validate error spec layer classifications against actual parser behavior
//! cargo run --manifest-path spec/tools/Cargo.toml \
//!     --bin validate_error_specs
//!
//! # Coverage dashboard (how many constructs have specs)
//! cargo run --manifest-path spec/tools/Cargo.toml \
//!     --bin gen_coverage_dashboard
//! ```
//!
//! # Module map
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`spec`] | Loaders and types for construct/error spec Markdown files |
//! | [`output`] | Formatters that turn parsed specs into generated artifacts |
//! | [`templates`] | Tera template engine for wrapping CHAT fragments into complete files |
//! | [`bootstrap`] | Utilities for seeding new specs from corpus data (grammar analysis, CST extraction) |
//! | [`description`] | Shared description/metadata helpers |
//!
//! ## Binary entry points
//!
//! Each generator is a separate `[[bin]]` target in `Cargo.toml`:
//!
//! | Binary | Purpose |
//! |--------|---------|
//! | `gen_tree_sitter_tests` | Generate `tree-sitter test` corpus files from specs |
//! | `gen_rust_tests` | Generate Rust `#[test]` files from construct + error specs |
//! | `gen_validation_tests` | Generate Rust tests for validation-layer errors only |
//! | `gen_error_docs` | Generate Markdown error documentation pages |
//! | `validate_error_specs` | Validate spec layer classifications against parser behavior |
//! | `validate_spec` | Validate individual spec file format integrity |
//! | `coverage` | Report spec coverage of grammar node types |
//! | `gen_coverage_dashboard` | Generate HTML/Markdown coverage dashboard |
//! | `bootstrap` | Seed new spec files from reference corpus examples |
//! | `bootstrap_tiers` | Seed tier-specific specs from corpus data |
//! | `corpus_to_specs` | Bulk-convert corpus examples to spec format |
//! | `fix_spec_layers` | Auto-fix layer classifications in error specs |
//! | `enhance_specs` | Add missing metadata to existing specs |
//! | `corpus_node_coverage` | Analyze CST node type coverage across the corpus |
//! | `extract_corpus_candidates` | Find corpus examples suitable for new specs |
//! | `perturb_corpus` | Generate perturbed corpus files for fuzz-like testing |
//!
//! # Examples
//!
//! Load all construct specs and inspect their examples:
//!
//! ```no_run
//! use generators::ConstructSpec;
//!
//! let specs = ConstructSpec::load_all("../../spec/constructs")
//!     .expect("failed to load construct specs");
//!
//! for spec in &specs {
//!     println!(
//!         "Category: {} / {} ({} examples)",
//!         spec.metadata.level,
//!         spec.metadata.category,
//!         spec.examples.len(),
//!     );
//!     for ex in &spec.examples {
//!         println!("  - {}: {}", ex.name, ex.description);
//!     }
//! }
//! ```
//!
//! Load all error specs and list their codes:
//!
//! ```no_run
//! use generators::ErrorSpec;
//!
//! let specs = ErrorSpec::load_all("../../spec/errors")
//!     .expect("failed to load error specs");
//!
//! for spec in &specs {
//!     for err in &spec.errors {
//!         println!(
//!             "{} ({}) -- {} [{}, {}]",
//!             err.code,
//!             err.name,
//!             err.severity,
//!             spec.metadata.error_type,
//!             spec.metadata.status,
//!         );
//!     }
//! }
//! ```

pub mod bootstrap;
pub mod description;
pub mod output;
pub mod spec;
pub mod templates;

// Re-exports
pub use spec::{
    construct::{ConstructExample, ConstructMetadata, ConstructSpec},
    error::{ErrorExample, ErrorMetadata, ErrorReference, ErrorSpec},
};
