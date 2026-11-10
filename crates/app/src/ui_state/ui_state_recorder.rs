use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use anyhow::Result;
use gpui::{App, AsyncApp, BorrowAppContext, Global, Task};

use super::review_change::ReviewChange;
use qd_cache::{
    BriefIndex, CacheTtl, ColorTheme, DiffViewSettings, FoldedGroups, HiddenBranchRule,
    NoteDestination, OthersFilter, PanelWidths, RepositoryOthersFilter, ReviewLayoutState,
    ReviewNote, ReviewQueueViewState, ReviewScope, ReviewSummary, StoredBrief, UiStateStore,
    ViewedFile, WindowPlacement,
};
use qd_github::RepoRef;
use qd_studio::{BriefRecord, Studio};

/// Batches a burst of changes (a scroll, a resize drag) into one write.
const WRITE_DELAY: Duration = Duration::from_millis(300);

/// Changes recorded since the last write; only the latest value of each part
/// is kept.
#[derive(Default)]
struct PendingUiState {
    window_placement: Option<WindowPlacement>,
    review_queue: Option<ReviewQueueViewState>,
    diff_view_settings: Option<DiffViewSettings>,
    cache_ttl: Option<CacheTtl>,
    color_theme: Option<ColorTheme>,
    review_layout: Option<ReviewLayoutState>,
    sidebar_width: Option<f32>,
    side_panel_width: Option<f32>,
    folded_groups: Option<FoldedGroups>,
    watched_repositories: Option<Vec<RepoRef>>,
    hidden_branch_rules: Option<Vec<HiddenBranchRule>>,
    /// The latest Others filter of each repository changed.
    others_filters: HashMap<RepoRef, OthersFilter>,
    /// Every viewed mark change, in the order made: unlike settings, each counts.
    viewed_changes: Vec<ViewedChange>,
    /// Every review note and summary change, in the order made.
    review_changes: Vec<ReviewChange>,
}

impl PendingUiState {
    fn is_empty(&self) -> bool {
        self.window_placement.is_none()
            && self.review_queue.is_none()
            && self.diff_view_settings.is_none()
            && self.cache_ttl.is_none()
            && self.color_theme.is_none()
            && self.review_layout.is_none()
            && self.sidebar_width.is_none()
            && self.side_panel_width.is_none()
            && self.folded_groups.is_none()
            && self.watched_repositories.is_none()
            && self.hidden_branch_rules.is_none()
            && self.others_filters.is_empty()
            && self.viewed_changes.is_empty()
            && self.review_changes.is_empty()
    }
}

/// A change to a file's viewed mark, waiting to be written.
enum ViewedChange {
    Mark {
        scope: ReviewScope,
        file: ViewedFile,
    },
    Unmark {
        scope: ReviewScope,
        filename: String,
    },
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

    /// The diff view settings saved before the last relaunch; `None` when
    /// there are none or they no longer read. Reads on the main thread: one
    /// small local row, read at startup only.
    pub fn saved_diff_view_settings(&self) -> Option<DiffViewSettings> {
        self.lock_store()
            .diff_view_settings()
            .unwrap_or_else(|error| report_unreadable("diff view settings", &error))
    }

    /// The cache TTL saved before the last relaunch; `None` when there is
    /// none or it no longer reads. Reads on the main thread at startup only.
    pub fn saved_cache_ttl(&self) -> Option<CacheTtl> {
        self.lock_store()
            .cache_ttl()
            .unwrap_or_else(|error| report_unreadable("cache TTL", &error))
    }

    /// Records the cache TTL and schedules a write.
    pub fn record_cache_ttl(cx: &mut App, ttl: CacheTtl) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.cache_ttl = Some(ttl);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// The color theme saved before the last relaunch; `None` when there is
    /// none or it no longer reads. Reads on the main thread at startup only.
    pub fn saved_color_theme(&self) -> Option<ColorTheme> {
        self.lock_store()
            .color_theme()
            .unwrap_or_else(|error| report_unreadable("color theme", &error))
    }

    /// Records the color theme and schedules a write.
    pub fn record_color_theme(cx: &mut App, theme: ColorTheme) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.color_theme = Some(theme);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// The review layout saved before the last relaunch; `None` when there
    /// is none or it no longer reads. Reads on the main thread at startup only.
    pub fn saved_review_layout(&self) -> Option<ReviewLayoutState> {
        self.lock_store()
            .review_layout()
            .unwrap_or_else(|error| report_unreadable("review layout", &error))
    }

    /// Records the review layout and schedules a write.
    pub fn record_review_layout(cx: &mut App, layout: ReviewLayoutState) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.review_layout = Some(layout);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// The panel widths saved before the last relaunch; `None` when there
    /// are none or they no longer read. Reads on the main thread at startup only.
    pub fn saved_panel_widths(&self) -> Option<PanelWidths> {
        self.lock_store()
            .panel_widths()
            .unwrap_or_else(|error| report_unreadable("panel widths", &error))
    }

