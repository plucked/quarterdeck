use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use anyhow::Result;
use gpui::{App, AsyncApp, BorrowAppContext, Global, Task};
use qc_cache::{ReviewQueueViewState, UiStateStore, WindowPlacement};

/// Batches a burst of changes (a scroll, a resize drag) into one write.
const WRITE_DELAY: Duration = Duration::from_millis(300);

/// Changes recorded since the last write; only the latest value of each part
/// is kept.
#[derive(Default)]
struct PendingUiState {
    window_placement: Option<WindowPlacement>,
    review_queue: Option<ReviewQueueViewState>,
}

impl PendingUiState {
    fn is_empty(&self) -> bool {
        self.window_placement.is_none() && self.review_queue.is_none()
    }
}

/// The gpui global through which views read the UI state saved before the
/// last relaunch and record changes to it.
///
/// Writes are batched: the first change starts a short timer, later changes
/// overwrite the pending values, and one write task saves them in order, so
/// an older value never lands after a newer one. Saving is best effort: a
/// failed write is reported on stderr, since losing it only costs the
/// restored position.
pub struct UiStateRecorder {
    /// Shared with the write running on the background executor.
    store: Arc<Mutex<UiStateStore>>,
    pending: PendingUiState,
    /// The task waiting to write or writing; while it runs, changes join it.
    write_task: Option<Task<()>>,
}

impl Global for UiStateRecorder {}

impl UiStateRecorder {
    /// Opens the UI state tables of the default database.
    pub fn open_default() -> Result<Self> {
        Ok(Self {
            store: Arc::new(Mutex::new(UiStateStore::open_default()?)),
            pending: PendingUiState::default(),
            write_task: None,
        })
    }

    /// The window placement saved before the last relaunch; `None` when
    /// there is none or it no longer reads. Reads on the main thread: one
    /// small local row, read at startup only.
    pub fn saved_window_placement(&self) -> Option<WindowPlacement> {
        self.lock_store()
            .window_placement()
            .unwrap_or_else(|error| report_unreadable("window placement", &error))
    }

    /// The review queue state saved before the last relaunch; `None` when
    /// there is none or it no longer reads. Reads on the main thread: one
    /// small local row, read at startup only.
    pub fn saved_review_queue_view_state(&self) -> Option<ReviewQueueViewState> {
        self.lock_store()
            .review_queue_view_state()
            .unwrap_or_else(|error| report_unreadable("review queue state", &error))
    }

    /// Records the window's placement and schedules a write.
    pub fn record_window_placement(cx: &mut App, placement: WindowPlacement) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.window_placement = Some(placement);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// Records the review queue's state and schedules a write.
    pub fn record_review_queue_view_state(cx: &mut App, state: ReviewQueueViewState) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.review_queue = Some(state);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// Writes pending changes without the batching delay. Used when the
    /// window loses focus: switching to the editor comes before the rebuild
    /// that kills and relaunches the app.
    pub fn write_soon(cx: &mut App) {
        cx.update_global::<Self, _>(|recorder, cx| recorder.schedule_write(Duration::ZERO, cx));
    }

    /// Starts the write task unless one is running; a running one picks up
    /// the new changes before it ends.
    fn schedule_write(&mut self, delay: Duration, cx: &mut App) {
        if self.write_task.is_some() || self.pending.is_empty() {
            return;
        }
        self.write_task = Some(cx.spawn(async move |cx| {
            cx.background_executor().timer(delay).await;
            write_until_nothing_pending(cx).await;
        }));
    }

    /// Takes the pending changes for a write, or ends the write task when
    /// there are none. Both happen in one main-thread step, so a change
    /// recorded right after always finds either a running task or none.
    fn take_pending_or_finish(&mut self) -> Option<(Arc<Mutex<UiStateStore>>, PendingUiState)> {
        if self.pending.is_empty() {
            self.write_task = None;
            return None;
        }
        Some((self.store.clone(), std::mem::take(&mut self.pending)))
    }

    /// Locks the store for one read or write.
    ///
    /// A poisoned lock is used as is: every store write is a single SQL
    /// statement, so a panic while holding the lock leaves the rows consistent.
    fn lock_store(&self) -> MutexGuard<'_, UiStateStore> {
        lock_store(&self.store)
    }
}

/// Writes batches of pending changes on the background executor until none
/// are left, then ends the write task.
async fn write_until_nothing_pending(cx: &mut AsyncApp) {
    loop {
        // Take the next batch on the main thread; stop when there is none or the app is gone.
        let next =
            cx.update_global::<UiStateRecorder, _>(|recorder, _| recorder.take_pending_or_finish());
        let Ok(Some((store, pending))) = next else {
            return;
        };

        // Write off the main thread: SQLite may wait for the disk.
        cx.background_executor()
            .spawn(async move { write_pending(&store, &pending) })
            .await;
    }
}

/// Writes each recorded part of `pending`, reporting failures.
fn write_pending(store: &Mutex<UiStateStore>, pending: &PendingUiState) {
    let store = lock_store(store);
    if let Some(placement) = &pending.window_placement
        && let Err(error) = store.put_window_placement(placement)
    {
        eprintln!("ui state: cannot save window placement: {error:#}");
    }
    if let Some(state) = &pending.review_queue
        && let Err(error) = store.put_review_queue_view_state(state)
    {
        eprintln!("ui state: cannot save review queue state: {error:#}");
    }
}

/// Locks the store; see [`UiStateRecorder::lock_store`] for why a poisoned
/// lock is safe to use.
fn lock_store(store: &Mutex<UiStateStore>) -> MutexGuard<'_, UiStateStore> {
    store.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Reports a saved row that no longer reads and restores nothing for it.
fn report_unreadable<T>(part: &str, error: &anyhow::Error) -> Option<T> {
    eprintln!("ui state: ignoring saved {part}: {error:#}");
    None
}
