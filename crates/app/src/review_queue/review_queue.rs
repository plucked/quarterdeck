use std::time::Instant;

use gpui::{
    App, Context, Entity, FocusHandle, ScrollStrategy, Subscription, Task, UniformListScrollHandle,
    Window, div, prelude::*,
};
use qc_cache::{DetailScrollPosition, PullRequestListKey, ReviewQueueViewState};
use qc_github::PullRequest;

use super::activation_refresh::ActivationRefresh;
use super::refresh_schedule::RefreshSchedule;
use super::refresh_status::RefreshStatus;
use super::restore_phase::RestorePhase;
use super::selection_after_refresh::SelectionAfterRefresh;
use super::selection_restore::SelectionRestore;
use crate::app_commands::{
    MarkViewedAndNext, NextFile, PreviousFile, REVIEW_QUEUE_CONTEXT, Refresh, ToggleFileTree,
};
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::pull_request_detail::{DetailEvent, PullRequestDetail};
use crate::theme;
use crate::ui_state::UiStateRecorder;

const QUEUE_LIST: PullRequestListKey = PullRequestListKey::REVIEW_REQUESTED;

/// A selection saved before the last relaunch, not yet matched to the queue.
struct PendingRestore {
    pull_request_id: u64,
    detail_scroll: DetailScrollPosition,
}

