use std::collections::HashMap;

use gpui::{
    Context, Entity, FocusHandle, ListAlignment, ListOffset, ListState, SharedString, Subscription,
    Task, Window, div, prelude::*, px,
};
use qd_cache::{BriefIndex, HiddenBranchRule, PanelVisibility, QueueTab};
use qd_github::{PullRequest, PullRequestStack, RepoRef};

use super::collapsed_groups::CollapsedGroups;
use super::fetch_queue_lists::read_cached_lists;
use super::hidden_branches::without_hidden_branches;
use super::layer_opening::OpeningLayer;
use super::others_tab::OthersTab;
use super::queue_lists::QueueLists;
use super::queue_row::QueueRow;
use super::queue_selection::PendingRestore;
use super::queue_stacks::read_cached_stacks;
use super::refresh_schedule::RefreshSchedule;
use super::refresh_status::RefreshStatus;
use super::restore_phase::RestorePhase;
use super::selection_origin::SelectionOrigin;
use super::stack_key::StackKey;
use crate::app_commands::REVIEW_QUEUE_CONTEXT;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::app_update::AppUpdater;
use crate::avatar_images::AvatarImages;
use crate::display_date::unix_now;
use crate::pull_request_detail::{DetailEvent, PullRequestDetail};
use crate::theme;
use crate::ui_state::{UiStateRecorder, read_brief_index};

/// How far past the sidebar's edges rows are laid out, so scrolling shows no gaps.
const QUEUE_LIST_OVERDRAW: f32 = 300.;

/// The main view: the viewer's own pull requests, those that request their
/// review or that they reviewed, and others' in the watched repositories on
/// the left (the Mine, Review and Others tabs, grouped by repository and
/// stack), and the selected one in a [`PullRequestDetail`] on the right.
///
/// The lists are shown from the cache at once and refreshed from GitHub in
/// the background: at launch, on cmd-r, on a timer after each refresh and
/// when the window becomes active with a stale queue.
///
/// The tab, selection and detail scroll are recorded as UI state on every
/// change and restored after a relaunch; a selected pull request that left
/// the lists falls back to no selection, unless it was opened from a stack.
///
/// Its impl is split by topic over this module's files (refresh, selection,
/// stacks, layer opening, actions, rendering); the fields those files change
/// are `pub(super)`, the rest stay private behind accessors.
pub struct ReviewQueue {
    pub(super) services: AppServices,
    focus: FocusHandle,
    /// Every pull request the last refresh (or the cache) found; the
    /// refresh brings these up to date.
    pub(super) fetched: QueueLists,
    /// The lists the tabs show: `fetched` without what the hidden branch
    /// rules hide.
    pub(super) lists: QueueLists,
    /// The hidden branch rules `lists` were made with.
    pub(super) hidden_branch_rules: Vec<HiddenBranchRule>,
    /// What the Review tab shows: `lists`' review requests and reviewed ones merged.
    pub(super) review_tab: Vec<PullRequest>,
    tab: QueueTab,
    /// Whether the sidebar (the lists) shows.
    sidebar_visibility: PanelVisibility,
    /// The sidebar rows of the shown tab.
    rows: Vec<QueueRow>,
    /// The repositories and stacks folded in the sidebar.
    collapsed: CollapsedGroups,
    pub(super) selected: Option<PullRequest>,
    pub(super) selection_origin: SelectionOrigin,
    pub(super) refresh_status: RefreshStatus,
    pub(super) refresh_schedule: RefreshSchedule,
    /// The sidebar list: rows differ in height (a stack layer is one line).
    queue_list: ListState,
    /// The right pane; it re-renders on its own when it scrolls.
    detail: Entity<PullRequestDetail>,
    /// Which pull requests have a stored brief, marked in the sidebar.
    brief_index: BriefIndex,
    /// The read of `brief_index` in flight.
    brief_index_task: Option<Task<()>>,
    /// The saved selection, until the lists it is in have loaded.
    pub(super) pending_restore: Option<PendingRestore>,
    /// The in-flight refresh; a new one starts only after it finished.
    pub(super) refresh_task: Option<Task<()>>,
    /// A refresh was asked for while one ran; it runs once that one finishes.
    pub(super) is_refresh_requested: bool,
    /// The timer waiting for the next timed refresh; any refresh replaces it.
    pub(super) auto_refresh_task: Option<Task<()>>,
    /// A stack layer in no list, being fetched to be shown.
    pub(super) opening_layer: Option<OpeningLayer>,
    /// Why the last stack layer could not be opened.
    pub(super) layer_error: Option<SharedString>,
    /// The sidebar's width, as dragged (and saved).
    sidebar_width: f32,
    /// The layers of the listed pull requests' stacks, so the sidebar shows
    /// whole stacks; from the cache at launch, checked after each refresh.
    pub(super) stacks: HashMap<StackKey, PullRequestStack>,
    /// The check of those stacks with GitHub; replacing it cancels the previous one.
    pub(super) stacks_task: Option<Task<()>>,
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

        let hidden_branch_rules = AppSettings::hidden_branch_rules(cx).to_vec();
        let shown_lists = without_hidden_branches(&lists, &hidden_branch_rules);
        let mut queue = Self {
            services,
            focus,
            review_tab: shown_lists.review_tab(),
            lists: shown_lists,
            fetched: lists,
            hidden_branch_rules,
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

    /// The lists the tabs show (without what the hidden branch rules hide).
    pub(super) fn lists(&self) -> &QueueLists {
        &self.lists
    }

    /// The hidden branch rules the shown lists were made with.
    pub(super) fn hidden_branch_rules(&self) -> &[HiddenBranchRule] {
        &self.hidden_branch_rules
    }

    /// Takes every pull request found, to show them again with
    /// [`Self::replace_lists`].
    pub(super) fn take_fetched(&mut self) -> QueueLists {
        std::mem::take(&mut self.fetched)
    }

    /// The pull requests of the shown tab: in the Review tab, review
    /// requests and reviewed ones; in the Others tab, those its filters let through.
    pub(super) fn shown_list(&self) -> &[PullRequest] {
        match self.tab {
            QueueTab::Authored => &self.lists.authored,
            QueueTab::ReviewRequested => &self.review_tab,
            QueueTab::Others => self.others_tab.shown(),
        }
    }

    /// What the Review tab shows, whichever tab is shown.
    pub(super) fn review_tab(&self) -> &[PullRequest] {
        &self.review_tab
    }

    pub(super) fn others_tab(&self) -> &OthersTab {
        &self.others_tab
    }

    pub(super) fn others_tab_mut(&mut self) -> &mut OthersTab {
        &mut self.others_tab
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

    /// Works out which of others' pull requests the Others tab shows.
    pub(super) fn update_others_shown(&mut self) {
        self.others_tab.update_shown(&self.lists, unix_now());
    }

    /// Rebuilds the sidebar rows of the shown tab, keeping the list's
    /// scroll position (a list reset alone would jump to the top).
    pub(super) fn rebuild_rows(&mut self) {
        let scroll_top = self.queue_list.logical_scroll_top();
        self.rows = match self.tab {
            QueueTab::Authored | QueueTab::ReviewRequested => {
                QueueRow::build_all(self.shown_list(), &self.collapsed, &self.stacks)
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
