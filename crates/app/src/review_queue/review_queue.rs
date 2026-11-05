use std::sync::Arc;
use std::time::Instant;

use gpui::{
    App, Context, FocusHandle, ScrollHandle, ScrollStrategy, Subscription, Task,
    UniformListScrollHandle, Window, div, point, prelude::*, px,
};
use qc_cache::{DiffViewSettings, PullRequestListKey, ReviewQueueViewState, ViewedFile};
use qc_diff::FileDiff;
use qc_github::PullRequest;
use qc_markdown::MarkdownDocument;
use qc_syntax::DiffSyntax;

use super::activation_refresh::ActivationRefresh;
use super::changed_files::ChangedFiles;
use super::file_review_state::FileReviewState;
use super::loaded_file::LoadedFile;
use super::refresh_schedule::RefreshSchedule;
use super::refresh_status::RefreshStatus;
use super::restore_phase::RestorePhase;
use super::selection_after_refresh::SelectionAfterRefresh;
use super::selection_restore::SelectionRestore;
use super::viewed_toggle::ViewedToggle;
use crate::app_commands::Refresh;
use crate::app_services::AppServices;
use crate::theme;
use crate::ui_state::{UiStateRecorder, read_viewed_files};

const QUEUE_LIST: PullRequestListKey = PullRequestListKey::REVIEW_REQUESTED;

/// Batches when the number of cores is unknown.
const DEFAULT_HIGHLIGHT_BATCHES: usize = 4;

/// The syntax colors computed for one diff, matched back by the diff's `Arc`.
struct HighlightedDiff {
    diff: Arc<FileDiff>,
    syntax: Option<Arc<DiffSyntax>>,
}

/// A pull request's changed files as loaded on the background executor,
/// with the viewed marks saved for it.
struct FetchedChanges {
    files: Vec<LoadedFile>,
    viewed_marks: Vec<ViewedFile>,
}

/// A selection saved before the last relaunch, not yet matched to the queue.
struct PendingRestore {
    pull_request_id: u64,
    detail_scroll_offset: f32,
}

/// The parsed description of one version of a pull request. Kept while that
/// version stays selected, so a refresh does not replace the document and
/// reset its text selection.
struct ParsedDescription {
    pull_request_id: u64,
    updated_at: String,
    /// `None` when the description is empty.
    document: Option<Arc<MarkdownDocument>>,
}

impl ParsedDescription {
    fn parse(pull_request: &PullRequest) -> Self {
        Self {
            pull_request_id: pull_request.id,
            updated_at: pull_request.updated_at.clone(),
            document: pull_request
                .body
                .as_deref()
                .map(MarkdownDocument::parse)
                .filter(|document| !document.is_empty())
                .map(Arc::new),
        }
    }

    /// Whether this is the description of `pull_request` as it is now.
    fn is_of(&self, pull_request: &PullRequest) -> bool {
        self.pull_request_id == pull_request.id && self.updated_at == pull_request.updated_at
    }
}

/// The main view: pull requests that request the viewer's review, and the
/// changed files of the selected one.
///
/// The queue is shown from the cache at once and refreshed from GitHub in the
/// background: at launch, on cmd-r, on a timer after each refresh and when the
/// window becomes active with a stale queue. Changed files are read from the
/// cache when still current for the pull request's `updated_at`, otherwise
/// fetched and cached.
///
/// The selection and detail scroll are recorded as UI state on every change and restored after a relaunch; a selected pull request that left
/// the queue falls back to no selection.
pub struct ReviewQueue {
    services: AppServices,
    focus: FocusHandle,
    pull_requests: Vec<PullRequest>,
    selected_index: Option<usize>,
    changed_files: Option<ChangedFiles>,
    refresh_status: RefreshStatus,
    refresh_schedule: RefreshSchedule,
    queue_list_scroll: UniformListScrollHandle,
    detail_scroll: ScrollHandle,
    /// The saved selection, until the queue it is in has loaded.
    pending_restore: Option<PendingRestore>,
    /// The saved detail scroll, applied once the restored files have loaded.
    pending_detail_scroll: Option<f32>,
    /// The selected pull request's description, parsed when the selection
    /// changes to another pull request or version; `None` without a selection.
    description: Option<ParsedDescription>,
    /// The in-flight refresh; replacing it cancels the previous one.
    refresh_task: Option<Task<()>>,
    /// The timer waiting for the next timed refresh; any refresh replaces it.
    auto_refresh_task: Option<Task<()>>,
    /// Keeps the window activation observer registered for the view's lifetime.
    _activation_subscription: Subscription,
    /// The in-flight file load; replacing it cancels the previous one.
    files_task: Option<Task<()>>,
    /// How diffs show; saved across launches.
    diff_settings: DiffViewSettings,
    /// Which loaded files are viewed and collapsed.
    file_review: FileReviewState,
    /// The rebuild of the loaded diffs after a settings change; replacing it cancels the previous one.
    diff_task: Option<Task<()>>,
    /// The syntax highlighting of the loaded diffs; replacing it cancels the previous one.
    syntax_task: Option<Task<()>>,
}