/// The main view: pull requests that request the viewer's review on the
/// left, and the selected one in a [`PullRequestDetail`] on the right.
///
/// The queue is shown from the cache at once and refreshed from GitHub in the
/// background: at launch, on cmd-r, on a timer after each refresh and when the
/// window becomes active with a stale queue.
///
/// The selection and detail scroll are recorded as UI state on every change
/// and restored after a relaunch; a selected pull request that left the
/// queue falls back to no selection.
pub struct ReviewQueue {
    services: AppServices,
    focus: FocusHandle,
    pull_requests: Vec<PullRequest>,
    selected_index: Option<usize>,
    refresh_status: RefreshStatus,
    refresh_schedule: RefreshSchedule,
    queue_list_scroll: UniformListScrollHandle,
    /// The right pane; it re-renders on its own when it scrolls.
    detail: Entity<PullRequestDetail>,
    /// The saved selection, until the queue it is in has loaded.
    pending_restore: Option<PendingRestore>,
    /// The in-flight refresh; replacing it cancels the previous one.
    refresh_task: Option<Task<()>>,
    /// The timer waiting for the next timed refresh; any refresh replaces it.
    auto_refresh_task: Option<Task<()>>,
    /// Keep the window activation and detail scroll observers registered.
    _subscriptions: [Subscription; 2],
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
                    detail_scroll: saved.detail_scroll,
                });

        // The detail pane reports its scrolling so the position is saved.
        let detail = cx.new(|cx| PullRequestDetail::new(services.clone(), cx));
        let detail_subscription =
            cx.subscribe(&detail, |queue, _, event: &DetailEvent, cx| match event {
                DetailEvent::Scrolled => queue.record_view_state(cx),
            });

        let mut queue = Self {
            services,
            focus,
            pull_requests,
            selected_index: None,
            refresh_status,
            refresh_schedule: RefreshSchedule::default(),
            queue_list_scroll: UniformListScrollHandle::new(),
            detail,
            pending_restore,
            refresh_task: None,
            auto_refresh_task: None,
            _subscriptions: [activation_subscription, detail_subscription],
        };
        queue.restore_selection(RestorePhase::CachedQueue, cx);
        queue.refresh(cx);
        queue
    }

    pub(super) fn queue_list_scroll(&self) -> &UniformListScrollHandle {
        &self.queue_list_scroll
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

    /// Selects the pull request at `index` (or none) and shows it in the
    /// detail pane, at `restore` once its files load when given.
    fn set_selected_index(
        &mut self,
        index: Option<usize>,
        restore: Option<DetailScrollPosition>,
        cx: &mut Context<Self>,
    ) {
        self.selected_index = index;
        let pull_request = self.selected_pull_request().cloned();
        self.detail
            .update(cx, |detail, cx| detail.show(pull_request, restore, cx));
    }

    pub(super) fn refresh_status(&self) -> &RefreshStatus {
        &self.refresh_status
    }

    /// Fetches the queue from GitHub, caches it and shows it, removes cached
    /// data unused for longer than the cache TTL, then arms the next timed refresh.
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
        let ttl = AppSettings::cache_ttl(cx).duration();

        self.refresh_status = RefreshStatus::Refreshing;
        cx.notify();
        self.refresh_task = Some(cx.spawn(async move |queue, cx| {
            // Fetch and cache off the main thread: both calls block.
            let fetched = cx
                .background_executor()
                .spawn(async move {
                    let pull_requests = client.review_requested()?;
                    cache.lock().replace_list(QUEUE_LIST, &pull_requests)?;

                    // With the queue current, drop what has not been used within the TTL.
                    cache.lock().delete_expired(ttl)?;
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

        // The detail pane reloads an updated pull request and keeps its scroll.
        match selection {
            SelectionAfterRefresh::Cleared => {
                self.set_selected_index(None, None, cx);
                self.record_view_state(cx);
            }
            SelectionAfterRefresh::Kept(index) | SelectionAfterRefresh::KeptAndUpdated(index) => {
                self.set_selected_index(Some(index), None, cx);
            }
        }

        // A selection saved before the relaunch gets its last chance here.
        self.restore_selection(RestorePhase::RefreshedQueue, cx);
        cx.notify();
    }

    /// Selects the pull request at `index` for the user: shows it from the
    /// top and records the new selection.
    pub(super) fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.selected_index == Some(index) {
            return;
        }
        // A choice by the user replaces whatever was being restored.
        self.pending_restore = None;
        self.set_selected_index(Some(index), None, cx);
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
                // The detail scroll waits for the files; the list scrolls to the row now.
                let restore = Some(pending.detail_scroll);
                self.pending_restore = None;
                self.set_selected_index(Some(index), restore, cx);
                self.queue_list_scroll
                    .scroll_to_item(index, ScrollStrategy::Center);
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
        let detail_scroll = self.pending_restore.as_ref().map_or_else(
            || self.detail.read(cx).scroll_position(),
            |pending| pending.detail_scroll,
        );
        UiStateRecorder::record_review_queue_view_state(
            cx,
            ReviewQueueViewState {
                selected_pull_request_id,
                detail_scroll,
            },
        );
    }

    /// Handles the Refresh action (cmd-r, View menu).
    fn on_refresh(&mut self, _: &Refresh, _: &mut Window, cx: &mut Context<Self>) {
        self.refresh(cx);
    }

    /// Handles the `NextFile` action (`]`): scrolls the detail to the next shown file.
    fn on_next_file(&mut self, _: &NextFile, _: &mut Window, cx: &mut Context<Self>) {
        self.detail.update(cx, PullRequestDetail::reveal_next_file);
    }

    /// Handles the `PreviousFile` action (`[`): scrolls the detail to the previous shown file.
    fn on_previous_file(&mut self, _: &PreviousFile, _: &mut Window, cx: &mut Context<Self>) {
        self.detail
            .update(cx, PullRequestDetail::reveal_previous_file);
    }

    /// Handles the `MarkViewedAndNext` action (`v`): marks the file at the
    /// top viewed and moves to the next unviewed one.
    fn on_mark_viewed_and_next(
        &mut self,
        _: &MarkViewedAndNext,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.detail
            .update(cx, PullRequestDetail::mark_current_viewed_and_advance);
    }

    /// Handles the `ToggleFileTree` action (cmd-b): shows or hides the file tree.
    fn on_toggle_file_tree(&mut self, _: &ToggleFileTree, _: &mut Window, cx: &mut Context<Self>) {
        self.detail.update(cx, PullRequestDetail::toggle_file_tree);
    }
}

impl Render for ReviewQueue {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context(REVIEW_QUEUE_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::on_refresh))
            .on_action(cx.listener(Self::on_next_file))
            .on_action(cx.listener(Self::on_previous_file))
            .on_action(cx.listener(Self::on_mark_viewed_and_next))
            .on_action(cx.listener(Self::on_toggle_file_tree))
            .flex()
            .size_full()
            .bg(theme::background())
            .text_color(theme::text())
            .text_sm()
            .child(self.render_sidebar(cx))
            .child(self.detail.clone())
    }
}
