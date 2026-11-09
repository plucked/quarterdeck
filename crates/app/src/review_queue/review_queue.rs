use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use gpui::{
    App, Context, Entity, FocusHandle, ListAlignment, ListOffset, ListState, SharedString,
    Subscription, Task, Window, div, prelude::*, px,
};
use qd_cache::{
    BriefIndex, DetailScrollPosition, PanelVisibility, PullRequestListKey, QueueTab,
    ReviewQueueViewState,
};
use qd_github::{
    GitHubClient, GitHubError, PullRequest, PullRequestStack, RepoRef, StackMembership,
};

use super::activation_refresh::ActivationRefresh;
use super::collapsed_groups::CollapsedGroups;
use super::others_tab::OthersTab;
use super::queue_lists::QueueLists;
use super::queue_row::QueueRow;
use super::refresh_schedule::RefreshSchedule;
use super::refresh_status::RefreshStatus;
use super::restore_phase::RestorePhase;
use super::selection_after_refresh::SelectionAfterRefresh;
use super::selection_origin::SelectionOrigin;
use super::selection_restore::SelectionRestore;
use super::stack_key::StackKey;
use crate::app_commands::{
    MarkViewedAndNext, NextFile, NextLayer, PreviousFile, PreviousLayer, REVIEW_QUEUE_CONTEXT,
    Refresh, ShowAuthored, ShowOthers, ShowReviewRequested, ToggleFileTree, ToggleSidebar,
    ToggleStackReview,
};
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::app_update::AppUpdater;
use crate::avatar_images::AvatarImages;
use crate::fetch_stack::{cached_stack, refresh_stack};
use crate::pull_request_detail::{DetailEvent, LayerRequest, PullRequestDetail};
use crate::shared_cache::SharedCache;
use crate::theme;
use crate::ui_state::{UiStateRecorder, read_brief_index};

/// How far past the sidebar's edges rows are laid out, so scrolling shows no gaps.
const QUEUE_LIST_OVERDRAW: f32 = 300.;

/// A selection saved before the last relaunch, not yet matched to the lists.
struct PendingRestore {
    pull_request_id: u64,
    detail_scroll: DetailScrollPosition,
}

/// Stack memberships by repository and pull request number.
type Memberships = HashMap<(RepoRef, u64), StackMembership>;

/// The viewer's own lists as a refresh fetched them (the first part of a
/// refresh), with their stacks filled in, and the memberships for others'.
struct OwnLists {
    authored: Vec<PullRequest>,
    review_requested: Vec<PullRequest>,
    memberships: Memberships,
    /// Why the stacks could not be read, when they could not (the lists keep the last known ones).
    stack_error: Option<String>,
}

/// Others' pull requests as a refresh fetched them (its second part).
struct FetchedOthers {
    pull_requests: Vec<PullRequest>,
    /// Where each repository's next sync starts.
    synced_until: HashMap<RepoRef, String>,
    /// Why others' pull requests could not be read, when they could not
    /// (the repositories that failed keep the last known ones).
    error: Option<String>,
    /// How long to wait before asking GitHub again (a used-up budget or a
    /// `retry-after`), when known.
    retry_after: Option<Duration>,
}

/// What a refresh starts from besides the client: the watched repositories
/// and what is known already, kept when part of the refresh fails.
struct RefreshInput {
    watched: Vec<RepoRef>,
    known_memberships: Memberships,
    known_others: Vec<PullRequest>,
    /// Where each repository's last sync this session ended.
    others_synced_until: HashMap<RepoRef, String>,
}

/// A stack layer being fetched because it is in no list.
struct OpeningLayer {
    number: u64,
    _task: Task<()>,
}

/// The main view: the viewer's own pull requests and those that request
/// their review on the left (one tab each, stacks grouped), and the
/// selected one in a [`PullRequestDetail`] on the right.
///
/// The lists are shown from the cache at once and refreshed from GitHub in
/// the background: at launch, on cmd-r, on a timer after each refresh and
/// when the window becomes active with a stale queue.
///
/// The tab, selection and detail scroll are recorded as UI state on every
/// change and restored after a relaunch; a selected pull request that left
/// the lists falls back to no selection, unless it was opened from a stack.
pub struct ReviewQueue {
    services: AppServices,
    focus: FocusHandle,
    lists: QueueLists,
    tab: QueueTab,
    /// Whether the sidebar (the lists) shows.
    sidebar_visibility: PanelVisibility,
    /// The sidebar rows of the shown tab.
    rows: Vec<QueueRow>,
    /// The repositories and stacks folded in the sidebar.
    collapsed: CollapsedGroups,
    selected: Option<PullRequest>,
    selection_origin: SelectionOrigin,
    refresh_status: RefreshStatus,
    refresh_schedule: RefreshSchedule,
    /// The sidebar list: rows differ in height (a stack layer is one line).
    queue_list: ListState,
    /// The right pane; it re-renders on its own when it scrolls.
    detail: Entity<PullRequestDetail>,
    /// Which pull requests have a stored brief, marked in the sidebar.
    brief_index: BriefIndex,
    /// The read of `brief_index` in flight.
    brief_index_task: Option<Task<()>>,
    /// The saved selection, until the lists it is in have loaded.
    pending_restore: Option<PendingRestore>,
    /// The in-flight refresh; a new one starts only after it finished.
    refresh_task: Option<Task<()>>,
    /// A refresh was asked for while one ran; it runs once that one finishes.
    is_refresh_requested: bool,
    /// The timer waiting for the next timed refresh; any refresh replaces it.
    auto_refresh_task: Option<Task<()>>,
    /// A stack layer in no list, being fetched to be shown.
    opening_layer: Option<OpeningLayer>,
    /// Why the last stack layer could not be opened.
    layer_error: Option<SharedString>,
    /// The sidebar's width, as dragged (and saved).
    sidebar_width: f32,
    /// The layers of the listed pull requests' stacks, so the sidebar shows
    /// whole stacks; from the cache at launch, checked after each refresh.
    stacks: HashMap<StackKey, PullRequestStack>,
    /// The check of those stacks with GitHub; replacing it cancels the previous one.
    stacks_task: Option<Task<()>>,
    /// The Others tab: watched repositories, their filters, what passes.
    others_tab: OthersTab,
    /// Keep the window activation, detail, updater, settings and avatar observers registered.
    _subscriptions: [Subscription; 5],
}

