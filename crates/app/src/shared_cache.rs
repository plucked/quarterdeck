use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use qc_cache::GitHubCache;

/// The cache, shared by the views and the background tasks they spawn, which
/// end independently of each other. Clones share one connection.
#[derive(Clone)]
pub struct SharedCache(Arc<Mutex<GitHubCache>>);

impl SharedCache {
    pub fn new(cache: GitHubCache) -> Self {
        Self(Arc::new(Mutex::new(cache)))
    }

    /// Locks the cache for one read or write.
    ///
    /// A poisoned lock is used as is: every cache write is a single statement
    /// or one SQLite transaction, so a panic while holding the lock leaves the
    /// database consistent.
    pub fn lock(&self) -> MutexGuard<'_, GitHubCache> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
