use std::sync::{Mutex, MutexGuard, PoisonError};

use qd_cache::UiStateStore;

/// Locks the UI state store for one read or write.
///
/// A poisoned lock is used as is: every store write is a single SQL
/// statement, so a panic while holding the lock leaves the rows consistent.
pub fn lock_store(store: &Mutex<UiStateStore>) -> MutexGuard<'_, UiStateStore> {
    store.lock().unwrap_or_else(PoisonError::into_inner)
}