impl ReviewQueue {
    /// Shows the cached lists and starts a refresh from GitHub.
    pub fn new(services: AppServices, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Focus the view so its key bindings (cmd-r) reach it.
        let focus = cx.focus_handle();
        window.focus(&focus);

        // Refresh a stale queue when the user comes back to the window.
        let activation_subscription =
            cx.observe_window_activation(window, Self::on_window_activation_changed);

        // The cache is local and small, so it is read synchronously at startup.
        let (lists, refresh_status) = match read_cached_lists(&services.cache) {
            Ok(lists) => (lists, RefreshStatus::Idle),
            Err(error) => (
                QueueLists::default(),
                RefreshStatus::Failed(format!("cache: {error:#}").into()),
            ),
        };

        // The cached layers of the listed stacks; local and small, read at startup.
        let stacks = read_cached_stacks(&services.cache, &lists);

        // Nothing saved, or a row that no longer reads, restores nothing.
        let sidebar_width = cx
            .global::<UiStateRecorder>()
            .saved_panel_widths()
            .unwrap_or_default()
            .sidebar;
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

        // The detail pane reports its scrolling (to save it) and the layers it opens.
        let detail = cx.new(|cx| PullRequestDetail::new(services.clone(), cx));
        let detail_subscription =
            cx.subscribe(&detail, |queue, _, event: &DetailEvent, cx| match event {
                DetailEvent::Scrolled => queue.record_view_state(cx),
                DetailEvent::OpenLayer(request) => queue.open_layer(request, cx),
                DetailEvent::PullRequestChanged => {
                    // Keep it in view after the refresh drops it from the lists.
                    queue.selection_origin = SelectionOrigin::Finished;
                    queue.refresh(cx);
                }
                DetailEvent::TargetChanged => cx.notify(),
                DetailEvent::BriefStored => queue.load_brief_index(cx),
            });

        let mut queue = Self {
            services,
            focus,
            lists,
            tab: saved.tab,
            sidebar_visibility: saved.sidebar,
            rows: Vec::new(),
            collapsed: CollapsedGroups::from_saved(
                cx.global::<UiStateRecorder>().saved_folded_groups(),
            ),
            selected: None,
            selection_origin: SelectionOrigin::Listed,
            refresh_status,
            refresh_schedule: RefreshSchedule::default(),
            queue_list: ListState::new(0, ListAlignment::Top, px(QUEUE_LIST_OVERDRAW)),
            detail,
            pending_restore,
            refresh_task: None,
            is_refresh_requested: false,
            auto_refresh_task: None,
            opening_layer: None,
            layer_error: None,
            sidebar_width,
            stacks,
            stacks_task: None,
            brief_index: BriefIndex::default(),
            brief_index_task: None,
            others_tab: OthersTab::new(
                AppSettings::watched_repositories(cx).to_vec(),
                cx.global::<UiStateRecorder>().saved_others_filters(),
            ),
            _subscriptions: [
                activation_subscription,
                detail_subscription,
                // The top bar offers the restart once an update is ready.
                cx.observe(&AppUpdater::global(cx), |_, _, cx| cx.notify()),
                // A change of the watched repositories refetches the Others tab.
                cx.observe_global::<AppSettings>(Self::on_settings_changed),
                // Author chips show avatars as they arrive.
                cx.observe(&AvatarImages::global(cx), |_, _, cx| cx.notify()),
            ],
        };
        queue.update_others_shown();
        queue.request_author_avatars(cx);
        queue.rebuild_rows();
        queue.restore_selection(RestorePhase::CachedQueue, cx);
        queue.refresh(cx);
        queue.load_brief_index(cx);
        queue
    }

    /// Reads which pull requests have a stored brief, on the background
    /// executor, and redraws the sidebar with their marks.
    fn load_brief_index(&mut self, cx: &mut Context<Self>) {
        let store = cx.global::<UiStateRecorder>().shared_store();
        self.brief_index_task = Some(cx.spawn(async move |queue, cx| {
            let index = cx
                .background_executor()
                .spawn(async move { read_brief_index(&store) })
                .await;
            queue
                .update(cx, |queue, cx| {
                    queue.brief_index = index;
                    cx.notify();
                })
                .ok();
        }));
    }