impl ReviewQueue {
    /// Shows the cached queue and starts a refresh from GitHub.
    pub fn new(services: AppServices, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Focus the view so its key bindings (cmd-r) reach it.
        let focus = cx.focus_handle();
        window.focus(&focus);

        // Refresh a stale queue when the user comes back to the window.
        let activation_subscription =
            cx.observe_window_activation(window, Self::on_window_activation_changed);

        // The cache is local and small, so it is read synchronously at startup.
        let cached = services.cache.lock().list(QUEUE_LIST);
        let (pull_requests, refresh_status) = match cached {
            Ok(pull_requests) => (pull_requests, RefreshStatus::Idle),
            Err(error) => (
                Vec::new(),
                RefreshStatus::Failed(format!("cache: {error:#}").into()),
            ),
        };

        // Nothing saved, or a row that no longer reads, restores nothing.
        let diff_settings = cx
            .global::<UiStateRecorder>()
            .saved_diff_view_settings()
            .unwrap_or_default();
        let saved = cx
            .global::<UiStateRecorder>()
            .saved_review_queue_view_state()
            .unwrap_or_default();
        let pending_restore =
            saved
                .selected_pull_request_id
                .map(|pull_request_id| PendingRestore {
                    pull_request_id,
                    detail_scroll_offset: saved.detail_scroll_offset,
                });

        let mut queue = Self {
            services,
            focus,
            pull_requests,
            selected_index: None,
            changed_files: None,
            refresh_status,
            refresh_schedule: RefreshSchedule::default(),
            queue_list_scroll: UniformListScrollHandle::new(),
            detail_scroll: ScrollHandle::new(),
            pending_restore,
            pending_detail_scroll: None,
            description: None,
            refresh_task: None,
            auto_refresh_task: None,
            files_task: None,
            diff_settings,
            file_review: FileReviewState::default(),
            diff_task: None,
            syntax_task: None,
            _activation_subscription: activation_subscription,
        };
        queue.restore_selection(RestorePhase::CachedQueue, cx);
        queue.refresh(cx);
        queue
    }

    pub(super) fn queue_list_scroll(&self) -> &UniformListScrollHandle {
        &self.queue_list_scroll
    }

    pub(super) fn detail_scroll(&self) -> &ScrollHandle {
        &self.detail_scroll
    }

    pub(super) fn pull_requests(&self) -> &[PullRequest] {
        &self.pull_requests
    }

    pub(super) fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    pub(super) fn selected_pull_request(&self) -> Option<&PullRequest> {
        self.selected_index
            .and_then(|index| self.pull_requests.get(index))
    }

    pub(super) fn description(&self) -> Option<&Arc<MarkdownDocument>> {
        self.description
            .as_ref()
            .and_then(|description| description.document.as_ref())
    }

    /// Selects the pull request at `index` (or none) and parses its
    /// description unless the same version is already parsed, so rendering
    /// never parses.
    fn set_selected_index(&mut self, index: Option<usize>) {
        self.selected_index = index;
        let Some(pull_request) = self.selected_pull_request() else {
            self.description = None;
            return;
        };
        let is_current = self
            .description
            .as_ref()
            .is_some_and(|description| description.is_of(pull_request));
        if !is_current {
            self.description = Some(ParsedDescription::parse(pull_request));
        }
    }

    pub(super) fn changed_files(&self) -> Option<&ChangedFiles> {
        self.changed_files.as_ref()
    }

    pub(super) fn diff_settings(&self) -> DiffViewSettings {
        self.diff_settings
    }

    pub(super) fn file_review(&self) -> &FileReviewState {
        &self.file_review
    }

    /// The loaded files, or none while loading or after a failure.
    fn loaded_files(&self) -> &[LoadedFile] {
        match &self.changed_files {
            Some(ChangedFiles::Loaded(files)) => files,
            Some(ChangedFiles::Loading | ChangedFiles::Failed(_)) | None => &[],
        }
    }