    /// The sidebar groups folded before the last relaunch; none when there
    /// are none or they no longer read. Reads on the main thread at startup only.
    pub fn saved_folded_groups(&self) -> FoldedGroups {
        self.lock_store().folded_groups().unwrap_or_else(|error| {
            report_unreadable::<()>("folded groups", &error);
            FoldedGroups::default()
        })
    }

    /// Records the folded sidebar groups and schedules a write.
    pub fn record_folded_groups(cx: &mut App, groups: FoldedGroups) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.folded_groups = Some(groups);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// The repositories the Others tab lists, as saved; none when there
    /// are none or they no longer read. Reads on the main thread at startup only.
    pub fn saved_watched_repositories(&self) -> Vec<RepoRef> {
        self.lock_store()
            .watched_repositories()
            .unwrap_or_else(|error| {
                report_unreadable::<()>("watched repositories", &error);
                Vec::new()
            })
    }

    /// Records the repositories the Others tab lists and schedules a write.
    pub fn record_watched_repositories(cx: &mut App, repositories: Vec<RepoRef>) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.watched_repositories = Some(repositories);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// The rules hiding pull requests by branch, as saved; none when there
    /// are none or they no longer read. Reads on the main thread at startup only.
    pub fn saved_hidden_branch_rules(&self) -> Vec<HiddenBranchRule> {
        self.lock_store()
            .hidden_branch_rules()
            .unwrap_or_else(|error| {
                report_unreadable::<()>("hidden branch rules", &error);
                Vec::new()
            })
    }

    /// Records the rules hiding pull requests by branch and schedules a write.
    pub fn record_hidden_branch_rules(cx: &mut App, rules: Vec<HiddenBranchRule>) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.hidden_branch_rules = Some(rules);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// The Others filters saved before the last relaunch; none when there
    /// are none or they no longer read. Reads on the main thread at startup only.
    pub fn saved_others_filters(&self) -> Vec<RepositoryOthersFilter> {
        self.lock_store().others_filters().unwrap_or_else(|error| {
            report_unreadable::<()>("Others filters", &error);
            Vec::new()
        })
    }

    /// Records `repo`'s Others filter and schedules a write.
    pub fn record_others_filter(cx: &mut App, repo: RepoRef, filter: OthersFilter) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.others_filters.insert(repo, filter);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// Records the queue sidebar's width and schedules a write; a drag
    /// records many, and only the last of a burst is written.
    pub fn record_sidebar_width(cx: &mut App, width: f32) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.sidebar_width = Some(width);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// Records the review pane's side panel width and schedules a write.
    pub fn record_side_panel_width(cx: &mut App, width: f32) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.side_panel_width = Some(width);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// The store, for reads on the background executor (viewed marks when a
    /// pull request's files load). Shared because those reads outlive the call.
    pub fn shared_store(&self) -> Arc<Mutex<UiStateStore>> {
        self.store.clone()
    }

    /// Records the diff view settings and schedules a write.
    pub fn record_diff_view_settings(cx: &mut App, settings: DiffViewSettings) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.diff_view_settings = Some(settings);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// Records that a file is viewed at its current patch and schedules a write.
    pub fn record_file_viewed(cx: &mut App, scope: ReviewScope, file: ViewedFile) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder
                .pending
                .viewed_changes
                .push(ViewedChange::Mark { scope, file });
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// Records that a file is no longer viewed and schedules a write.
    pub fn record_file_unviewed(cx: &mut App, scope: ReviewScope, filename: String) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder
                .pending
                .viewed_changes
                .push(ViewedChange::Unmark { scope, filename });
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// Records a change to the review notes and schedules a write.
    pub fn record_review_change(cx: &mut App, change: ReviewChange) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.pending.review_changes.push(change);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
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
    if let Some(settings) = &pending.diff_view_settings
        && let Err(error) = store.put_diff_view_settings(settings)
    {
        eprintln!("ui state: cannot save diff view settings: {error:#}");
    }
    if let Some(layout) = &pending.review_layout
        && let Err(error) = store.put_review_layout(layout)
    {
        eprintln!("ui state: cannot save the review layout: {error:#}");
    }
    if let Some(width) = pending.sidebar_width
        && let Err(error) = store.put_sidebar_width(width)
    {
        eprintln!("ui state: cannot save the sidebar width: {error:#}");
    }
    if let Some(width) = pending.side_panel_width
        && let Err(error) = store.put_side_panel_width(width)
    {
        eprintln!("ui state: cannot save the side panel width: {error:#}");
    }
    if let Some(groups) = &pending.folded_groups
        && let Err(error) = store.put_folded_groups(groups)
    {
        eprintln!("ui state: cannot save the folded groups: {error:#}");
    }
    if let Some(repositories) = &pending.watched_repositories
        && let Err(error) = store.put_watched_repositories(repositories)
    {
        eprintln!("ui state: cannot save the watched repositories: {error:#}");
    }
    if let Some(rules) = &pending.hidden_branch_rules
        && let Err(error) = store.put_hidden_branch_rules(rules)
    {
        eprintln!("ui state: cannot save the hidden branch rules: {error:#}");
    }
    for (repo, filter) in &pending.others_filters {
        if let Err(error) = store.put_others_filter(repo, filter) {
            eprintln!("ui state: cannot save an Others filter: {error:#}");
        }
    }
    if let Some(ttl) = pending.cache_ttl
        && let Err(error) = store.put_cache_ttl(ttl)
    {
        eprintln!("ui state: cannot save the cache TTL: {error:#}");
    }
    if let Some(theme) = pending.color_theme
        && let Err(error) = store.put_color_theme(theme)
    {
        eprintln!("ui state: cannot save the color theme: {error:#}");
    }
    for change in &pending.viewed_changes {
        let written = match change {
            ViewedChange::Mark { scope, file } => store.mark_file_viewed(scope, file),
            ViewedChange::Unmark { scope, filename } => store.unmark_file_viewed(scope, filename),
        };
        if let Err(error) = written {
            eprintln!("ui state: cannot save a viewed mark: {error:#}");
        }
    }
    for change in &pending.review_changes {
        let written = match change {
            ReviewChange::PutNote(note) => store.put_review_note(note),
            ReviewChange::DeleteNote(id) => store.delete_review_note(*id),
            ReviewChange::PutSummary(pull_request_id, summary) => {
                store.put_review_summary(*pull_request_id, summary)
            }
            ReviewChange::DeleteSummary(pull_request_id) => {
                store.delete_review_summary(*pull_request_id)
            }
        };
        if let Err(error) = written {
            eprintln!("ui state: cannot save a review note: {error:#}");
        }
    }
}