    /// Whether pull request `number` of `repo` has a stored brief.
    pub(super) fn has_brief(&self, repo: &RepoRef, number: u64) -> bool {
        self.brief_index.has_brief(repo, number)
    }

    pub(super) fn queue_list(&self) -> &ListState {
        &self.queue_list
    }

    pub(super) fn tab(&self) -> QueueTab {
        self.tab
    }

    pub(super) fn sidebar_visibility(&self) -> PanelVisibility {
        self.sidebar_visibility
    }

    pub(super) fn detail(&self) -> &Entity<PullRequestDetail> {
        &self.detail
    }

    /// Starts the shown stack's combined review at layer `number` of `repo`
    /// (a shift-click on its row); `false` when another stack is shown.
    pub(super) fn start_combined_at(
        &mut self,
        repo: &RepoRef,
        number: u64,
        cx: &mut Context<Self>,
    ) -> bool {
        self.detail
            .update(cx, |detail, cx| detail.start_combined_at(repo, number, cx))
    }

    /// Shows or hides the sidebar, and saves it.
    pub(super) fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_visibility = self.sidebar_visibility.toggled();
        self.record_view_state(cx);
        cx.notify();
    }

    pub(super) fn lists(&self) -> &QueueLists {
        &self.lists
    }

    /// The pull requests of the shown tab: in the Others tab, those its
    /// filters let through.
    pub(super) fn shown_list(&self) -> &[PullRequest] {
        match self.tab {
            QueueTab::Authored | QueueTab::ReviewRequested => self.lists.get(self.tab),
            QueueTab::Others => self.others_tab.shown(),
        }
    }

    pub(super) fn others_tab(&self) -> &OthersTab {
        &self.others_tab
    }

    pub(super) fn others_tab_mut(&mut self) -> &mut OthersTab {
        &mut self.others_tab
    }

    pub(super) fn lists_mut(&mut self) -> &mut QueueLists {
        &mut self.lists
    }

    /// Whether `repo`'s group is folded in the sidebar.
    pub(super) fn is_repository_collapsed(&self, repo: &RepoRef) -> bool {
        self.collapsed.repositories.contains(repo)
    }

    pub(super) fn rows(&self) -> &[QueueRow] {
        &self.rows
    }

    pub(super) fn selected_pull_request(&self) -> Option<&PullRequest> {
        self.selected.as_ref()
    }

    pub(super) fn refresh_status(&self) -> &RefreshStatus {
        &self.refresh_status
    }

    /// The number of the stack layer being opened, while it is fetched.
    pub(super) fn opening_layer(&self) -> Option<u64> {
        self.opening_layer.as_ref().map(|opening| opening.number)
    }

    pub(super) fn layer_error(&self) -> Option<&SharedString> {
        self.layer_error.as_ref()
    }

    pub(super) fn sidebar_width(&self) -> f32 {
        self.sidebar_width
    }

    /// Sets the sidebar's width while its divider is dragged, and saves it.
    pub(super) fn set_sidebar_width(&mut self, width: f32, cx: &mut Context<Self>) {
        self.sidebar_width = width;
        UiStateRecorder::record_sidebar_width(cx, width);
        cx.notify();
    }

    /// Rebuilds the sidebar rows of the shown tab, keeping the list's
    /// scroll position (a list reset alone would jump to the top).
    pub(super) fn rebuild_rows(&mut self) {
        let scroll_top = self.queue_list.logical_scroll_top();
        self.rows = match self.tab {
            QueueTab::Authored | QueueTab::ReviewRequested => {
                QueueRow::build_all(self.lists.get(self.tab), &self.collapsed, &self.stacks)
            }
            QueueTab::Others => QueueRow::build_others(
                self.others_tab.shown(),
                &self.others_tab.repositories(&self.lists),
                &self.collapsed,
                &self.stacks,
            ),
        };
        self.queue_list.reset(self.rows.len());
        self.queue_list.scroll_to(ListOffset {
            item_ix: scroll_top.item_ix.min(self.rows.len().saturating_sub(1)),
            offset_in_item: scroll_top.offset_in_item,
        });
    }

    /// The known layers of stack `key`.
    pub(super) fn stack(&self, key: &StackKey) -> Option<&PullRequestStack> {
        self.stacks.get(key)
    }

    /// Whether pull request `number` of `repo` asks for the viewer's review.
    pub(super) fn needs_review(&self, repo: &RepoRef, number: u64) -> bool {
        self.lists
            .review_requested
            .iter()
            .any(|pull_request| pull_request.number == number && &pull_request.repo == repo)
    }

    /// Whether the selected pull request is pull request `number` of `repo`.
    pub(super) fn is_selected(&self, repo: &RepoRef, number: u64) -> bool {
        self.selected
            .as_ref()
            .is_some_and(|selected| selected.number == number && &selected.repo == repo)
    }

    /// Shows a layer of stack `key` that the shown list lacks (a click on its row).
    pub(super) fn open_other_layer(
        &mut self,
        key: &StackKey,
        layer: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(stack) = self.stacks.get(key) else {
            return;
        };
        let Some(number) = stack.layers.get(layer).map(|layer| layer.number) else {
            return;
        };
        let request = LayerRequest {
            repo: stack.repo.clone(),
            number,
            membership: StackMembership {
                number: stack.number,
                position: u64::try_from(layer + 1).unwrap_or(u64::MAX),
                size: u64::try_from(stack.layers.len()).unwrap_or(u64::MAX),
                base_ref: stack.base_ref.clone(),
            },
        };
        self.open_layer(&request, cx);
    }

    /// Checks the layers of every listed stack with GitHub (each request
    /// free when unchanged), then shows the stacks whole.
    fn refresh_stacks(&mut self, cx: &mut Context<Self>) {
        let keys = listed_stack_keys(&self.lists);
        let Ok(client) = self.services.github.clone() else {
            return;
        };
        let cache = self.services.cache.clone();
        self.stacks_task = Some(cx.spawn(async move |queue, cx| {
            let refreshed = cx
                .background_executor()
                .spawn(async move {
                    keys.into_iter()
                        .filter_map(|key| {
                            match refresh_stack(Ok(client.clone()), &cache, &key.repo, key.number) {
                                Ok(stack) => Some((key, stack)),
                                Err(error) => {
                                    eprintln!(
                                        "stack {}#{}: {error:#}",
                                        key.repo.full_name(),
                                        key.number
                                    );
                                    None
                                }
                            }
                        })
                        .collect::<HashMap<_, _>>()
                })
                .await;
            queue
                .update(cx, |queue, cx| {
                    // Keep the last known layers of a stack whose check failed.
                    let listed = listed_stack_keys(&queue.lists);
                    queue.stacks.retain(|key, _| listed.contains(key));
                    queue.stacks.extend(refreshed);
                    queue.rebuild_rows();
                    cx.notify();
                })
                .ok();
        }));
    }

    /// Shows the list of `tab` and records it.
    pub(super) fn show_tab(&mut self, tab: QueueTab, cx: &mut Context<Self>) {
        if self.tab == tab {
            return;
        }
        self.tab = tab;
        self.rebuild_rows();
        self.scroll_to_selection();
        self.record_view_state(cx);
        cx.notify();
    }

    /// Collapses or expands a repository's group in the sidebar.
    pub(super) fn toggle_repository_group(&mut self, repo: &RepoRef, cx: &mut Context<Self>) {
        self.collapsed.toggle_repository(repo);
        UiStateRecorder::record_folded_groups(cx, self.collapsed.to_saved());
        self.rebuild_rows();
        cx.notify();
    }

    /// Collapses or expands a stack's group in the sidebar.
    pub(super) fn toggle_stack_group(&mut self, key: &StackKey, cx: &mut Context<Self>) {
        self.collapsed.toggle_stack(key);
        UiStateRecorder::record_folded_groups(cx, self.collapsed.to_saved());
        self.rebuild_rows();
        cx.notify();
    }

    /// Opens the combined review of a stack (a click on its header in the
    /// sidebar): its highest listed layer, combined from the lowest unmerged one.
    pub(super) fn open_stack(&mut self, key: &StackKey, cx: &mut Context<Self>) {
        let Some(top) = self
            .shown_list()
            .iter()
            .filter(|pull_request| StackKey::of(pull_request).as_ref() == Some(key))
            .max_by_key(|pull_request| {
                pull_request
                    .stack
                    .as_ref()
                    .map_or(0, |membership| membership.position)
            })
            .cloned()
        else {
            return;
        };

        // Already shown: only the review changes; else it is set before the new layer shows.
        let is_shown = self.selected.as_ref().map(|selected| selected.id) == Some(top.id);
        if is_shown {
            self.detail
                .update(cx, PullRequestDetail::review_combined_stack);
            cx.notify();
            return;
        }
        self.detail
            .update(cx, PullRequestDetail::prepare_combined_stack_review);
        self.select_pull_request(top, SelectionOrigin::Listed, cx);
    }

    /// Whether the detail shows the combined review of stack `key`, so its
    /// header is highlighted.
    pub(super) fn is_stack_reviewed(&self, key: &StackKey, cx: &App) -> bool {
        self.selected
            .as_ref()
            .and_then(StackKey::of)
            .is_some_and(|selected| &selected == key)
            && self.detail.read(cx).is_combined_review()
    }

    /// Shows `pull_request` (or nothing) in the detail pane, at `restore`
    /// once its files load when given.
    fn set_selected(
        &mut self,
        pull_request: Option<PullRequest>,
        origin: SelectionOrigin,
        restore: Option<DetailScrollPosition>,
        cx: &mut Context<Self>,
    ) {
        self.selected.clone_from(&pull_request);
        self.selection_origin = origin;
        self.detail
            .update(cx, |detail, cx| detail.show(pull_request, restore, cx));
        cx.notify();
    }

    /// Scrolls the sidebar to the selected pull request's row, if the shown tab has it.
    fn scroll_to_selection(&self) {
        let Some(selected_id) = self.selected.as_ref().map(|selected| selected.id) else {
            return;
        };
        let list = self.shown_list();
        let row = self.rows.iter().position(|row| {
            row.pull_request_index().is_some_and(|index| {
                list.get(index)
                    .is_some_and(|pull_request| pull_request.id == selected_id)
            })
        });
        if let Some(row) = row {
            self.queue_list.scroll_to_reveal_item(row);
        }
    }

    /// Fetches the lists and the stacks of their pull requests from GitHub,
    /// caches and shows them (the viewer's own first, then others' in the
    /// watched repositories), removes cached data unused for longer than the
    /// cache TTL, then arms the next timed refresh. One refresh runs at a
    /// time: asked for during one, it runs once that one finishes.
    pub(super) fn refresh(&mut self, cx: &mut Context<Self>) {
        // A refresh in flight finishes first: two would send their requests at once.
        if matches!(self.refresh_status, RefreshStatus::Refreshing) {
            self.is_refresh_requested = true;
            return;
        }

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
        let input = RefreshInput {
            watched: self.others_tab.watched().to_vec(),
            known_memberships: known_memberships(&self.lists),
            known_others: self.lists.others.clone(),
            others_synced_until: self.others_tab.synced_until().clone(),
        };

        self.refresh_status = RefreshStatus::Refreshing;
        cx.notify();
        self.refresh_task = Some(cx.spawn(async move |queue, cx| {
            // The viewer's own lists and every stack first, off the main thread
            // (every call blocks), so they show without waiting for others'.
            let own_client = client.clone();
            let own_cache = cache.clone();
            let watched = input.watched.clone();
            let known_memberships = input.known_memberships;
            let own = cx
                .background_executor()
                .spawn(async move {
                    fetch_own_lists(&own_client, &own_cache, &watched, known_memberships)
                })
                .await;
            let own = match own {
                Ok(own) => own,
                Err(error) => {
                    queue
                        .update(cx, |queue, cx| queue.finish_refresh(Some(&error), None, cx))
                        .ok();
                    return;
                }
            };
            let memberships = own.memberships.clone();
            if queue
                .update(cx, |queue, cx| queue.show_own_lists(own, cx))
                .is_err()
            {
                return;
            }

            // Then others' pull requests, cached; then drop what has not been used within the TTL.
            let others = cx
                .background_executor()
                .spawn(async move {
                    let mut fetched = fetch_others(
                        &client,
                        &input.watched,
                        &input.known_others,
                        &input.others_synced_until,
                    );
                    fill_memberships(&mut fetched.pull_requests, &memberships);
                    let mut cache = cache.lock();
                    cache.replace_list(PullRequestListKey::OTHERS, &fetched.pull_requests)?;
                    cache.delete_expired(ttl)?;
                    anyhow::Ok(fetched)
                })
                .await;

            // Show the result and arm the next refresh; a closed view simply drops it.
            queue
                .update(cx, |queue, cx| match others {
                    Ok(others) => queue.show_others(others, cx),
                    Err(error) => queue.finish_refresh(Some(&error), None, cx),
                })
                .ok();
        }));
    }

    /// Shows the viewer's refreshed own lists, with the stacks just read
    /// filled into the Others list known so far; the refresh goes on.
    fn show_own_lists(&mut self, own: OwnLists, cx: &mut Context<Self>) {
        if let Some(error) = &own.stack_error {
            eprintln!("stacks: keeping the last known ones: {error}");
        }
        let mut others = self.lists.others.clone();
        fill_memberships(&mut others, &own.memberships);
        self.replace_lists(
            QueueLists {
                authored: own.authored,
                review_requested: own.review_requested,
                others,
            },
            cx,
        );
    }

    /// Shows others' refreshed pull requests and ends the refresh.
    fn show_others(&mut self, others: FetchedOthers, cx: &mut Context<Self>) {
        self.others_tab.set_error(others.error);
        self.others_tab.set_synced_until(others.synced_until);
        let lists = QueueLists {
            authored: std::mem::take(&mut self.lists.authored),
            review_requested: std::mem::take(&mut self.lists.review_requested),
            others: others.pull_requests,
        };
        self.replace_lists(lists, cx);
        self.finish_refresh(None, others.retry_after, cx);
    }

    /// Ends a refresh: says why it failed (`failure`), holds the next one
    /// back while GitHub refuses requests (`retry_after`, or the failure's:
    /// until a used-up budget refills or as long as GitHub asked), then
    /// runs a refresh asked for meanwhile or arms the timed one.
    fn finish_refresh(
        &mut self,
        failure: Option<&anyhow::Error>,
        retry_after: Option<Duration>,
        cx: &mut Context<Self>,
    ) {
        // The outcome; a successful one checks the listed stacks with GitHub.
        if let Some(error) = failure {
            self.refresh_status = RefreshStatus::Failed(format!("{error:#}").into());
        } else {
            self.refresh_status = RefreshStatus::Idle;
            self.refresh_stacks(cx);
        }

        // Record when it finished, and how long GitHub refuses requests.
        let now = Instant::now();
        self.refresh_schedule.record_finished(now);
        let retry_after = retry_after.or_else(|| {
            failure
                .and_then(|error| error.downcast_ref::<GitHubError>())
                .and_then(GitHubError::retry_after)
        });
        if let Some(wait) = retry_after {
            self.refresh_schedule.hold_until(now + wait);
        }

        // A refresh asked for meanwhile runs now, unless GitHub refuses requests: then the timer runs it.
        let runs_requested =
            std::mem::take(&mut self.is_refresh_requested) && !self.refresh_schedule.is_held(now);
        if runs_requested {
            self.refresh(cx);
        } else {
            self.schedule_next_refresh(cx);
        }
        cx.notify();
    }

    /// Starts the timer for the next refresh: an interval after the last
    /// one, or once GitHub allows requests again. A failed refresh is retried
    /// on the same timer, not sooner.
    fn schedule_next_refresh(&mut self, cx: &mut Context<Self>) {
        let delay = self.refresh_schedule.delay_until_next(Instant::now());

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

    /// Replaces the lists after a refresh, keeping the selection where the
    /// pull request is still listed (or was opened from a stack).
    fn replace_lists(&mut self, lists: QueueLists, cx: &mut Context<Self>) {
        let refreshed = self
            .selected
            .as_ref()
            .and_then(|selected| lists.find_by_id(selected.id))
            .cloned();
        let selection = SelectionAfterRefresh::decide(
            self.selected.is_some(),
            self.selection_origin,
            refreshed.as_ref(),
        );
        self.lists = lists;
        self.update_others_shown();
        self.request_author_avatars(cx);
        self.rebuild_rows();

        // The detail pane reloads an updated pull request and keeps its scroll.
        match selection {
            SelectionAfterRefresh::Cleared => {
                if self.selected.is_some() {
                    self.set_selected(None, SelectionOrigin::Listed, None, cx);
                    self.record_view_state(cx);
                }
            }
            SelectionAfterRefresh::Unchanged => {}
            SelectionAfterRefresh::Refreshed(pull_request) => {
                let pull_request = pull_request.clone();
                self.set_selected(Some(pull_request), SelectionOrigin::Listed, None, cx);
            }
        }

        // A selection saved before the relaunch gets its last chance here.
        self.restore_selection(RestorePhase::RefreshedQueue, cx);
        cx.notify();
    }

    /// Selects the pull request of the shown list at `index` for the user:
    /// shows it from the top and records the new selection.
    pub(super) fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(pull_request) = self.shown_list().get(index).cloned() else {
            return;
        };
        self.select_pull_request(pull_request, SelectionOrigin::Listed, cx);
    }

    /// Selects `pull_request` for the user, unless it is already selected.
    fn select_pull_request(
        &mut self,
        pull_request: PullRequest,
        origin: SelectionOrigin,
        cx: &mut Context<Self>,
    ) {
        if self.selected.as_ref().map(|selected| selected.id) == Some(pull_request.id) {
            return;
        }
        // A choice by the user replaces whatever was being restored.
        self.pending_restore = None;
        self.opening_layer = None;
        self.layer_error = None;
        self.set_selected(Some(pull_request), origin, None, cx);
        self.scroll_to_selection();
        self.record_view_state(cx);
    }

    /// Shows the stack layer the detail asked for: from a list when listed,
    /// otherwise fetched from GitHub (one request) and cached.
    fn open_layer(&mut self, request: &LayerRequest, cx: &mut Context<Self>) {
        if let Some(listed) = self
            .lists
            .find_by_number(&request.repo, request.number)
            .cloned()
        {
            self.select_pull_request(listed, SelectionOrigin::Listed, cx);
            return;
        }
        let Ok(client) = self.services.github.clone() else {
            self.layer_error = Some("Opening a layer that is in no list needs GitHub.".into());
            cx.notify();
            return;
        };
        let cache = self.services.cache.clone();
        let request = request.clone();
        let number = request.number;
        self.layer_error = None;
        let task = cx.spawn(async move |queue, cx| {
            let fetched = cx
                .background_executor()
                .spawn(async move { fetch_layer(&client, &cache, &request) })
                .await;
            queue
                .update(cx, |queue, cx| {
                    queue.opening_layer = None;
                    match fetched {
                        Ok(pull_request) => {
                            queue.select_pull_request(
                                pull_request,
                                SelectionOrigin::StackLayer,
                                cx,
                            );
                        }
                        Err(error) => {
                            queue.layer_error =
                                Some(format!("Cannot open #{number}: {error:#}").into());
                        }
                    }
                    cx.notify();
                })
                .ok();
        });
        self.opening_layer = Some(OpeningLayer {
            number,
            _task: task,
        });
        cx.notify();
    }

    /// Matches the selection saved before the relaunch: in the lists, or
    /// at launch also among cached pull requests in no list (a stack layer
    /// opened from the map). Selects it with its scroll position, keeps
    /// waiting for the refreshed lists, or drops it.
    fn restore_selection(&mut self, phase: RestorePhase, cx: &mut Context<Self>) {
        let Some(pending) = &self.pending_restore else {
            return;
        };
        let listed = self.lists.find_by_id(pending.pull_request_id).cloned();
        let (found, origin) = match (listed, phase) {
            (Some(listed), _) => (Some(listed), SelectionOrigin::Listed),
            (None, RestorePhase::CachedQueue) => (
                self.services
                    .cache
                    .lock()
                    .pull_request(pending.pull_request_id)
                    .ok()
                    .flatten(),
                SelectionOrigin::StackLayer,
            ),
            (None, RestorePhase::RefreshedQueue) => (None, SelectionOrigin::Listed),
        };
        match SelectionRestore::decide(found.as_ref(), phase) {
            SelectionRestore::Select(pull_request) => {
                // The detail scroll waits for the files; the list scrolls to the row now.
                let pull_request = pull_request.clone();
                let restore = Some(pending.detail_scroll);
                self.pending_restore = None;
                self.set_selected(Some(pull_request), origin, restore, cx);
                self.scroll_to_selection();
            }
            SelectionRestore::WaitForRefresh => {}
            SelectionRestore::Abandon => {
                self.pending_restore = None;
                self.record_view_state(cx);
            }
        }
    }

    /// Records the tab, selection and detail scroll as UI state. A
    /// selection or scroll still waiting to be restored is recorded as saved,
    /// so a relaunch before it resolves does not lose it.
    pub fn record_view_state(&self, cx: &mut App) {
        let selected_pull_request_id = self
            .selected
            .as_ref()
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
                tab: self.tab,
                sidebar: self.sidebar_visibility,
            },
        );
    }

    /// Handles the Refresh action (cmd-r, View menu).
    fn on_refresh(&mut self, _: &Refresh, _: &mut Window, cx: &mut Context<Self>) {
        self.refresh(cx);
    }

    /// Handles the `ShowAuthored` action (cmd-1): shows the viewer's own pull requests.
    fn on_show_authored(&mut self, _: &ShowAuthored, _: &mut Window, cx: &mut Context<Self>) {
        self.show_tab(QueueTab::Authored, cx);
    }

    /// Handles the `ShowReviewRequested` action (cmd-2): shows the review requests.
    fn on_show_review_requested(
        &mut self,
        _: &ShowReviewRequested,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_tab(QueueTab::ReviewRequested, cx);
    }

    /// Handles the `ShowOthers` action (cmd-3): shows others' pull requests.
    fn on_show_others(&mut self, _: &ShowOthers, _: &mut Window, cx: &mut Context<Self>) {
        self.show_tab(QueueTab::Others, cx);
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
    /// Handles the `ToggleSidebar` action (ctrl-cmd-s): shows or hides the lists.
    fn on_toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.toggle_sidebar(cx);
    }

    fn on_toggle_file_tree(&mut self, _: &ToggleFileTree, _: &mut Window, cx: &mut Context<Self>) {
        self.detail.update(cx, PullRequestDetail::toggle_file_tree);
    }

    /// Handles the `NextLayer` action (`}`): shows the stack layer above.
    fn on_next_layer(&mut self, _: &NextLayer, _: &mut Window, cx: &mut Context<Self>) {
        self.detail.update(cx, PullRequestDetail::open_next_layer);
    }

    /// Handles the `PreviousLayer` action (`{`): shows the stack layer below.
    fn on_previous_layer(&mut self, _: &PreviousLayer, _: &mut Window, cx: &mut Context<Self>) {
        self.detail
            .update(cx, PullRequestDetail::open_previous_layer);
    }

    /// Handles the `ToggleStackReview` action (`c`): one layer or combined layers.
    fn on_toggle_stack_review(
        &mut self,
        _: &ToggleStackReview,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.detail
            .update(cx, PullRequestDetail::toggle_review_mode);
    }
}

