//! The pinned embedding model's identity, read from the packaged manifest.
//!
//! The manifest is the single owner of which Hub commits the local model graph
//! uses, and it is read here rather than restated: a constant on this side
//! would be a second place the truth lives and would go on agreeing with a
//! model that had moved.

use std::sync::LazyLock;

/// The packaged model graph, the same bytes the Python worker validates and the
/// evidence cache hashes for its revision namespace.
const LOCAL_PYANNOTE_MODEL_MANIFEST: &str =
    include_str!("../../../../../batchalign/inference/local_pyannote_model.json");

/// The parsed answer, computed once per process.
///
/// The manifest is a compile-time constant, so the parse can only ever produce
/// one result; doing it per file would re-parse the same bytes for every
/// transcript in a corpus run.
static PINNED_EMBEDDING_REVISION: LazyLock<Result<String, InvalidModelManifest>> =
    LazyLock::new(read_pinned_embedding_revision);

/// The exact Hub commit of the embedding model this binary pins.
///
/// Returned as a `String` for the evidence's provenance. A caller cannot
/// substitute its own: this is the only producer, and it reads the same file
/// the worker downloads from.
pub fn pinned_embedding_revision() -> Result<String, InvalidModelManifest> {
    PINNED_EMBEDDING_REVISION.clone()
}

fn read_pinned_embedding_revision() -> Result<String, InvalidModelManifest> {
    let manifest: serde_json::Value =
        serde_json::from_str(LOCAL_PYANNOTE_MODEL_MANIFEST).map_err(|error| {
            InvalidModelManifest {
                detail: error.to_string(),
            }
        })?;
    let revision = manifest
        .get("embedding")
        .and_then(|embedding| embedding.get("revision"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| InvalidModelManifest {
            detail: "the packaged model manifest names no embedding revision".to_owned(),
        })?;
    Ok(format!("pyannote-embedding:{revision}"))
}

/// The packaged manifest could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the packaged local model manifest is unusable: {detail}")]
pub struct InvalidModelManifest {
    /// What was wrong with it.
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The revision is a 40-hex Hub commit, read from the packaged manifest.
    ///
    /// Not a golden string: pinning the value here would mean this test had to
    /// be edited every time the pin moved, which trains a contributor to edit
    /// the assertion rather than to ask whether the move was intended. What
    /// matters is that a real commit is found and labelled.
    #[test]
    fn the_pinned_embedding_revision_is_a_hub_commit() {
        let revision = match pinned_embedding_revision() {
            Ok(revision) => revision,
            Err(error) => panic!("the packaged manifest is readable: {error}"),
        };
        let commit = revision
            .strip_prefix("pyannote-embedding:")
            .unwrap_or_default();
        assert_eq!(commit.len(), 40, "got {revision}");
        assert!(commit.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