    /// Applies new diff settings: saves them and, when they change how
    /// patches are built (whitespace, highlight), rebuilds the loaded diffs
    /// off the main thread. Layout and wrapping only redraw.
    pub(super) fn set_diff_settings(&mut self, settings: DiffViewSettings, cx: &mut Context<Self>) {
        let previous = self.diff_settings;
        self.diff_settings = settings;
        UiStateRecorder::record_diff_view_settings(cx, settings);
        cx.notify();
        if previous.diff_options() == settings.diff_options() {
            return;
        }

        let files = self.loaded_files().to_vec();
        let options = settings.diff_options();
        self.diff_task = Some(cx.spawn(async move |queue, cx| {
            let rebuilt = cx
                .background_executor()
                .spawn(async move {
                    files
                        .iter()
                        .map(|loaded| loaded.rebuilt(options))
                        .collect::<Vec<_>>()
                })
                .await;
            // Only replace files that are still the ones shown.
            queue
                .update(cx, |queue, cx| {
                    let is_same_files = queue.loaded_files().len() == rebuilt.len()
                        && queue
                            .loaded_files()
                            .iter()
                            .zip(&rebuilt)
                            .all(|(shown, built)| shown.fingerprint == built.fingerprint);
                    if is_same_files {
                        queue.changed_files = Some(ChangedFiles::Loaded(rebuilt));
                        queue.highlight_syntax(cx);
                        cx.notify();
                    }
                })
                .ok();
        }));
    }

    /// Collapses or expands the file at `index`.
    pub(super) fn toggle_file_collapsed(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(filename) = self
            .loaded_files()
            .get(index)
            .map(|loaded| loaded.filename().to_owned())
        else {
            return;
        };
        self.file_review.toggle_collapsed(&filename);
        cx.notify();
    }

    /// Marks the file at `index` viewed or not, and saves the mark.
    pub(super) fn toggle_file_viewed(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(pull_request_id) = self
            .selected_pull_request()
            .map(|pull_request| pull_request.id)
        else {
            return;
        };
        let Some(loaded) = self.loaded_files().get(index).cloned() else {
            return;
        };
        match self.file_review.toggle_viewed(&loaded) {
            ViewedToggle::Marked(file) => {
                UiStateRecorder::record_file_viewed(cx, pull_request_id, file);
            }
            ViewedToggle::Unmarked(filename) => {
                UiStateRecorder::record_file_unviewed(cx, pull_request_id, filename);
            }
        }
        cx.notify();
    }

    pub(super) fn collapse_all_files(&mut self, cx: &mut Context<Self>) {
        let files = self.loaded_files().to_vec();
        self.file_review.collapse_all(&files);
        cx.notify();
    }

    pub(super) fn expand_all_files(&mut self, cx: &mut Context<Self>) {
        self.file_review.expand_all();
        cx.notify();
    }

    pub(super) fn refresh_status(&self) -> &RefreshStatus {
        &self.refresh_status
    }

    /// Fetches the queue from GitHub, caches it and shows it, then arms the
    /// next timed refresh.
    pub(super) fn refresh(&mut self, cx: &mut Context<Self>) {
        // Any refresh moves the timed one: it is re-armed when this one finishes.
        self.auto_refresh_task = None;

        // Without a client there is nothing to refresh; say why.
        let client = match &self.services.github {
            Ok(client) => client.clone(),
            Err(reason) => {
                self.refresh_status = RefreshStatus::Failed(reason.clone().into());
                cx.notify();
                return;
            }
        };
        let cache = self.services.cache.clone();

        self.refresh_status = RefreshStatus::Refreshing;
        cx.notify();
        self.refresh_task = Some(cx.spawn(async move |queue, cx| {
            // Fetch and cache off the main thread: both calls block.
            let fetched = cx
                .background_executor()
                .spawn(async move {
                    let pull_requests = client.review_requested()?;
                    cache.lock().replace_list(QUEUE_LIST, &pull_requests)?;
                    anyhow::Ok(pull_requests)
                })
                .await;

            // Show the result and arm the next timed refresh; a closed view simply drops it.
            queue
                .update(cx, |queue, cx| {
                    match fetched {
                        Ok(pull_requests) => queue.replace_pull_requests(pull_requests, cx),
                        Err(error) => {
                            queue.refresh_status =
                                RefreshStatus::Failed(format!("{error:#}").into());
                            cx.notify();
                        }
                    }
                    queue.schedule_next_refresh(cx);
                })
                .ok();
        }));
    }

