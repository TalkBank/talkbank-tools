//! Dependency graph generation for %mor and %gra tiers.
//!
//! Generates DOT (Graphviz) format graphs showing grammatical relations
//! between morphological chunks, similar to the legacy Perl gra-cgi tool.
//!
//! # Layout
//!
//! - **LR (left-to-right)**: Words flow left to right, dependency arcs curve above/below
//! - **Invisible ordering edges**: Maintain word sequence without visual clutter
//! - **Colored dependency edges**: Labeled with relation type (SUBJ, OBJ, DET, etc.)
//! - **ROOT node**: Invisible node at position 0 for root relations
//!
//! # Related CHAT Manual Sections
//!
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Format>
//! - <https://talkbank.org/0info/manuals/CHAT.html#File_Headers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Tier>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>

use talkbank_model::alignment::GraAlignment;
use talkbank_model::model::{GraTier, MorTier, Utterance};

mod builder;
mod edges;
mod labels;

#[cfg(test)]
mod tests;

/// Generate a DOT format dependency graph for an utterance's %mor and %gra tiers.
///
/// # Returns
///
/// DOT format string suitable for rendering with Graphviz, or error message.
///
/// # Format
///
/// The generated graph:
/// - Shows all morphological chunks (words, clitics, terminators) as nodes
/// - Connects chunks with dependency edges from %gra tier
/// - Uses invisible edges to enforce left-to-right word order
/// - Layouts with `rankdir=LR` for left-to-right presentation
///
/// # Example Output
///
/// ```text
/// digraph utterance {
///   rankdir=LR;
///   node [shape=box, style=filled, fillcolor=white, fontname=Arial];
///   edge [fontname=Arial, fontsize=10];
///
///   0 [label="ROOT", shape=point, style=invis];
///   1 [label="the\n1"];
///   2 [label="cat\n2"];
///   3 [label="sat\n3"];
///
///   1 -> 2 [style=invis];
///   2 -> 3 [style=invis];
///
///   1 -> 2 [label="DET"];
///   2 -> 3 [label="SUBJ"];
///   3 -> 0 [label="ROOT"];
/// }
/// ```
pub fn generate_dot_graph(utterance: &Utterance) -> Result<String, String> {
    let mor_tier = require_mor_tier(utterance)?;
    let gra_tier = require_gra_tier(utterance)?;
    let gra_alignment = require_gra_alignment(utterance)?;

    let node_labels = labels::collect_node_labels(mor_tier);
    builder::render_graph(&node_labels, gra_tier, gra_alignment)
}

/// Require a `%mor` tier and return a clear user-facing error when missing.
fn require_mor_tier(utterance: &Utterance) -> Result<&MorTier, String> {
    utterance
        .mor_tier()
        .ok_or_else(|| "No %mor tier found".to_string())
}

/// Require a `%gra` tier and return a clear user-facing error when missing.
fn require_gra_tier(utterance: &Utterance) -> Result<&GraTier, String> {
    utterance
        .gra_tier()
        .ok_or_else(|| "No %gra tier found".to_string())
}

/// Require `%gra` alignment metadata before dependency graph rendering.
fn require_gra_alignment(utterance: &Utterance) -> Result<&GraAlignment, String> {
    let alignment_metadata = utterance
        .alignments
        .as_ref()
        .ok_or_else(|| "No alignment metadata available".to_string())?;

    alignment_metadata
        .gra
        .as_ref()
        .ok_or_else(|| "No %gra alignment computed".to_string())
}
