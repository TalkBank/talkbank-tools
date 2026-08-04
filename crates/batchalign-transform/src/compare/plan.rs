//! Untrusted TOML plan parsing and plan-relative resolution.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::artifact::{
    AggregatePolicy, ArtifactContractError, ArtifactPair, ComparisonPlan, ComparisonSubject,
    PairingPolicy, ProducedRun, RelativeArtifactPath, RunArtifactRoot, RunManifest,
    ValidatedComparisonPlan,
};

/// Supported comparison-plan schema.
pub const COMPARISON_PLAN_SCHEMA_VERSION: u32 = 1;

/// Untrusted, path-bearing comparison plan as authored in TOML.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonPlanDocument {
    schema_version: u32,
    pairing: PairingPolicy,
    output: PathBuf,
    left: RunReference,
    right: RunReference,
    #[serde(default)]
    speaker_map: Option<BTreeMap<String, String>>,
    pairs: Vec<PairDocument>,
    #[serde(default, alias = "transcription_exclusion_tokens")]
    exclusion_tokens: BTreeSet<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunReference {
    manifest: PathBuf,
    #[serde(alias = "artifact_root")]
    artifacts: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PairDocument {
    left: String,
    right: String,
    #[serde(default)]
    speaker_map: Option<BTreeMap<String, String>>,
    #[serde(default)]
    aggregate: AggregateDocument,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
enum AggregateDocument {
    #[default]
    Included,
    HeldOut {
        reason: String,
    },
}

/// A plan whose external paths have been resolved, but whose bytes are not yet trusted.
#[derive(Debug, Clone)]
pub struct ResolvedComparisonPlan {
    plan: ComparisonPlan,
    pair_options: Vec<(AggregatePolicy, Option<BTreeMap<String, String>>)>,
    exclusion_tokens: BTreeSet<String>,
}

impl ComparisonPlanDocument {
    /// Parse a TOML plan with unknown-field rejection.
    pub fn from_toml(text: &str) -> Result<Self, ArtifactContractError> {
        toml::from_str(text).map_err(|error| {
            ArtifactContractError::InvalidManifest(format!("invalid comparison plan TOML: {error}"))
        })
    }

    /// Read a plan and resolve all relative paths from the plan file's directory.
    pub fn read_and_resolve(
        path: &Path,
        subject: ComparisonSubject,
    ) -> Result<ResolvedComparisonPlan, ArtifactContractError> {
        reject_symlink(path, "plan")?;
        let text = fs::read_to_string(path).map_err(|source| ArtifactContractError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let document = Self::from_toml(&text)?;
        let base = path.parent().unwrap_or_else(|| Path::new("."));
        document.resolve(base, subject)
    }

    /// Resolve this document against an explicit base directory.
    pub fn resolve(
        self,
        base: &Path,
        subject: ComparisonSubject,
    ) -> Result<ResolvedComparisonPlan, ArtifactContractError> {
        if self.schema_version != COMPARISON_PLAN_SCHEMA_VERSION {
            return Err(ArtifactContractError::InvalidManifest(format!(
                "comparison plan schema version {} is not supported (expected {})",
                self.schema_version, COMPARISON_PLAN_SCHEMA_VERSION
            )));
        }
        if self.pairs.is_empty() {
            return Err(ArtifactContractError::NotPairable(
                "comparison plan requires at least one artifact pair".to_string(),
            ));
        }
        validate_map(self.speaker_map.as_ref())?;
        if self.exclusion_tokens.iter().any(|token| token.is_empty()) {
            return Err(ArtifactContractError::InvalidManifest(
                "exclusion tokens must not be empty".to_string(),
            ));
        }
        let left = resolve_run(base, self.left)?;
        let right = resolve_run(base, self.right)?;
        let mut pairs = Vec::with_capacity(self.pairs.len());
        let mut pair_options = Vec::with_capacity(self.pairs.len());
        for pair in self.pairs {
            validate_map(pair.speaker_map.as_ref())?;
            let effective_map = pair.speaker_map.or_else(|| self.speaker_map.clone());
            let aggregate = match pair.aggregate {
                AggregateDocument::Included => AggregatePolicy::Included,
                AggregateDocument::HeldOut { reason } if !reason.trim().is_empty() => {
                    AggregatePolicy::HeldOut { reason }
                }
                AggregateDocument::HeldOut { .. } => {
                    return Err(ArtifactContractError::InvalidManifest(
                        "held-out aggregate policy requires a reason".to_string(),
                    ));
                }
            };
            pairs.push(ArtifactPair::new(
                RelativeArtifactPath::new(pair.left)?,
                RelativeArtifactPath::new(pair.right)?,
            ));
            pair_options.push((aggregate, effective_map));
        }
        let output = resolve_relative(base, &self.output)?;
        Ok(ResolvedComparisonPlan {
            plan: ComparisonPlan {
                subject,
                runs: [left, right],
                pairing: self.pairing,
                artifact_pairs: pairs,
                output,
            },
            pair_options,
            exclusion_tokens: self.exclusion_tokens,
        })
    }
}

impl ResolvedComparisonPlan {
    /// Verify manifests, hashes, pair paths, identities, maps, and pairing policy.
    pub fn validate(self) -> Result<ValidatedComparisonPlan, ArtifactContractError> {
        self.plan
            .validate()?
            .apply_plan_options(self.pair_options, self.exclusion_tokens)
    }
}

fn resolve_run(base: &Path, reference: RunReference) -> Result<ProducedRun, ArtifactContractError> {
    let manifest_path = resolve_relative(base, &reference.manifest)?;
    reject_symlink(&manifest_path, "manifest")?;
    let text = fs::read_to_string(&manifest_path).map_err(|source| ArtifactContractError::Io {
        path: manifest_path.clone(),
        source,
    })?;
    let manifest: RunManifest = serde_json::from_str(&text).map_err(|error| {
        ArtifactContractError::InvalidManifest(format!(
            "cannot parse {}: {error}",
            manifest_path.display()
        ))
    })?;
    let artifacts = resolve_relative(base, &reference.artifacts)?;
    Ok(ProducedRun {
        manifest,
        artifacts: RunArtifactRoot::new(artifacts),
    })
}

fn resolve_relative(base: &Path, path: &Path) -> Result<PathBuf, ArtifactContractError> {
    if path.as_os_str().is_empty() {
        return Err(ArtifactContractError::InvalidArtifactPath(String::new()));
    }
    if path.is_absolute() {
        return Err(ArtifactContractError::InvalidArtifactPath(
            path.display().to_string(),
        ));
    }
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(ArtifactContractError::InvalidArtifactPath(
            path.display().to_string(),
        ));
    }
    Ok(base.join(path))
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), ArtifactContractError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| ArtifactContractError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if metadata.file_type().is_symlink() {
        return Err(ArtifactContractError::InvalidManifest(format!(
            "{label} path must not be a symlink: {}",
            path.display()
        )));
    }
    Ok(())
}

fn validate_map(map: Option<&BTreeMap<String, String>>) -> Result<(), ArtifactContractError> {
    let Some(map) = map else {
        return Ok(());
    };
    if map.is_empty() {
        return Err(ArtifactContractError::InvalidManifest(
            "speaker map must not be empty".to_string(),
        ));
    }
    let mut targets = BTreeSet::new();
    for (left, right) in map {
        if left.trim().is_empty() || right.trim().is_empty() {
            return Err(ArtifactContractError::InvalidManifest(
                "speaker map labels must not be empty".to_string(),
            ));
        }
        if !targets.insert(right) {
            return Err(ArtifactContractError::InvalidManifest(format!(
                "speaker map target {right:?} is assigned more than once"
            )));
        }
    }
    Ok(())
}