impl Render for ReviewQueue {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context(REVIEW_QUEUE_CONTEXT)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::on_refresh))
            .on_action(cx.listener(Self::on_show_authored))
            .on_action(cx.listener(Self::on_show_review_requested))
            .on_action(cx.listener(Self::on_show_others))
            .on_action(cx.listener(Self::on_next_file))
            .on_action(cx.listener(Self::on_previous_file))
            .on_action(cx.listener(Self::on_mark_viewed_and_next))
            .on_action(cx.listener(Self::on_toggle_file_tree))
            .on_action(cx.listener(Self::on_toggle_sidebar))
            .on_action(cx.listener(Self::on_next_layer))
            .on_action(cx.listener(Self::on_previous_layer))
            .on_action(cx.listener(Self::on_toggle_stack_review))
            .flex()
            .flex_col()
            .size_full()
            .bg(theme::background())
            .text_color(theme::text())
            .text_sm()
            .child(self.render_top_bar(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .when(self.sidebar_visibility == PanelVisibility::Shown, |panes| {
                        panes
                            .child(self.render_sidebar(cx))
                            .child(self.render_sidebar_handle(cx))
                    })
                    .child(self.detail.clone()),
            )
    }
}

/// The cached lists, read at startup.
fn read_cached_lists(cache: &SharedCache) -> anyhow::Result<QueueLists> {
    let cache = cache.lock();
    Ok(QueueLists {
        authored: cache.list(PullRequestListKey::AUTHORED)?,
        review_requested: cache.list(PullRequestListKey::REVIEW_REQUESTED)?,
        others: cache.list(PullRequestListKey::OTHERS)?,
    })
}

