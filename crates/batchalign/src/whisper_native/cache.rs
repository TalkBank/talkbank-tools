//! Process-level cache for the loaded `WhisperContext`.
//!
//! `WhisperContext::new_with_params` reads a 1-3 GB ggml model from disk
//! and runs ~3 s of init on cold load. Production callers run hundreds of
//! transcribe jobs back-to-back; reloading per call would dominate wall
//! time. This module holds a single `Arc<WhisperContext>` per process,
//! lazy-initialized on first request, reused for the rest of the process
//! lifetime. `WhisperContext` is `Send + Sync`, so `Arc` cloning is the
//! natural sharing primitive.
//!
//! ## Single-slot policy
//!
//! The cache holds exactly one (path, context) pair per process. If a
//! second request asks for a different model path, this returns
//! `ModelPathChanged` rather than evicting and reloading; silently
//! reloading would (a) cost another 3 s + 3 GB, (b) leak the old context
//! until refcount drops, and (c) hide a misconfiguration where two
//! components disagree on which model to use. The fleet runs one model
//! per host (set via `BATCHALIGN_WHISPER_RS_MODEL`); a path change at
//! runtime is a bug, not a feature.
//!
//! ## Race window
//!
//! `OnceLock::set` is atomic. If two threads call `get_or_load` before
//! either has initialized, both will run the loader and one will lose the
//! `set` race. The loser drops its `Arc<WhisperContext>` and gets the
//! winner's instead. Cost: one wasted load on cold concurrent start.
//! Avoiding it would require a `Mutex` (which this code deliberately avoids)
//! or `OnceLock::get_or_try_init` (unstable as of Rust 1.83).

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use super::error::WhisperNativeError;

/// Single-slot cache for a heavyweight loaded resource keyed by file
/// path. Generic over the loaded type so the production cache (holding
/// `Arc<WhisperContext>`) and unit tests (holding cheap stand-ins) share
/// the same logic.
///
/// `dead_code` allowance covers the no-feature build, where the
/// production consumer in `backend.rs` is feature-gated out but the
/// unit tests in this file (run with or without the feature) keep the
/// type exercised.
#[allow(dead_code)]
pub(super) struct PathKeyedCache<T> {
    cell: OnceLock<(PathBuf, Arc<T>)>,
}

#[allow(dead_code)]
impl<T> PathKeyedCache<T> {
    pub(super) const fn new() -> Self {
        Self {
            cell: OnceLock::new(),
        }
    }

    /// Return the cached `Arc<T>` for `requested_path`, loading it via
    /// `loader` on first call. Returns `ModelPathChanged` if the cache
    /// is already populated with a different path.
    pub(super) fn get_or_load<L>(
        &self,
        requested_path: &Path,
        loader: L,
    ) -> Result<Arc<T>, WhisperNativeError>
    where
        L: FnOnce(&Path) -> Result<Arc<T>, WhisperNativeError>,
    {
        if let Some((cached_path, value)) = self.cell.get() {
            if cached_path == requested_path {
                return Ok(Arc::clone(value));
            }
            return Err(WhisperNativeError::ModelPathChanged {
                cached: cached_path.clone(),
                requested: requested_path.to_path_buf(),
            });
        }
        let value = loader(requested_path)?;
        match self
            .cell
            .set((requested_path.to_path_buf(), Arc::clone(&value)))
        {
            // We won the race: return the value we just loaded directly.
            Ok(()) => Ok(value),
            // A concurrent caller won `set` first; the cell is now populated.
            // Drop our load and return the winner's `Arc` (functionally
            // equivalent when the paths match).
            Err(_) => match self.cell.get() {
                Some((cached_path, winner)) if cached_path == requested_path => {
                    Ok(Arc::clone(winner))
                }
                // A concurrent caller raced us with a *different* path and
                // won. Unusual but well-defined; surface it as the same error.
                Some((cached_path, _)) => Err(WhisperNativeError::ModelPathChanged {
                    cached: cached_path.clone(),
                    requested: requested_path.to_path_buf(),
                }),
                // Unreachable in practice (the cell is `Some` after any `set`
                // resolves); a typed error rather than a panic because this
                // crate denies `unreachable!`.
                None => Err(WhisperNativeError::CacheInvariant(
                    "OnceLock returned None after a resolved set".to_string(),
                )),
            },
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn first_call_invokes_loader() {
        let cache: PathKeyedCache<u32> = PathKeyedCache::new();
        let calls = AtomicUsize::new(0);
        let v = cache
            .get_or_load(Path::new("/m.bin"), |_| {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(Arc::new(42u32))
            })
            .unwrap();
        assert_eq!(*v, 42);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn second_call_same_path_reuses_arc() {
        let cache: PathKeyedCache<u32> = PathKeyedCache::new();
        let calls = AtomicUsize::new(0);
        let load = |_: &Path| {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(Arc::new(7u32))
        };
        let a = cache.get_or_load(Path::new("/m.bin"), load).unwrap();
        let b = cache.get_or_load(Path::new("/m.bin"), load).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(Arc::ptr_eq(&a, &b));
    }

    #[test]
    fn different_path_returns_model_path_changed() {
        let cache: PathKeyedCache<u32> = PathKeyedCache::new();
        let _ = cache
            .get_or_load(Path::new("/first.bin"), |_| Ok(Arc::new(1u32)))
            .unwrap();
        let err = cache
            .get_or_load(Path::new("/second.bin"), |_| Ok(Arc::new(2u32)))
            .unwrap_err();
        match err {
            WhisperNativeError::ModelPathChanged { cached, requested } => {
                assert_eq!(cached, PathBuf::from("/first.bin"));
                assert_eq!(requested, PathBuf::from("/second.bin"));
            }
            other => panic!("expected ModelPathChanged, got {other:?}"),
        }
    }

    #[test]
    fn loader_error_is_propagated_and_cache_remains_uninitialized() {
        let cache: PathKeyedCache<u32> = PathKeyedCache::new();
        let err = cache
            .get_or_load(Path::new("/m.bin"), |_| {
                Err(WhisperNativeError::ModelPathMissing)
            })
            .unwrap_err();
        assert!(matches!(err, WhisperNativeError::ModelPathMissing));
        // Subsequent successful load still works; the cache wasn't
        // poisoned by the failed first attempt.
        let v = cache
            .get_or_load(Path::new("/m.bin"), |_| Ok(Arc::new(99u32)))
            .unwrap();
        assert_eq!(*v, 99);
    }
}
