use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use qd_cache::UiStateStore;

use super::ReviewChange;
use super::pending_ui_state::ViewedChange;

/// The UI state store, shared by the `UiStateRecorder` global, its batched
/// writes and the reads views run on the background executor, which end
/// independently of each other. Clones share one connection.
#[derive(Clone)]
pub struct SharedUiStateStore {
    database: Arc<Mutex<UiStateStore>>,
    review_writes: Arc<Mutex<VecDeque<ReviewChange>>>,
    viewed_changes: Arc<Mutex<Vec<ViewedChange>>>,
}

impl SharedUiStateStore {
    pub fn new(store: UiStateStore) -> Self {
        Self {
            database: Arc::new(Mutex::new(store)),
            review_writes: Arc::default(),
            viewed_changes: Arc::default(),
        }
    }

    pub(super) fn review_writes(&self) -> MutexGuard<'_, VecDeque<ReviewChange>> {
        self.review_writes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Edits in click order. Readers and flushes acquire the database lock first.
    pub(super) fn viewed_changes(&self) -> MutexGuard<'_, Vec<ViewedChange>> {
        self.viewed_changes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
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
        self.database.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
