//! Clear cache entries for a specific corpus path prefix.
//!
//! Usage: cargo run --release --bin clear-cache-prefix /path/to/corpus

use std::env;
use talkbank_transform::UnifiedCache;
use talkbank_transform::unified_cache::CacheError;

/// CLI entrypoint that removes cache records whose key begins with the given path prefix.
fn main() -> Result<(), CacheError> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <path-prefix>", args[0]);
        eprintln!(
            "Example: {} /path/to/childes-data/EastAsian/Indonesian/Jakarta",
            args[0]
        );
        return Err(CacheError::Message("Missing path prefix".to_string()));
    }

    let prefix = &args[1];

    println!("Loading cache...");
    let cache = UnifiedCache::new()?;

    let stats = cache.stats()?;
    println!("Cache has {} total entries", stats.total_entries);
    println!("Clearing entries matching prefix: {}", prefix);

    let removed = cache.clear_prefix(prefix)?;
    println!("✓ Cleared {} entries", removed);
    let stats_after = cache.stats()?;
    println!(
        "Cache now has {} entries remaining",
        stats_after.total_entries
    );

    Ok(())
}