/// The review notes of `scope` (both destinations) and, for a pull request,
/// the GitHub review's summary, read through the shared store on the
/// background executor; unreadable rows count as none.
pub fn read_review_notes(
    store: &Mutex<UiStateStore>,
    scope: &ReviewScope,
) -> (Vec<ReviewNote>, Option<ReviewSummary>) {
    let store = lock_store(store);
    let mut notes = Vec::new();
    for destination in [NoteDestination::Local, NoteDestination::GitHub] {
        match store.review_notes(scope, destination) {
            Ok(read) => notes.extend(read),
            Err(error) => eprintln!("ui state: ignoring review notes: {error:#}"),
        }
    }
    let summary = match scope {
        ReviewScope::PullRequest(pull_request_id) => store
            .review_summary(*pull_request_id)
            .unwrap_or_else(|error| {
                eprintln!("ui state: ignoring the review summary: {error:#}");
                None
            }),
        ReviewScope::CombinedLayers { .. } => None,
    };
    (notes, summary)
}

/// The stored brief of `scope`, read through the shared store on the
/// background executor; an unreadable brief counts as none.
pub fn read_brief(store: &Mutex<UiStateStore>, scope: &ReviewScope) -> Option<StoredBrief> {
    lock_store(store).brief(scope).unwrap_or_else(|error| {
        eprintln!("ui state: ignoring a stored brief: {error:#}");
        None
    })
}

/// Stores the brief `studio` with its `record` as the brief of `scope`,
/// through the shared store on the background executor; a failure is
/// reported on stderr (the brief still shows for the session).
pub fn write_brief(
    store: &Mutex<UiStateStore>,
    scope: &ReviewScope,
    studio: &Studio,
    record: &BriefRecord,
) {
    if let Err(error) = lock_store(store).put_brief(scope, studio, record) {
        eprintln!("ui state: cannot store a brief: {error:#}");
    }
}

/// Which pull requests have a stored brief, read through the shared store
/// on the background executor; unreadable means none.
pub fn read_brief_index(store: &Mutex<UiStateStore>) -> BriefIndex {
    lock_store(store).brief_index().unwrap_or_else(|error| {
        eprintln!("ui state: ignoring the stored briefs: {error:#}");
        BriefIndex::default()
    })
}

/// The viewed marks of a pull request, read through the shared store on the
/// background executor; an unreadable table counts as no marks.
pub fn read_viewed_files(store: &Mutex<UiStateStore>, scope: &ReviewScope) -> Vec<ViewedFile> {
    lock_store(store)
        .viewed_files(scope)
        .unwrap_or_else(|error| {
            eprintln!("ui state: ignoring viewed marks: {error:#}");
            Vec::new()
        })
}

/// Locks the store. A poisoned lock is used as is: every store write is a
/// single SQL statement, so a panic while holding the lock leaves the rows consistent.
fn lock_store(store: &Mutex<UiStateStore>) -> MutexGuard<'_, UiStateStore> {
    store.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Reports a saved row that no longer reads and restores nothing for it.
fn report_unreadable<T>(part: &str, error: &anyhow::Error) -> Option<T> {
    eprintln!("ui state: ignoring saved {part}: {error:#}");
    None
}
