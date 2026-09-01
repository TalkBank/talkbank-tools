//! Shared runtime filesystem-path overrides.
//!
//! Centralizes environment-variable seams for Batchalign-owned cache paths so
//! the app runtime and the CLI `cache` command agree on where analysis/media
//! artifacts live.

use std::path::PathBuf;

/// Environment variable that overrides the analysis-cache directory.
///
/// When set, the SQLite cache database lives at
/// `$BATCHALIGN_ANALYSIS_CACHE_DIR/cache.db`.
pub const ANALYSIS_CACHE_DIR_ENV: &str = "BATCHALIGN_ANALYSIS_CACHE_DIR";

/// Accepted spelling that follows the `batchalign3` application name.
pub const ANALYSIS_CACHE_DIR_ALIAS_ENV: &str = "BATCHALIGN3_ANALYSIS_CACHE_DIR";

/// Environment variable that overrides the media-conversion cache directory.
pub const MEDIA_CACHE_DIR_ENV: &str = "BATCHALIGN_MEDIA_CACHE_DIR";

fn override_dir_from(source: Option<&str>) -> Option<PathBuf> {
    source
        .map(str::trim)
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
}

/// Parse an explicit analysis-cache directory override from a caller-provided
/// source.
pub fn analysis_cache_dir_override_from(source: Option<&str>) -> Option<PathBuf> {
    override_dir_from(source)
}

/// Resolve the canonical analysis-cache setting and its application-name
/// alias. A nonempty canonical value wins if both are present.
pub fn analysis_cache_dir_override_from_sources(
    canonical: Option<&str>,
    alias: Option<&str>,
) -> Option<PathBuf> {
    analysis_cache_dir_override_from(canonical).or_else(|| analysis_cache_dir_override_from(alias))
}

/// Parse the analysis-cache directory override from the current process
/// environment.
pub fn analysis_cache_dir_override_from_env() -> Option<PathBuf> {
    analysis_cache_dir_override_from_sources(
        std::env::var(ANALYSIS_CACHE_DIR_ENV).ok().as_deref(),
        std::env::var(ANALYSIS_CACHE_DIR_ALIAS_ENV).ok().as_deref(),
    )
}

/// Parse an explicit media-cache directory override from a caller-provided
/// source.
pub fn media_cache_dir_override_from(source: Option<&str>) -> Option<PathBuf> {
    override_dir_from(source)
}

/// Parse the media-cache directory override from the current process
/// environment.
pub fn media_cache_dir_override_from_env() -> Option<PathBuf> {
    media_cache_dir_override_from(std::env::var(MEDIA_CACHE_DIR_ENV).ok().as_deref())
}

#[cfg(test)]
mod tests {
    use super::{
        analysis_cache_dir_override_from, analysis_cache_dir_override_from_sources,
        media_cache_dir_override_from,
    };
    use std::path::PathBuf;

    #[test]
    fn analysis_override_trims_whitespace() {
        assert_eq!(
            analysis_cache_dir_override_from(Some("  /tmp/analysis-cache  ")),
            Some(PathBuf::from("/tmp/analysis-cache"))
        );
    }

    #[test]
    fn analysis_override_ignores_empty_values() {
        assert_eq!(analysis_cache_dir_override_from(Some("   ")), None);
        assert_eq!(analysis_cache_dir_override_from(None), None);
    }

    #[test]
    fn analysis_override_accepts_batchalign3_alias() {
        assert_eq!(
            analysis_cache_dir_override_from_sources(None, Some("/tmp/alias-cache")),
            Some(PathBuf::from("/tmp/alias-cache"))
        );
    }

    #[test]
    fn canonical_analysis_override_precedes_alias() {
        assert_eq!(
            analysis_cache_dir_override_from_sources(
                Some("/tmp/canonical-cache"),
                Some("/tmp/alias-cache"),
            ),
            Some(PathBuf::from("/tmp/canonical-cache"))
        );
    }

    #[test]
    fn media_override_trims_whitespace() {
        assert_eq!(
            media_cache_dir_override_from(Some("  /tmp/media-cache  ")),
            Some(PathBuf::from("/tmp/media-cache"))
        );
    }

    #[test]
    fn media_override_ignores_empty_values() {
        assert_eq!(media_cache_dir_override_from(Some("")), None);
        assert_eq!(media_cache_dir_override_from(None), None);
    }
}
