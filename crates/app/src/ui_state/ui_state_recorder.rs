use std::time::Duration;

use anyhow::Result;
use gpui::{App, AsyncApp, BorrowAppContext, Global, Task};
use qd_cache::{
    CacheTtl, ColorTheme, DiffViewSettings, FoldedGroups, HiddenBranchRule, OthersFilter,
    PanelWidths, RepositoryOthersFilter, ReviewLayoutState, ReviewQueueViewState, ReviewScope,
    UiStateStore, ViewedFile, WindowPlacement,
};
use qd_github::RepoRef;

use super::SharedUiStateStore;
use super::pending_ui_state::{PendingUiState, ViewedChange};
use super::review_change::ReviewChange;

/// Batches a burst of changes (a scroll, a resize drag) into one write.
const WRITE_DELAY: Duration = Duration::from_millis(300);

/// A batch of pending changes taken for one write, with the store to write them to.
struct PendingWrite {
    store: SharedUiStateStore,
    pending: PendingUiState,
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
    store: SharedUiStateStore,
    pending: PendingUiState,
    /// The task waiting to write or writing; while it runs, changes join it.
    write_task: Option<Task<()>>,
}

impl Global for UiStateRecorder {}

impl UiStateRecorder {
    /// Opens the UI state tables of the default database.
    pub fn open_default() -> Result<Self> {
        Ok(Self {
            store: SharedUiStateStore::new(UiStateStore::open_default()?),
            pending: PendingUiState::default(),
            write_task: None,
        })
    }

    /// The window placement saved before the last relaunch; `None` when
    /// there is none or it no longer reads. Reads on the main thread: one
    /// small local row, read at startup only.
    pub fn saved_window_placement(&self) -> Option<WindowPlacement> {
        self.store
            .lock()
            .window_placement()
            .unwrap_or_else(|error| report_unreadable("window placement", &error))
    }

    /// The review queue state saved before the last relaunch; `None` when
    /// there is none or it no longer reads. Reads on the main thread: one
    /// small local row, read at startup only.
    pub fn saved_review_queue_view_state(&self) -> Option<ReviewQueueViewState> {
        self.store
            .lock()
            .review_queue_view_state()
            .unwrap_or_else(|error| report_unreadable("review queue state", &error))
    }

    /// The diff view settings saved before the last relaunch; `None` when
    /// there are none or they no longer read. Reads on the main thread: one
    /// small local row, read at startup only.
    pub fn saved_diff_view_settings(&self) -> Option<DiffViewSettings> {
        self.store
            .lock()
            .diff_view_settings()
            .unwrap_or_else(|error| report_unreadable("diff view settings", &error))
    }

    /// The cache TTL saved before the last relaunch; `None` when there is
    /// none or it no longer reads. Reads on the main thread at startup only.
    pub fn saved_cache_ttl(&self) -> Option<CacheTtl> {
        self.store
            .lock()
            .cache_ttl()
            .unwrap_or_else(|error| report_unreadable("cache TTL", &error))
    }

    /// Records the cache TTL and schedules a write.
    pub fn record_cache_ttl(cx: &mut App, ttl: CacheTtl) {
        Self::record(cx, |pending| pending.cache_ttl = Some(ttl));
    }

    /// The color theme saved before the last relaunch; `None` when there is
    /// none or it no longer reads. Reads on the main thread at startup only.
    pub fn saved_color_theme(&self) -> Option<ColorTheme> {
        self.store
            .lock()
            .color_theme()
            .unwrap_or_else(|error| report_unreadable("color theme", &error))
    }

    /// Records the color theme and schedules a write.
    pub fn record_color_theme(cx: &mut App, theme: ColorTheme) {
        Self::record(cx, |pending| pending.color_theme = Some(theme));
    }

    /// The review layout saved before the last relaunch; `None` when there
    /// is none or it no longer reads. Reads on the main thread at startup only.
    pub fn saved_review_layout(&self) -> Option<ReviewLayoutState> {
        self.store
            .lock()
            .review_layout()
            .unwrap_or_else(|error| report_unreadable("review layout", &error))
    }

    /// Records the review layout and schedules a write.
    pub fn record_review_layout(cx: &mut App, layout: ReviewLayoutState) {
        Self::record(cx, |pending| pending.review_layout = Some(layout));
    }

    /// The panel widths saved before the last relaunch; `None` when there
    /// are none or they no longer read. Reads on the main thread at startup only.
    pub fn saved_panel_widths(&self) -> Option<PanelWidths> {
        self.store
            .lock()
            .panel_widths()
            .unwrap_or_else(|error| report_unreadable("panel widths", &error))
    }

    /// The sidebar groups folded before the last relaunch; none when there
    /// are none or they no longer read. Reads on the main thread at startup only.
    pub fn saved_folded_groups(&self) -> FoldedGroups {
        self.store.lock().folded_groups().unwrap_or_else(|error| {
            report_unreadable::<()>("folded groups", &error);
            FoldedGroups::default()
        })
    }

    /// Records the folded sidebar groups and schedules a write.
    pub fn record_folded_groups(cx: &mut App, groups: FoldedGroups) {
        Self::record(cx, |pending| pending.folded_groups = Some(groups));
    }

