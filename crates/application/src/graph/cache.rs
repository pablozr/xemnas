//! A small cache of what is read from a project folder, keyed by the folder.
//!
//! The graph asks the same questions of the same folder many times in a row
//! (every refresh, every context pack): its names, its declared members, its
//! file list. Each answer is kept for [`TTL`] and at most [`CAPACITY`]
//! folders are kept; the same pattern as the repository identity cache.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long an answer is reused.
const TTL: Duration = Duration::from_secs(300);
/// Upper bound of kept folders; the cache is dropped when it is exceeded.
const CAPACITY: usize = 32;

/// Answers about project folders, bounded and expiring.
pub(crate) struct FolderCache<V> {
    entries: Mutex<HashMap<PathBuf, (Instant, Arc<V>)>>,
}

impl<V> FolderCache<V> {
    pub(crate) fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// The answer for `root`, worked out by `make` when there is no fresh one.
    /// `make` runs outside the lock, so two callers may both compute it.
    pub(crate) fn get_or_make(&self, root: &Path, make: impl FnOnce() -> V) -> Arc<V> {
        if let Ok(entries) = self.entries.lock() {
            if let Some((at, value)) = entries.get(root) {
                if at.elapsed() < TTL {
                    return Arc::clone(value);
                }
            }
        }
        let value = Arc::new(make());
        if let Ok(mut entries) = self.entries.lock() {
            if entries.len() >= CAPACITY {
                entries.clear();
            }
            entries.insert(root.to_path_buf(), (Instant::now(), Arc::clone(&value)));
        }
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answer_is_reused_and_the_cache_stays_bounded() {
        let cache: FolderCache<usize> = FolderCache::new();
        let mut computed = 0;
        let first = cache.get_or_make(Path::new("a"), || {
            computed += 1;
            7
        });
        let second = cache.get_or_make(Path::new("a"), || {
            computed += 1;
            8
        });
        assert_eq!((*first, *second, computed), (7, 7, 1));
        for index in 0..CAPACITY + 5 {
            cache.get_or_make(Path::new(&format!("folder{index}")), || index);
        }
        let kept = cache.entries.lock().expect("lock").len();
        assert!(kept <= CAPACITY, "{kept}");
    }
}
