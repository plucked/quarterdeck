use std::sync::{Arc, Mutex};
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
#[derive(Clone)]
struct PendingWrite {
    store: SharedUiStateStore,
    pending: Arc<Mutex<Option<PendingUiState>>>,
}

/// The gpui global through which views read the UI state saved before the
/// last relaunch and record changes to it.
///
/// Writes are batched: the first change starts a short timer, later changes
/// overwrite the pending values, and one write task saves them in order, so
/// an older value never lands after a newer one. Review edits stay queued until SQLite accepts them,
/// and orderly shutdown drains both pending and in-flight changes.
pub struct UiStateRecorder {
    /// Shared with the write running on the background executor.
    store: SharedUiStateStore,
    pending: PendingUiState,
    /// The task waiting to write or writing; while it runs, changes join it.
    write_task: Option<Task<()>>,
    in_flight: Option<PendingWrite>,
    pub review_save_error: Option<String>,
}

impl Global for UiStateRecorder {}

impl UiStateRecorder {
    /// Opens the UI state tables of the default database.
    pub fn open(path: &std::path::Path) -> Result<Self> {
        Ok(Self {
            store: SharedUiStateStore::new(UiStateStore::open(path)?),
            pending: PendingUiState::default(),
            write_task: None,
            in_flight: None,
            review_save_error: None,
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

    /// Folder coloring saved before launch; none when unsaved or unreadable.
    /// Reads one small local row on the main thread at startup only.
    pub fn saved_folder_heatmap(&self) -> Option<bool> {
        self.store
            .lock()
            .folder_heatmap()
            .unwrap_or_else(|error| report_unreadable("folder heatmap", &error))
    }

    /// Records folder coloring and schedules a write.
    pub fn record_folder_heatmap(cx: &mut App, enabled: bool) {
        Self::record(cx, |pending| pending.folder_heatmap = Some(enabled));
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
        Self::record_viewed_change(cx, ViewedChange::Mark { scope, file });
    }

    /// Records that a file is no longer viewed and schedules a write.
    pub fn record_file_unviewed(cx: &mut App, scope: ReviewScope, filename: String) {
        Self::record_viewed_change(cx, ViewedChange::Unmark { scope, filename });
    }

    /// Makes the edit available to background reads as soon as the checkbox changes.
    fn record_viewed_change(cx: &mut App, change: ViewedChange) {
        cx.update_global::<Self, _>(|recorder, cx| {
            recorder.store.viewed_changes().push(change);
            recorder.schedule_write(WRITE_DELAY, cx);
        });
    }

    /// Records a change to the review notes and schedules a write.
    pub fn record_review_change(cx: &mut App, change: ReviewChange) {
        cx.update_global::<Self, _>(|rec, cx| {
            rec.store.review_writes().push_back(change);
            rec.schedule_write(WRITE_DELAY, cx);
        });
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
        cx.update_global::<Self, _>(|rec, cx| {
            if rec.in_flight.is_some() {
                return;
            }
            rec.write_task.take();
            rec.schedule_write(Duration::ZERO, cx);
        });
    }

    /// Starts the write task unless one is running; a running one picks up
    /// the new changes before it ends.
    fn schedule_write(&mut self, delay: Duration, cx: &mut App) {
        if self.write_task.is_some() || !self.has_pending_changes() {
            return;
        }
        self.write_task = Some(cx.spawn(async move |cx| {
            cx.background_executor().timer(delay).await;
            write_until_nothing_pending(cx).await;
        }));
    }

    /// Whether a batch still has settings, viewed marks or review edits to save.
    fn has_pending_changes(&self) -> bool {
        !self.pending.is_empty()
            || !self.store.viewed_changes().is_empty()
            || !self.store.review_writes().is_empty()
    }

    /// Takes the pending changes for a write, or ends the write task when
    /// there are none. Both happen in one main-thread step, so a change
    /// recorded right after always finds either a running task or none.
    fn take_pending_or_finish(&mut self) -> Option<PendingWrite> {
        if !self.has_pending_changes() {
            self.write_task = None;
            return None;
        }
        let write = PendingWrite {
            store: self.store.clone(),
            pending: Arc::new(Mutex::new(Some(std::mem::take(&mut self.pending)))),
        };
        self.in_flight = Some(write.clone());
        Some(write)
    }
}

impl PendingWrite {
    fn flush(&self) {
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(batch) = pending.take() {
            batch.write_to(&self.store);
        }
    }
}

impl UiStateRecorder {
    pub fn flush_on_quit(&mut self) {
        if let Some(write) = self.in_flight.take() {
            write.flush();
        }
        self.pending.write_to(&self.store);
        if let Err(error) = self.store.flush_reviews() {
            eprintln!("Cannot save review drafts on quit: {error:#}");
        }
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
        let error = cx
            .background_executor()
            .spawn(async move {
                write.flush();
                write.store.flush_reviews().err().map(|e| e.to_string())
            })
            .await;
        let retry = error.is_some();
        if cx
            .update_global::<UiStateRecorder, _>(|rec, cx| {
                rec.in_flight = None;
                if rec.review_save_error != error {
                    rec.review_save_error = error;
                    cx.refresh_windows();
                }
            })
            .is_err()
        {
            return;
        }
        if retry {
            cx.background_executor().timer(Duration::from_secs(2)).await;
        }
    }
}

/// Reports a saved row that no longer reads and restores nothing for it.
fn report_unreadable<T>(part: &str, error: &anyhow::Error) -> Option<T> {
    eprintln!("ui state: ignoring saved {part}: {error:#}");
    None
}

#[cfg(test)]
mod durability_tests {
    use super::super::{read_review_notes, read_viewed_files};
    use super::*;
    use qd_cache::{NoteAnchor, NoteDestination, NoteId, ReviewNote, ReviewSummary};
    fn note(scope: &ReviewScope) -> ReviewNote {
        ReviewNote {
            id: NoteId::generate(),
            scope: scope.clone(),
            destination: NoteDestination::GitHub,
            path: "a.rs".into(),
            anchor: NoteAnchor::Line {
                side: qd_github::LineSide::New,
                line: 3,
                start_line: None,
            },
            head_sha: Some("head".into()),
            base_sha: Some("base".into()),
            excerpt: Some("+original".into()),
            body: "draft".into(),
            created_at: 1,
            updated_at: 1,
        }
    }

    /// A reload sees edits handed to the writer and a later unmark of the same file.
    #[test]
    fn viewed_changes_remain_visible_across_writer_handoff() {
        let directory = tempfile::tempdir().unwrap();
        let mut recorder = UiStateRecorder::open(&directory.path().join("cache")).unwrap();
        let scope = ReviewScope::PullRequest(1);
        let file = qd_github::PullRequestFile {
            filename: "a.rs".into(),
            previous_filename: None,
            status: "added".into(),
            additions: 1,
            deletions: 0,
            patch: Some("@@ -0,0 +1 @@\n+hello".into()),
            sha: None,
        };
        let mark = ViewedFile {
            filename: file.filename.clone(),
            fingerprint: qd_cache::PatchFingerprint::of(&file),
        };

        // Hold the writer between the foreground handoff and its database write.
        recorder.store.viewed_changes().push(ViewedChange::Mark {
            scope: scope.clone(),
            file: mark.clone(),
        });
        let worker = recorder.take_pending_or_finish().unwrap();
        assert_eq!(read_viewed_files(&recorder.store, &scope), [mark]);
        recorder.store.viewed_changes().push(ViewedChange::Unmark {
            scope: scope.clone(),
            filename: file.filename,
        });
        assert_eq!(read_viewed_files(&recorder.store, &scope), []);

        // Orderly shutdown must also keep the later edit if the worker runs again.
        recorder.flush_on_quit();
        worker.flush();
        assert_eq!(recorder.store.lock().viewed_files(&scope).unwrap(), []);
    }

    #[test]
    fn quit_drains_in_flight_and_later_changes_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cache");
        let mut rec = UiStateRecorder::open(&path).unwrap();
        rec.pending.sidebar_width = Some(200.);
        let worker = rec.take_pending_or_finish().unwrap();
        rec.pending.sidebar_width = Some(300.);
        let scope = ReviewScope::PullRequest(1);
        let original = note(&scope);
        rec.store
            .review_writes()
            .push_back(ReviewChange::PutNote(original.clone()));
        rec.flush_on_quit();
        worker.flush();
        let reopened = UiStateRecorder::open(&path).unwrap();
        assert_eq!(reopened.saved_panel_widths().unwrap().sidebar, 300.);
        assert_eq!(read_review_notes(&reopened.store, &scope).0, vec![original]);
    }

    #[test]
    fn failed_draft_write_stays_visible_and_retries_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("cache");
        let rec = UiStateRecorder::open(&db).unwrap();
        let sql = rusqlite::Connection::open(&db).unwrap();
        sql.execute_batch("CREATE TRIGGER fail_draft BEFORE INSERT ON review_notes BEGIN SELECT RAISE(FAIL,'test write failure'); END;").unwrap();
        let scope = ReviewScope::PullRequest(2);
        let original = note(&scope);
        rec.store
            .review_writes()
            .push_back(ReviewChange::PutNote(original.clone()));
        assert!(rec.store.flush_reviews().is_err());
        assert_eq!(
            read_review_notes(&rec.store, &scope).0,
            vec![original.clone()]
        );
        rec.store
            .review_writes()
            .push_back(ReviewChange::DeleteNote(original.id));
        assert_eq!(
            read_review_notes(&rec.store, &scope).0,
            Vec::<ReviewNote>::new()
        );
        sql.execute_batch("DROP TRIGGER fail_draft").unwrap();
        rec.store.flush_reviews().unwrap();
        assert!(rec.store.review_writes().is_empty());
        assert_eq!(
            read_review_notes(&rec.store, &scope).0,
            Vec::<ReviewNote>::new()
        );
    }

    #[test]
    fn newer_summary_and_original_note_context_survive_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let rec = UiStateRecorder::open(&dir.path().join("cache")).unwrap();
        let submitted = ReviewSummary {
            body: "one".into(),
            ..ReviewSummary::default()
        };
        let mut next = submitted.clone();
        next.body = "two".into();
        let scope = ReviewScope::PullRequest(1);
        let note = note(&scope);
        rec.store.review_writes().extend([
            ReviewChange::PutNote(note),
            ReviewChange::PutSummary(1, submitted.clone()),
            ReviewChange::PutSummary(1, next.clone()),
            ReviewChange::DeleteSummary(1, submitted),
        ]);
        for _ in 0..2 {
            let (notes, summary) = read_review_notes(&rec.store, &scope);
            assert_eq!(summary, Some(next.clone()));
            assert!(notes[0].matches_revision(Some("head"), Some("base")));
            assert!(!notes[0].matches_revision(Some("push"), Some("base")));
            assert!(!notes[0].matches_revision(Some("head"), Some("rebase")));
            assert_eq!(notes[0].excerpt.as_deref(), Some("+original"));
            rec.store.flush_reviews().unwrap();
        }
    }
}