    /// The repositories the Others tab lists, as saved; none when there
    /// are none or they no longer read. Reads on the main thread at startup only.
    pub fn saved_watched_repositories(&self) -> Vec<RepoRef> {
        self.store
            .lock()
            .watched_repositories()
            .unwrap_or_else(|error| {
                report_unreadable::<()>("watched repositories", &error);
                Vec::new()
            })
    }

    /// Records the repositories the Others tab lists and schedules a write.
    pub fn record_watched_repositories(cx: &mut App, repositories: Vec<RepoRef>) {
        Self::record(cx, |pending| {
            pending.watched_repositories = Some(repositories);
        });
    }

    /// The rules hiding pull requests by branch, as saved; none when there
    /// are none or they no longer read. Reads on the main thread at startup only.
    pub fn saved_hidden_branch_rules(&self) -> Vec<HiddenBranchRule> {
        self.store
            .lock()
            .hidden_branch_rules()
            .unwrap_or_else(|error| {
                report_unreadable::<()>("hidden branch rules", &error);
                Vec::new()
            })
    }

    /// Records the rules hiding pull requests by branch and schedules a write.
    pub fn record_hidden_branch_rules(cx: &mut App, rules: Vec<HiddenBranchRule>) {
        Self::record(cx, |pending| pending.hidden_branch_rules = Some(rules));
    }

    /// The Others filters saved before the last relaunch; none when there
    /// are none or they no longer read. Reads on the main thread at startup only.
    pub fn saved_others_filters(&self) -> Vec<RepositoryOthersFilter> {
        self.store.lock().others_filters().unwrap_or_else(|error| {
            report_unreadable::<()>("Others filters", &error);
            Vec::new()
        })
    }

    /// Records `repo`'s Others filter and schedules a write.
    pub fn record_others_filter(cx: &mut App, repo: RepoRef, filter: OthersFilter) {
        Self::record(cx, |pending| {
            pending.others_filters.insert(repo, filter);
        });
    }

    /// Records the queue sidebar's width and schedules a write; a drag
    /// records many, and only the last of a burst is written.
    pub fn record_sidebar_width(cx: &mut App, width: f32) {
        Self::record(cx, |pending| pending.sidebar_width = Some(width));
    }

    /// Records the review pane's side panel width and schedules a write.
    pub fn record_side_panel_width(cx: &mut App, width: f32) {
        Self::record(cx, |pending| pending.side_panel_width = Some(width));
    }

    /// The store, for reads on the background executor (viewed marks when a
    /// pull request's files load). Shared because those reads outlive the call.
    pub fn shared_store(&self) -> SharedUiStateStore {
        self.store.clone()
    }

    /// Records the diff view settings and schedules a write.
    pub fn record_diff_view_settings(cx: &mut App, settings: DiffViewSettings) {
        Self::record(cx, |pending| pending.diff_view_settings = Some(settings));
    }

    /// Records that a file is viewed at its current patch and schedules a write.
    pub fn record_file_viewed(cx: &mut App, scope: ReviewScope, file: ViewedFile) {
        Self::record(cx, |pending| {
            pending
                .viewed_changes
                .push(ViewedChange::Mark { scope, file });
        });
    }

    /// Records that a file is no longer viewed and schedules a write.
    pub fn record_file_unviewed(cx: &mut App, scope: ReviewScope, filename: String) {
        Self::record(cx, |pending| {
            pending
                .viewed_changes
                .push(ViewedChange::Unmark { scope, filename });
        });
    }

    /// Records a change to the review notes and schedules a write.
    pub fn record_review_change(cx: &mut App, change: ReviewChange) {
        Self::record(cx, |pending| pending.review_changes.push(change));
    }

    /// Records the window's placement and schedules a write.
    pub fn record_window_placement(cx: &mut App, placement: WindowPlacement) {
        Self::record(cx, |pending| pending.window_placement = Some(placement));
    }

    /// Records the review queue's state and schedules a write.
    pub fn record_review_queue_view_state(cx: &mut App, state: ReviewQueueViewState) {
        Self::record(cx, |pending| pending.review_queue = Some(state));
    }

    /// Applies `change` to the pending changes and schedules a write.
    fn record(cx: &mut App, change: impl FnOnce(&mut PendingUiState)) {
        cx.update_global::<Self, _>(|recorder, cx| {
            change(&mut recorder.pending);
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
    fn take_pending_or_finish(&mut self) -> Option<PendingWrite> {
        if self.pending.is_empty() {
            self.write_task = None;
            return None;
        }
        Some(PendingWrite {
            store: self.store.clone(),
            pending: std::mem::take(&mut self.pending),
        })
    }
}

/// Writes batches of pending changes on the background executor until none
/// are left, then ends the write task.
async fn write_until_nothing_pending(cx: &mut AsyncApp) {
    loop {
        // Take the next batch on the main thread; stop when there is none or the app is gone.
        let next =
            cx.update_global::<UiStateRecorder, _>(|recorder, _| recorder.take_pending_or_finish());
        let Ok(Some(write)) = next else {
            return;
        };

        // Write off the main thread: SQLite may wait for the disk.
        cx.background_executor()
            .spawn(async move { write.pending.write_to(&write.store) })
            .await;
    }
}

/// Reports a saved row that no longer reads and restores nothing for it.
fn report_unreadable<T>(part: &str, error: &anyhow::Error) -> Option<T> {
    eprintln!("ui state: ignoring saved {part}: {error:#}");
    None
}
