use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use qd_cache::UiStateStore;

/// The UI state store, shared by the `UiStateRecorder` global, its batched
/// writes and the reads views run on the background executor, which end
/// independently of each other. Clones share one connection.
#[derive(Clone)]
pub struct SharedUiStateStore(
    Arc<Mutex<UiStateStore>>,
    Arc<Mutex<std::collections::VecDeque<super::ReviewChange>>>,
);

impl SharedUiStateStore {
    pub fn new(store: UiStateStore) -> Self {
        Self(Arc::new(Mutex::new(store)), Arc::default())
    }

    pub(super) fn review_writes(
        &self,
    ) -> MutexGuard<'_, std::collections::VecDeque<super::ReviewChange>> {
        self.1.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn flush_reviews(&self) -> anyhow::Result<()> {
        // The database lock orders concurrent flushes and reads. Holding only
        // that lock for disk I/O leaves the UI free to enqueue the next edit.
        let store = self.lock();
        loop {
            let next = self.review_writes().front().cloned();
            let Some(change) = next else {
                break;
            };
            change.write(&store)?;
            self.review_writes().pop_front();
        }
        Ok(())
    }

    /// Locks the store for one read or write.
    ///
    /// A poisoned lock is used as is: every store write is a single SQL
    /// statement, so a panic while holding the lock leaves the rows consistent.
    pub fn lock(&self) -> MutexGuard<'_, UiStateStore> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
