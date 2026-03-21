//! Bootstrap infrastructure for generating spec scaffolding from the grammar.
//!
//! This module provides tools to automatically generate spec files for all
//! nontrivial tree-sitter node types through a multi-stage pipeline:
//!
//! 1. **grammar** -- extract named nodes from `grammar.js`
//! 2. **analyzer** -- classify each node with heuristic priority rules
//! 3. **config_gen** -- emit `all_nodes_annotated.yaml` for human review
//! 4. **classifier** -- load the user-edited `node_config.yaml`
//! 5. **template** / **scaffold** -- apply templates and write spec files
//!
//! Supporting modules: **cst_extractor** (tree-sitter CST with placeholders),
//! **cst_ir** (intermediate representation), **fixture_parser** (extract
//! directives from fixture files), **template_generator** (generate templates
//! from parsed fixtures), **examples** (node-specific example inputs).

pub mod analyzer;
pub mod classifier;
pub mod config_gen;
pub mod cst_extractor;
pub mod cst_ir;
pub mod examples;
pub mod fixture_parser;
pub mod grammar;
pub mod scaffold;
pub mod template;
pub mod template_generator;