/// The stacks the listed pull requests belong to, each once.
fn listed_stack_keys(lists: &QueueLists) -> HashSet<StackKey> {
    lists.iter().filter_map(StackKey::of).collect()
}

/// The cached layers of the listed stacks; a stack not cached (or not
/// readable) is left out, and its listed layers show alone until checked.
fn read_cached_stacks(
    cache: &SharedCache,
    lists: &QueueLists,
) -> HashMap<StackKey, PullRequestStack> {
    listed_stack_keys(lists)
        .into_iter()
        .filter_map(|key| {
            let stack = cached_stack(cache, &key.repo, key.number).ok()??;
            Some((key, stack))
        })
        .collect()
}

/// The stack memberships the lists know now, kept when GraphQL fails.
fn known_memberships(lists: &QueueLists) -> Memberships {
    lists
        .iter()
        .filter_map(|pull_request| {
            let membership = pull_request.stack.clone()?;
            Some(((pull_request.repo.clone(), pull_request.number), membership))
        })
        .collect()
}

/// Fetches the viewer's own lists (one search request each) and the
/// stacks of every list (one GraphQL request, others' in `watched`
/// included), fills in their memberships and caches the lists. When the
/// stacks cannot be read, the memberships known before are kept.
///
/// Blocking: runs on the background executor.
fn fetch_own_lists(
    client: &GitHubClient,
    cache: &SharedCache,
    watched: &[RepoRef],
    known_memberships: Memberships,
) -> anyhow::Result<OwnLists> {
    // Both lists, then the stacks of all three in one request.
    let mut authored = client.authored()?;
    let mut review_requested = client.review_requested()?;
    let (memberships, stack_error) = match client.stack_memberships(watched) {
        Ok(listed) => (
            listed
                .into_iter()
                .map(|listed| ((listed.repo, listed.number), listed.membership))
                .collect(),
            None,
        ),
        Err(error) => (known_memberships, Some(format!("{error:#}"))),
    };
    fill_memberships(&mut authored, &memberships);
    fill_memberships(&mut review_requested, &memberships);

    // Cache them before showing them.
    let mut cache = cache.lock();
    cache.replace_list(PullRequestListKey::AUTHORED, &authored)?;
    cache.replace_list(PullRequestListKey::REVIEW_REQUESTED, &review_requested)?;
    Ok(OwnLists {
        authored,
        review_requested,
        memberships,
        stack_error,
    })
}

