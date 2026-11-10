use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use qd_cache::UiStateStore;

/// The UI state store, shared by the `UiStateRecorder` global, its batched
/// writes and the reads views run on the background executor, which end
/// independently of each other. Clones share one connection.
#[derive(Clone)]
pub struct SharedUiStateStore(Arc<Mutex<UiStateStore>>);

impl SharedUiStateStore {
    pub fn new(store: UiStateStore) -> Self {
        Self(Arc::new(Mutex::new(store)))
    }

    /// Locks the store for one read or write.
    ///
    /// A poisoned lock is used as is: every store write is a single SQL
    /// statement, so a panic while holding the lock leaves the rows consistent.
    pub fn lock(&self) -> MutexGuard<'_, UiStateStore> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
