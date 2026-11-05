use std::sync::Arc;
use std::time::Instant;

use gpui::{
    App, Context, FocusHandle, ScrollHandle, ScrollStrategy, Subscription, Task,
    UniformListScrollHandle, Window, div, point, prelude::*, px,
};
use qc_cache::{PullRequestListKey, ReviewQueueViewState};
use qc_github::PullRequest;
use qc_markdown::MarkdownDocument;

use super::activation_refresh::ActivationRefresh;
use super::changed_files::ChangedFiles;
use super::refresh_schedule::RefreshSchedule;
use super::refresh_status::RefreshStatus;
use super::restore_phase::RestorePhase;
use super::selection_after_refresh::SelectionAfterRefresh;
use super::selection_restore::SelectionRestore;
use crate::app_commands::Refresh;
use crate::app_services::AppServices;
use crate::theme;
use crate::ui_state::UiStateRecorder;

const QUEUE_LIST: PullRequestListKey = PullRequestListKey::REVIEW_REQUESTED;

/// A selection saved before the last relaunch, not yet matched to the queue.
struct PendingRestore {
    pull_request_id: u64,
    detail_scroll_offset: f32,
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
    /// or the queue changes; `None` when it has none.
    description: Option<Arc<MarkdownDocument>>,
    /// The in-flight refresh; replacing it cancels the previous one.
    refresh_task: Option<Task<()>>,
    /// The timer waiting for the next timed refresh; any refresh replaces it.
    auto_refresh_task: Option<Task<()>>,
    /// Keeps the window activation observer registered for the view's lifetime.
    _activation_subscription: Subscription,
    /// The in-flight file load; replacing it cancels the previous one.
    files_task: Option<Task<()>>,
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
        self.description.as_ref()
    }

    /// Selects the pull request at `index` (or none) and parses its
    /// description, so rendering never parses.
    fn set_selected_index(&mut self, index: Option<usize>) {
        self.selected_index = index;
        self.description = self
            .selected_pull_request()
            .and_then(|pull_request| pull_request.body.as_deref())
            .map(MarkdownDocument::parse)
            .filter(|document| !document.is_empty())
            .map(Arc::new);
    }

    pub(super) fn changed_files(&self) -> Option<&ChangedFiles> {
        self.changed_files.as_ref()
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

        self.changed_files = Some(ChangedFiles::Loading);
        cx.notify();
        self.files_task = Some(cx.spawn(async move |queue, cx| {
            let loaded = cx
                .background_executor()
                .spawn(async move {
                    // A cache hit needs no network.
                    if let Some(files) = cache.lock().files(&pull_request)? {
                        return anyhow::Ok(files);
                    }

                    // Otherwise fetch, then cache for the next selection.
                    let files = client
                        .map_err(anyhow::Error::msg)?
                        .pull_request_files(&pull_request.repo, pull_request.number)?;
                    cache.lock().put_files(&pull_request, &files)?;
                    Ok(files)
                })
                .await;

            // Show the result and apply a restored scroll, which gpui clamps
            // if the files got shorter; a closed view simply drops it.
            queue
                .update(cx, |queue, cx| {
                    queue.changed_files = Some(match loaded {
                        Ok(files) => ChangedFiles::Loaded(files),
                        Err(error) => ChangedFiles::Failed(format!("{error:#}").into()),
                    });
                    if let Some(offset) = queue.pending_detail_scroll.take() {
                        queue.detail_scroll.set_offset(point(px(0.), px(offset)));
                    }
                    cx.notify();
                })
                .ok();
        }));
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