/// Sets each of `pull_requests`' stack membership from `memberships`
/// (none for a standalone one).
fn fill_memberships(pull_requests: &mut [PullRequest], memberships: &Memberships) {
    for pull_request in pull_requests {
        pull_request.stack = memberships
            .get(&(pull_request.repo.clone(), pull_request.number))
            .cloned();
    }
}

/// Others' open pull requests in `watched`, each repository brought up to
/// date from what `known` has of it and where its last sync ended
/// (`synced_until`), most recently updated first. A repository that fails
/// keeps what was known of it, and the first failure is returned to show;
/// once GitHub refuses requests (rate limit, slow down), the rest are not
/// asked and keep what was known too.
///
/// Blocking: runs on the background executor.
fn fetch_others(
    client: &GitHubClient,
    watched: &[RepoRef],
    known: &[PullRequest],
    synced_until: &HashMap<RepoRef, String>,
) -> FetchedOthers {
    let known_of = |repo: &RepoRef| -> Vec<PullRequest> {
        known
            .iter()
            .filter(|pull_request| &pull_request.repo == repo)
            .cloned()
            .collect()
    };
    let mut fetched = FetchedOthers {
        pull_requests: Vec::new(),
        synced_until: HashMap::new(),
        error: None,
        retry_after: None,
    };
    if watched.is_empty() {
        return fetched;
    }

    // Whose pull requests to leave out: the viewer's (asked once per session).
    let viewer_login = match client.viewer_login() {
        Ok(login) => login,
        Err(error) => {
            fetched.pull_requests = known.to_vec();
            fetched.synced_until.clone_from(synced_until);
            fetched.retry_after = error.retry_after();
            fetched.error = Some(format!("{error:#}"));
            return fetched;
        }
    };

    // Each repository on its own, so one failure keeps the others current.
    let mut is_throttled = false;
    for repo in watched {
        let known = known_of(repo);
        let last_synced = synced_until.get(repo);
        let result = (!is_throttled).then(|| {
            client.other_pull_requests(repo, &known, last_synced.map(String::as_str), &viewer_login)
        });
        match result {
            Some(Ok(update)) => {
                fetched.pull_requests.extend(update.pull_requests);
                if let Some(until) = update.synced_until {
                    fetched.synced_until.insert(repo.clone(), until);
                }
            }
            Some(Err(error)) => {
                is_throttled = error.is_throttled();
                fetched.retry_after = fetched.retry_after.or(error.retry_after());
                fetched
                    .error
                    .get_or_insert_with(|| format!("{}: {error:#}", repo.full_name()));
                keep_known(&mut fetched, repo, known, last_synced);
            }
            None => keep_known(&mut fetched, repo, known, last_synced),
        }
    }
    fetched
        .pull_requests
        .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    fetched
}

/// Keeps what was known of `repo` (its pull requests and where its last
/// sync ended) in `fetched`, when it could not be brought up to date.
fn keep_known(
    fetched: &mut FetchedOthers,
    repo: &RepoRef,
    known: Vec<PullRequest>,
    last_synced: Option<&String>,
) {
    fetched.pull_requests.extend(known);
    if let Some(until) = last_synced {
        fetched.synced_until.insert(repo.clone(), until.clone());
    }
}

/// Fetches a stack layer that is in no list, with the membership the stack
/// gave it, and caches it so a relaunch can show it again.
///
/// Blocking: runs on the background executor.
fn fetch_layer(
    client: &GitHubClient,
    cache: &SharedCache,
    request: &LayerRequest,
) -> anyhow::Result<PullRequest> {
    let mut pull_request = client.pull_request_by_number(&request.repo, request.number)?;
    pull_request.stack = Some(request.membership.clone());
    cache.lock().put_pull_request(&pull_request)?;
    Ok(pull_request)
}