    /// Records that a refresh finished and starts the timer for the next one.
    /// A failed refresh is retried on the same timer, not sooner.
    fn schedule_next_refresh(&mut self, cx: &mut Context<Self>) {
        let now = Instant::now();
        self.refresh_schedule.record_finished(now);
        let delay = self.refresh_schedule.delay_until_next(now);

        self.auto_refresh_task = Some(cx.spawn(async move |queue, cx| {
            // Sleep until the deadline, then refresh; a closed view simply drops it.
            cx.background_executor().timer(delay).await;
            queue.update(cx, Self::refresh).ok();
        }));
    }

    /// Refreshes a stale queue when the window becomes active.
    fn on_window_activation_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Deactivation needs nothing; only coming back to the window counts.
        if !window.is_window_active() {
            return;
        }
        match self
            .refresh_schedule
            .decide_on_activation(&self.refresh_status, Instant::now())
        {
            ActivationRefresh::Refresh => self.refresh(cx),
            ActivationRefresh::NotDue | ActivationRefresh::AlreadyRefreshing => {}
        }
    }

    /// Replaces the queue after a refresh, keeping the selection where the
    /// pull request is still in it.
    fn replace_pull_requests(&mut self, pull_requests: Vec<PullRequest>, cx: &mut Context<Self>) {
        let selection = SelectionAfterRefresh::decide(self.selected_pull_request(), &pull_requests);
        self.pull_requests = pull_requests;
        self.refresh_status = RefreshStatus::Idle;

        match selection {
            SelectionAfterRefresh::Cleared => {
                self.set_selected_index(None);
                self.changed_files = None;
                self.files_task = None;
                self.record_view_state(cx);
            }
            SelectionAfterRefresh::Kept(index) => self.set_selected_index(Some(index)),
            SelectionAfterRefresh::KeptAndUpdated(index) => {
                self.set_selected_index(Some(index));
                self.load_changed_files(cx);
            }
        }

        // A selection saved before the relaunch gets its last chance here.
        self.restore_selection(RestorePhase::RefreshedQueue, cx);
        cx.notify();
    }

    /// Selects the pull request at `index` for the user: shows its files from
    /// the top and records the new selection.
    pub(super) fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.selected_index == Some(index) {
            return;
        }
        // A choice by the user replaces whatever was being restored.
        self.pending_restore = None;
        self.pending_detail_scroll = None;
        self.detail_scroll.set_offset(point(px(0.), px(0.)));

        self.set_selected_index(Some(index));
        self.load_changed_files(cx);
        self.record_view_state(cx);
    }

    /// Matches the selection saved before the relaunch against the current
    /// queue: selects it with its scroll position, keeps waiting for the
    /// refreshed queue, or drops it.
    fn restore_selection(&mut self, phase: RestorePhase, cx: &mut Context<Self>) {
        let Some(pending) = &self.pending_restore else {
            return;
        };
        match SelectionRestore::decide(pending.pull_request_id, &self.pull_requests, phase) {
            SelectionRestore::Select(index) => {
                // The scroll waits for the files; the list scrolls to the row now.
                self.pending_detail_scroll = Some(pending.detail_scroll_offset);
                self.pending_restore = None;
                self.set_selected_index(Some(index));
                self.queue_list_scroll
                    .scroll_to_item(index, ScrollStrategy::Center);
                self.load_changed_files(cx);
            }
            SelectionRestore::WaitForRefresh => {}
            SelectionRestore::Abandon => {
                self.pending_restore = None;
                self.record_view_state(cx);
            }
        }
    }

    /// Records the selection and detail scroll as UI state. A
    /// selection or scroll still waiting to be restored is recorded as saved,
    /// so a relaunch before it resolves does not lose it.
    pub fn record_view_state(&self, cx: &mut App) {
        let selected_pull_request_id = self
            .selected_pull_request()
            .map(|pull_request| pull_request.id)
            .or(self
                .pending_restore
                .as_ref()
                .map(|pending| pending.pull_request_id));
        let detail_scroll_offset = self
            .pending_detail_scroll
            .or(self
                .pending_restore
                .as_ref()
                .map(|pending| pending.detail_scroll_offset))
            .unwrap_or_else(|| self.detail_scroll.offset().y.into());
        UiStateRecorder::record_review_queue_view_state(
            cx,
            ReviewQueueViewState {
                selected_pull_request_id,
                detail_scroll_offset,
            },
        );
    }

    /// Loads the selected pull request's changed files: from the cache when
    /// current, otherwise from GitHub, caching the result.
    fn load_changed_files(&mut self, cx: &mut Context<Self>) {
        let Some(pull_request) = self.selected_pull_request().cloned() else {
            return;
        };
        let cache = self.services.cache.clone();
        let client = self.services.github.clone();
        let ui_state = cx.global::<UiStateRecorder>().shared_store();
        let options = self.diff_settings.diff_options();

        self.changed_files = Some(ChangedFiles::Loading);
        self.file_review = FileReviewState::default();
        cx.notify();
        self.files_task = Some(cx.spawn(async move |queue, cx| {
            let loaded = cx
                .background_executor()
                .spawn(async move {
                    // A cache hit needs no network; otherwise fetch, then cache for
                    // the next selection. The read ends its lock before the fetch locks again.
                    let cached = cache.lock().files(&pull_request)?;
                    let files = if let Some(files) = cached {
                        files
                    } else {
                        let files = client
                            .map_err(anyhow::Error::msg)?
                            .pull_request_files(&pull_request.repo, pull_request.number)?;
                        cache.lock().put_files(&pull_request, &files)?;
                        files
                    };

                    // Build every diff here too: word comparisons can take a moment on big files.
                    anyhow::Ok(FetchedChanges {
                        files: files
                            .into_iter()
                            .map(|file| LoadedFile::build(file, options))
                            .collect(),
                        viewed_marks: read_viewed_files(&ui_state, pull_request.id),
                    })
                })
                .await;

            // Show the result and apply a restored scroll, which gpui clamps
            // if the files got shorter; a closed view simply drops it.
            queue
                .update(cx, |queue, cx| {
                    queue.changed_files = Some(match loaded {
                        Ok(fetched) => {
                            queue.file_review =
                                FileReviewState::for_loaded(&fetched.files, &fetched.viewed_marks);
                            ChangedFiles::Loaded(fetched.files)
                        }
                        Err(error) => ChangedFiles::Failed(format!("{error:#}").into()),
                    });
                    if let Some(offset) = queue.pending_detail_scroll.take() {
                        queue.detail_scroll.set_offset(point(px(0.), px(offset)));
                    }
                    queue.highlight_syntax(cx);
                    cx.notify();
                })
                .ok();
        }));
    }

    /// Colors the loaded diffs by language off the main thread, one batch of
    /// files per core, showing each batch's colors as it finishes. Files show
    /// plain until then, so loading never waits on it.
    fn highlight_syntax(&mut self, cx: &mut Context<Self>) {
        let files = self.loaded_files().to_vec();
        if files.is_empty() {
            self.syntax_task = None;
            return;
        }

        // Start every batch at once; they run on the executor's threads.
        let cores =
            std::thread::available_parallelism().map_or(DEFAULT_HIGHLIGHT_BATCHES, usize::from);
        let batch_size = files.len().div_ceil(cores);
        let executor = cx.background_executor().clone();
        let batches: Vec<Task<Vec<HighlightedDiff>>> = files
            .chunks(batch_size)
            .map(|batch| {
                let batch = batch.to_vec();
                executor.spawn(async move {
                    batch
                        .into_iter()
                        .map(|loaded| HighlightedDiff {
                            syntax: loaded.highlight_syntax(),
                            diff: loaded.diff,
                        })
                        .collect()
                })
            })
            .collect();

        // Show batches in order; dropping this task cancels the rest.
        self.syntax_task = Some(cx.spawn(async move |queue, cx| {
            for batch in batches {
                let highlighted = batch.await;
                let applied = queue.update(cx, |queue, cx| {
                    queue.apply_syntax(highlighted);
                    cx.notify();
                });
                if applied.is_err() {
                    return;
                }
            }
        }));
    }

    /// Sets the syntax colors of the files whose diffs are still the ones
    /// highlighted; a diff rebuilt meanwhile waits for its own pass.
    fn apply_syntax(&mut self, highlighted: Vec<HighlightedDiff>) {
        let Some(ChangedFiles::Loaded(files)) = &mut self.changed_files else {
            return;
        };
        for result in highlighted {
            if let Some(loaded) = files
                .iter_mut()
                .find(|loaded| Arc::ptr_eq(&loaded.diff, &result.diff))
            {
                loaded.syntax = result.syntax;
            }
        }
    }

    /// Handles the Refresh action (cmd-r, View menu).
    fn on_refresh(&mut self, _: &Refresh, _: &mut Window, cx: &mut Context<Self>) {
        self.refresh(cx);
    }
}

impl Render for ReviewQueue {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::on_refresh))
            .flex()
            .size_full()
            .bg(theme::background())
            .text_color(theme::text())
            .text_sm()
            .child(self.render_sidebar(cx))
            .child(self.render_detail(cx))
    }
}
