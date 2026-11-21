use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    AppContext as _, Context, Entity, EventEmitter, FocusHandle, ListAlignment, ListState,
    ScrollHandle, SharedString, Subscription, Task, px,
};
use qd_cache::{
    DetailScrollPosition, DiffViewSettings, PanelVisibility, ReviewLayoutState, ReviewScope,
    StackReviewMode,
};
use qd_github::{PullRequest, PullRequestStack};
use qd_markdown::MarkdownDocument;

use super::action_panel::ActionPanel;
use super::batch_progress::BatchProgress;
use super::brief_handlers::detail_brief_handlers;
use super::brief_pane::BriefPane;
use super::changed_files::ChangedFiles;
use super::context_loads::ContextLoads;
use super::detail_event::DetailEvent;
use super::detail_row_list::DetailRowList;
use super::diff_selection::DiffSelection;
use super::fetch_changed_files::fetch_changed_files;
use super::file_review_state::FileReviewState;
use super::file_tree_state::FileTreeState;
use super::load_progress::LoadProgress;
use super::loaded_file::LoadedFile;
use super::parsed_description::ParsedDescription;
use super::review_pane::ReviewPane;
use super::review_target::ReviewTarget;
use super::scroll_anchor::ScrollAnchor;
use super::show_change::ShowChange;
use super::smart_filter_state::SmartFilterState;
use super::stack_state::StackState;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::avatar_images::{AvatarImages, AvatarLoaded};
use crate::checks_panel::ChecksPanel;
use crate::diff_view::CodeSegments;
use crate::scrollbar::ListRowTops;
use crate::ui_state::UiStateRecorder;

/// How far beyond the visible area rows are laid out, so fast scrolling
/// rarely shows rows before they are drawn.
const LIST_OVERDRAW: f32 = 600.;

/// The right pane: a sticky header with the shown pull request's title and
/// view tabs, the changed files as one virtualized list of rows (or the
/// summary or AI-written brief in their place), and the side
/// panel beside them with its Files, Review, Actions and Checks tabs. Only
/// rows in view are rendered, so its cost does not grow with the pull
/// request. The tree and the diff share one file order and one filter; the
/// review's comment cards sit under their lines in the diff.
///
/// Files are read from the cache when current for the pull request's
/// `updated_at`, otherwise fetched and cached; diffs are built and colored
/// off the main thread. Emits [`DetailEvent::Scrolled`] when it scrolls.
pub struct PullRequestDetail {
    pub(super) services: AppServices,
    pub(super) pull_request: Option<PullRequest>,
    focus_mode: bool,
    description: Option<ParsedDescription>,
    pub(super) changed_files: Option<ChangedFiles>,
    /// How diffs show, as last applied from [`AppSettings`].
    pub(super) diff_settings: DiffViewSettings,
    /// Which loaded files are viewed and collapsed.
    pub(super) file_review: FileReviewState,
    pub(super) rows: DetailRowList,
    pub(super) list: ListState,
    /// The file the reader last chose using the tree, keyboard, diff header or code.
    /// While its header is visible this disambiguates short files sharing the viewport.
    pub(super) selected_diff_file: Option<usize>,
    /// How far each file's lines are scrolled sideways when lines do not wrap.
    pub(super) horizontal_offsets: HashMap<usize, f32>,
    /// A scroll position to apply once the files are loaded (a restore after
    /// relaunch, or the position kept across a reload of the same pull request).
    pub(super) pending_scroll: Option<DetailScrollPosition>,
    /// The in-flight file load; replacing it cancels the previous one.
    pub(super) files_task: Option<Task<()>>,
    /// The rebuild of the loaded diffs after a settings change.
    pub(super) diff_task: Option<Task<()>>,
    /// The syntax highlighting of the loaded diffs.
    pub(super) syntax_task: Option<Task<()>>,
    /// File texts being fetched to reveal lines around hunks.
    pub(super) context_loads: ContextLoads,
    /// Focused when code is pressed, so cmd-c copies the diff selection.
    pub(super) focus: FocusHandle,
    /// The code texts painted this frame, for mouse hit testing.
    pub(super) code_segments: CodeSegments,
    pub(super) diff_selection: Option<DiffSelection>,
    /// Commentable patch range of the last completed drag.
    pub(super) selected_comment_range: Option<super::comment_range::CommentRange>,
    pub(super) diff_find: Option<super::diff_find::DiffFind>,
    /// Whether a press in the code is being dragged.
    pub(super) is_selecting: bool,
    /// The file tree panel: the files as a tree, the filter, folders, visibility.
    pub(super) file_tree: FileTreeState,
    pub(super) focus_drawer: super::focus_drawer::FocusDrawer,
    pub(super) walkthrough: super::review_walkthrough::ReviewWalkthrough,
    pub(super) viewed_undo: Vec<super::viewed_change::ViewedChange>,
    /// The Summary tab's scroll, reset for each new pull request.
    summary_scroll: ScrollHandle,
    /// Whether stacked pull requests show one layer or combined layers: the
    /// reader's preference, restored at launch and kept when nothing shows.
    pub(super) review_mode: StackReviewMode,
    /// What the shown (or loading) files are of.
    pub(super) target: ReviewTarget,
    /// Whose viewed marks the shown files use.
    pub(super) viewed_scope: Option<ReviewScope>,
    /// The shown pull request's stack: its layers, their load and the
    /// combined range's start.
    pub(super) stack: StackState,
    /// Syntax batches colored of all started, while highlighting runs.
    pub(super) highlight_progress: Option<BatchProgress>,
    /// The side panel's width, as dragged (and saved).
    side_panel_width: f32,
    /// Where each row starts, estimated for the scrollbar (the list lays
    /// out only the rows on screen), and the pane width it was estimated
    /// for; `None` once the rows changed.
    pub(super) row_tops: ListRowTops,
    pub(super) row_tops_width: Option<f32>,
    /// Review comments, the comment being written, and the review tab.
    pub(super) review: ReviewPane,
    /// The merge or close panel in the Actions tab, while open.
    action_panel: Option<ActionPanel>,
    deferred_reload: Option<PullRequest>,
    layer_refresh: Option<LayerRefresh>,
    pub(super) layer_refresh_error: Option<SharedString>,
    background_panels: HashMap<u64, ActionPanel>,
    /// The selected detail mode, AI-written briefs and the run in flight.
    pub(super) brief: BriefPane,
    /// The head commit of the shown files, when known: what a brief's
    /// version is compared with.
    pub(super) shown_head_sha: Option<String>,
    pub(super) shown_base_sha: Option<String>,
    pub(super) smart_filters: HashMap<ReviewScope, SmartFilterState>,
    /// The side panel's Checks tab, following the shown pull request.
    pub(super) checks: Entity<ChecksPanel>,
    /// Keep the observers of [`AppSettings`] and the avatars (header and
    /// comment authors) registered for the view's lifetime.
    _subscriptions: [Subscription; 2],
}

/// The queue version being checked against the open layer. Dropping it cancels delivery.
struct LayerRefresh {
    pull_request: PullRequest,
    _task: Task<()>,
}

impl EventEmitter<DetailEvent> for PullRequestDetail {}

impl PullRequestDetail {
    /// An empty pane that follows the diff settings in [`AppSettings`].
    pub fn new(services: AppServices, cx: &mut Context<Self>) -> Self {
        let diff_settings = AppSettings::diff_view(cx);
        let settings_subscription = cx.observe_global::<AppSettings>(|detail, cx| {
            detail.apply_diff_settings(AppSettings::diff_view(cx), cx);
        });

        // Only the rows on screen are laid out: measuring every line of a
        // large pull request at once blocks the window for seconds. The
        // scrollbar works from `row_tops` instead.
        let list = ListState::new(0, ListAlignment::Top, px(LIST_OVERDRAW));
        let detail = cx.entity().downgrade();
        list.set_scroll_handler(move |_, _, cx| {
            detail
                .update(cx, |detail, cx| {
                    // Scrolling by hand: the file at the top is current again.
                    detail.selected_diff_file = None;
                    cx.emit(DetailEvent::Scrolled);
                })
                .ok();
        });

        let layout = cx
            .global::<UiStateRecorder>()
            .saved_review_layout()
            .unwrap_or_default();
        let checks = cx.new(|_| ChecksPanel::new(services.clone()));
        let search_input = cx.new(|cx| crate::text_area::TextArea::single_line("Find files…", cx));
        let mut detail = Self {
            services,
            pull_request: None,
            focus_mode: false,
            description: None,
            changed_files: None,
            diff_settings,
            file_review: FileReviewState::default(),
            rows: DetailRowList::default(),
            list,
            selected_diff_file: None,
            horizontal_offsets: HashMap::new(),
            pending_scroll: None,
            files_task: None,
            diff_task: None,
            syntax_task: None,
            context_loads: ContextLoads::default(),
            focus: cx.focus_handle(),
            code_segments: CodeSegments::default(),
            diff_selection: None,
            selected_comment_range: None,
            diff_find: None,
            is_selecting: false,
            file_tree: FileTreeState::new(layout.clone()),
            focus_drawer: super::focus_drawer::FocusDrawer::new(
                layout.files_mode,
                search_input.clone(),
            ),
            walkthrough: super::review_walkthrough::ReviewWalkthrough::default(),
            viewed_undo: Vec::new(),
            summary_scroll: ScrollHandle::new(),
            review_mode: layout.stack_review,
            target: ReviewTarget::Layer,
            viewed_scope: None,
            stack: StackState::default(),
            highlight_progress: None,
            row_tops: ListRowTops::from_heights([]),
            row_tops_width: None,
            action_panel: None,
            deferred_reload: None,
            layer_refresh: None,
            layer_refresh_error: None,
            background_panels: HashMap::new(),
            brief: BriefPane::new(detail_brief_handlers(cx)),
            shown_head_sha: None,
            shown_base_sha: None,
            smart_filters: HashMap::new(),
            checks,
            review: ReviewPane {
                destination: layout.comment_destination,
                ..ReviewPane::default()
            },
            side_panel_width: cx
                .global::<UiStateRecorder>()
                .saved_panel_widths()
                .unwrap_or_default()
                .side_panel,
            _subscriptions: [
                settings_subscription,
                // Only the avatars it shows redraw it, not every author the queue lists.
                cx.subscribe(
                    &AvatarImages::global(cx),
                    |detail, _, event: &AvatarLoaded, cx| {
                        if detail.shows_avatar(&event.url) {
                            cx.notify();
                        }
                    },
                ),
            ],
        };
        detail.focus_drawer.subscription = Some(cx.subscribe(
            &search_input,
            |detail, _, event: &crate::text_area::TextAreaEvent, cx| {
                detail.on_file_search(*event, cx);
            },
        ));
        detail.brief.mode = match layout.files_mode {
            qd_cache::FilesMode::Tree => super::detail_mode::DetailMode::Diff,
            qd_cache::FilesMode::Plan => super::detail_mode::DetailMode::GuidedReview,
        };
        detail.rebuild_rows(ScrollAnchor::Top);
        detail
    }

    /// Guided rows stay isolated from ordinary browsing even when a selected step is retained.
    pub(super) fn guided_diff_active(&self) -> bool {
        self.brief.mode == super::detail_mode::DetailMode::GuidedReview
            && self.walkthrough.selected_group.is_some()
    }

    pub(super) fn list(&self) -> &ListState {
        if self.guided_diff_active() {
            &self.walkthrough.group_diff.list
        } else {
            &self.list
        }
    }

    pub(super) fn rows(&self) -> &DetailRowList {
        if self.guided_diff_active() {
            &self.walkthrough.group_diff.rows
        } else {
            &self.rows
        }
    }

    pub(super) fn focus(&self) -> &FocusHandle {
        &self.focus
    }

    pub(super) fn code_segments(&self) -> &CodeSegments {
        &self.code_segments
    }

    pub(super) fn is_selecting(&self) -> bool {
        self.is_selecting
    }

    pub(super) fn pull_request(&self) -> Option<&PullRequest> {
        self.pull_request.as_ref()
    }

    pub(super) fn description(&self) -> Option<&Arc<MarkdownDocument>> {
        self.description
            .as_ref()
            .and_then(ParsedDescription::document)
    }

    /// The loaded files to change in place, once loaded.
    pub(super) fn loaded_files_mut(&mut self) -> Option<&mut Vec<LoadedFile>> {
        self.changed_files
            .as_mut()
            .and_then(ChangedFiles::loaded_mut)
    }

    pub(super) fn context_loads(&self) -> &ContextLoads {
        &self.context_loads
    }

    pub(super) fn context_loads_mut(&mut self) -> &mut ContextLoads {
        &mut self.context_loads
    }

    pub(super) fn diff_settings(&self) -> DiffViewSettings {
        self.diff_settings
    }

    pub(super) fn file_review(&self) -> &FileReviewState {
        &self.file_review
    }

    /// How far file `file`'s lines are scrolled sideways.
    pub(super) fn horizontal_offset(&self, file: usize) -> f32 {
        self.horizontal_offsets
            .get(&file)
            .copied()
            .unwrap_or_default()
    }

    /// The loaded files, or none while loading or after a failure.
    pub(super) fn loaded_files(&self) -> &[LoadedFile] {
        self.changed_files
            .as_ref()
            .map_or(&[], ChangedFiles::loaded)
    }

    /// Why the files' load failed, if it did.
    pub(super) fn load_failure(&self) -> Option<&SharedString> {
        self.changed_files.as_ref().and_then(ChangedFiles::failure)
    }

    /// How far the files' load has come, while they load.
    pub(super) fn load_progress(&self) -> Option<LoadProgress> {
        self.changed_files.as_ref().and_then(ChangedFiles::progress)
    }

    /// Syntax batches colored of all, while highlighting runs.
    pub(super) fn highlight_progress(&self) -> Option<BatchProgress> {
        self.highlight_progress
    }

    pub(super) fn review_mode(&self) -> StackReviewMode {
        self.review_mode
    }

    pub(super) fn target(&self) -> &ReviewTarget {
        &self.target
    }

    /// The layers available for reading, in ancestry order. Merge and delete use
    /// the native stack separately, since inference grants no GitHub membership.
    pub(super) fn review_layers(&self) -> Option<&[qd_github::StackLayer]> {
        self.stack
            .queue_path
            .as_deref()
            .or_else(|| self.stack().map(|stack| stack.layers.as_slice()))
    }

    pub(super) fn has_review_stack(&self) -> bool {
        self.stack.queue_path.is_some()
            || self
                .pull_request
                .as_ref()
                .is_some_and(|request| request.stack.is_some())
    }

    pub(super) fn stack(&self) -> Option<&PullRequestStack> {
        self.stack.loaded.as_ref()
    }

    pub(super) fn side_panel_width(&self) -> f32 {
        self.side_panel_width
    }

    /// Sets the side panel's width while its divider is dragged, and saves it.
    pub(super) fn set_side_panel_width(&mut self, width: f32, cx: &mut Context<Self>) {
        self.side_panel_width = width;
        UiStateRecorder::record_side_panel_width(cx, width);
        cx.notify();
    }

    pub(super) fn is_stack_loading(&self) -> bool {
        self.stack.is_loading
    }

    pub(super) fn stack_error(&self) -> Option<&SharedString> {
        self.stack.error.as_ref()
    }

    /// The position of the shown pull request in its loaded stack.
    pub(super) fn current_layer_index(&self) -> Option<usize> {
        let number = self.pull_request.as_ref()?.number;
        self.stack
            .loaded
            .as_ref()?
            .layers
            .iter()
            .position(|layer| layer.number == number)
    }

    pub(super) fn action_panel(&self) -> Option<&ActionPanel> {
        self.action_panel.as_ref()
    }

    pub(super) fn action_panel_mut(&mut self) -> Option<&mut ActionPanel> {
        self.action_panel.as_mut()
    }

    pub(super) fn panel_for_operation(&mut self, id: u64) -> Option<&mut ActionPanel> {
        if self.pull_request.as_ref().is_some_and(|pr| pr.id == id) {
            self.action_panel.as_mut()
        } else {
            self.background_panels.get_mut(&id)
        }
    }

    fn switch_action_panel(&mut self, next: Option<u64>) {
        if let (Some(pr), Some(panel)) = (&self.pull_request, self.action_panel.take())
            && panel.is_working()
        {
            self.background_panels.insert(pr.id, panel);
        }
        self.action_panel = next.and_then(|id| self.background_panels.remove(&id));
    }

    pub(super) fn set_action_panel(&mut self, panel: Option<ActionPanel>) {
        self.action_panel = panel;
    }

    pub(super) fn review(&self) -> &ReviewPane {
        &self.review
    }

    /// Whether the avatar at `url` shows here: the shown pull request's
    /// author's, or a comment author's in its GitHub conversations.
    fn shows_avatar(&self, url: &str) -> bool {
        let is_author = self
            .pull_request
            .as_ref()
            .is_some_and(|pull_request| pull_request.author.avatar_url == url);
        let is_commenter = self.review.comments.github.as_ref().is_some_and(|state| {
            state
                .threads
                .iter()
                .flat_map(|thread| &thread.comments)
                .any(|comment| comment.author_avatar_url == url)
        });
        is_author || is_commenter
    }

    /// The head commit of the shown files, when known.
    pub(super) fn shown_head_sha(&self) -> Option<&str> {
        self.shown_head_sha.as_deref()
    }

    pub(super) fn brief(&self) -> &BriefPane {
        &self.brief
    }

    pub(super) fn brief_mut(&mut self) -> &mut BriefPane {
        &mut self.brief
    }

    pub(super) fn review_mut(&mut self) -> &mut ReviewPane {
        &mut self.review
    }

    pub(super) fn services(&self) -> &AppServices {
        &self.services
    }

    /// Whose viewed marks and notes the shown files use, once loaded.
    pub(super) fn review_scope(&self) -> Option<&ReviewScope> {
        self.viewed_scope.as_ref()
    }

    /// The layout to save: the file tree's part and the review mode.
    pub(super) fn review_layout(&self) -> ReviewLayoutState {
        ReviewLayoutState {
            files_mode: self.focus_drawer.mode,
            stack_review: self.review_mode,
            comment_destination: self.review.destination,
            ..self.file_tree.layout()
        }
    }

    /// Shows the selected PR. A refresh keeps the open diff until its commits or target
    /// change. Navigation starts at the top or restores the position saved at relaunch.
    pub fn show(
        &mut self,
        pull_request: Option<PullRequest>,
        restore: Option<DetailScrollPosition>,
        cx: &mut Context<Self>,
    ) {
        // Both stages of a queue refresh can deliver the same version.
        if self
            .layer_refresh
            .as_ref()
            .is_some_and(|pending| Some(&pending.pull_request) == pull_request.as_ref())
        {
            return;
        }
        self.layer_refresh = None;
        self.deferred_reload = None;
        let Some(pull_request) = pull_request else {
            self.focus_drawer.reset_contents();
            self.viewed_undo.clear();
            self.switch_action_panel(None);
            self.pull_request = None;
            self.layer_refresh_error = None;
            self.description = None;
            self.changed_files = None;
            self.viewed_scope = None;
            self.shown_head_sha = None;
            self.shown_base_sha = None;
            self.reset_review();
            self.files_task = None;
            self.pending_scroll = None;
            // A new stack state also cancels the stack's load.
            self.stack = StackState::default();
            self.rebuild_rows(ScrollAnchor::Top);
            self.follow_checks(cx);
            cx.notify();
            return;
        };
        // Decide by what was shown before, then carry it out.
        let change = ShowChange::decide(
            self.pull_request.as_ref(),
            self.stack.loaded.as_ref(),
            &pull_request,
        );
        match change {
            ShowChange::SameVersion { loads_stack } => {
                let retry_files = matches!(self.changed_files, Some(ChangedFiles::Failed(_)));
                if self.pull_request.as_ref() == Some(&pull_request)
                    && !retry_files
                    && (!loads_stack || self.stack.is_loading)
                {
                    return;
                }
                self.update_metadata(pull_request, cx);
                if retry_files {
                    self.load_changed_files(cx);
                }
                if loads_stack && !self.stack.is_loading {
                    self.load_stack(cx);
                }
                self.follow_checks(cx);
                cx.notify();
            }
            ShowChange::NewVersion { is_other_stack } => {
                if self.review.editor.is_some() {
                    self.deferred_reload = Some(pull_request);
                    return;
                }
                if !is_other_stack {
                    match (&self.target, &self.changed_files) {
                        (ReviewTarget::Layer, Some(ChangedFiles::Loaded(_))) => {
                            self.check_layer_refresh(pull_request, cx);
                            return;
                        }
                        (
                            ReviewTarget::Combined(_),
                            Some(ChangedFiles::Loaded(_) | ChangedFiles::Loading(_)),
                        ) => {
                            // The queue checks the stack separately. Its range identifies the files.
                            self.update_metadata(pull_request, cx);
                            self.follow_checks(cx);
                            cx.notify();
                            return;
                        }
                        _ => {}
                    }
                }
                self.pending_scroll = restore.or_else(|| Some(self.scroll_position()));
                if is_other_stack {
                    self.stack.drop_layers();
                }
                self.show_new_version(pull_request, cx);
            }
            ShowChange::OtherPullRequest { is_other_stack } => {
                // Another pull request starts with no action panel and its summary at the top.
                self.pending_scroll = restore;
                self.switch_action_panel(Some(pull_request.id));
                self.summary_scroll.set_offset(gpui::point(px(0.), px(0.)));
                if is_other_stack {
                    self.stack.drop_layers();
                }
                self.show_new_version(pull_request, cx);
            }
        }
    }

    /// Verifies the refreshed layer in the background while the reader keeps using its diff.
    fn check_layer_refresh(&mut self, pull_request: PullRequest, cx: &mut Context<Self>) {
        let client = self.services.github.clone();
        let cache = self.services.cache.clone();
        let next = pull_request.clone();
        let task = cx.spawn(async move |detail, cx| {
            let revision = cx
                .background_executor()
                .spawn(async move {
                    fetch_changed_files(client, &cache, &next, &mut |_| {})
                        .map(|snapshot| snapshot.revision)
                })
                .await;
            detail
                .update(cx, |detail, cx| {
                    let Some(pending) = detail.layer_refresh.take() else {
                        return;
                    };
                    let next = pending.pull_request;
                    // A new range owns its comparison, and an open editor defers the refresh.
                    if detail.target != ReviewTarget::Layer || detail.review.editor.is_some() {
                        detail.show(Some(next), None, cx);
                        return;
                    }
                    let revision = match revision {
                        Ok(revision) => revision,
                        Err(error) => {
                            // Keep the last confirmed version. The next refresh will retry it.
                            detail.layer_refresh_error = Some(format!("{error:#}").into());
                            cx.notify();
                            return;
                        }
                    };
                    if detail.shown_head_sha.as_ref() == Some(&revision.head_sha)
                        && detail.shown_base_sha.as_ref() == Some(&revision.base_sha)
                    {
                        detail.update_metadata(next, cx);
                        detail.follow_checks(cx);
                        detail.load_github_review(cx);
                        cx.notify();
                    } else {
                        detail.pending_scroll = Some(detail.scroll_position());
                        detail.show_new_version(next, cx);
                    }
                })
                .ok();
        });
        self.layer_refresh = Some(LayerRefresh {
            pull_request,
            _task: task,
        });
    }

    pub(super) fn resume_deferred_reload(&mut self, cx: &mut Context<Self>) {
        if let Some(pr) = self.deferred_reload.take() {
            self.show(Some(pr), None, cx);
        }
        // A stack update can also have arrived while the editor held its range.
        self.retarget(cx);
    }

    /// Installs the review path before the first file request for a selection.
    pub(crate) fn show_queue_selection(
        &mut self,
        pull_request: Option<PullRequest>,
        restore: Option<DetailScrollPosition>,
        path: Option<Vec<qd_github::StackLayer>>,
        cx: &mut Context<Self>,
    ) {
        if self.pull_request.as_ref().map(|pr| &pr.repo) != pull_request.as_ref().map(|pr| &pr.repo)
            || self
                .stack
                .queue_path
                .as_ref()
                .and_then(|layers| layers.first())
                .map(|layer| layer.number)
                != path
                    .as_ref()
                    .and_then(|layers| layers.first())
                    .map(|layer| layer.number)
        {
            self.stack.combined_start = None;
        }
        let changed = self.stack.queue_path != path;
        self.stack.queue_path = path;
        self.show(pull_request, restore, cx);
        if changed {
            self.retarget(cx);
            self.follow_checks(cx);
            cx.notify();
        }
    }

    /// Re-evaluates an open comparison after the queue learns new parent links or commits.
    pub(crate) fn set_queue_path(
        &mut self,
        path: Option<Vec<qd_github::StackLayer>>,
        cx: &mut Context<Self>,
    ) {
        if self.stack.queue_path != path {
            self.stack.queue_path = path;
            self.retarget(cx);
            self.follow_checks(cx);
            cx.notify();
        }
    }

    /// Shows a version of `pull_request` not shown before: the header and
    /// description now, then its files and stack.
    fn show_new_version(&mut self, pull_request: PullRequest, cx: &mut Context<Self>) {
        self.update_metadata(pull_request, cx);
        self.horizontal_offsets.clear();

        // New commits can move the layers' heads, so the stack is checked again (free when unchanged).
        self.target = self.decide_target();
        cx.emit(DetailEvent::TargetChanged);
        self.load_changed_files(cx);
        self.load_stack(cx);
        self.follow_checks(cx);
    }

    /// Updates the header and keeps the summary's text selection while its source is identical.
    fn update_metadata(&mut self, pull_request: PullRequest, cx: &mut Context<Self>) {
        AvatarImages::load(cx, [pull_request.author.avatar_url.clone()]);
        self.layer_refresh_error = None;
        if self
            .pull_request
            .as_ref()
            .is_none_or(|shown| shown.id != pull_request.id || shown.body != pull_request.body)
        {
            self.description = Some(ParsedDescription::parse(&pull_request));
        }
        self.pull_request = Some(pull_request);
    }

    /// The side panel's Checks tab.
    pub(super) fn checks(&self) -> &Entity<ChecksPanel> {
        &self.checks
    }

    /// Whether a pull request is shown (the side panel can be toggled).
    pub fn has_pull_request(&self) -> bool {
        self.pull_request.is_some()
    }

    /// Whether the current view fills the review window.
    pub fn is_focus_mode(&self) -> bool {
        self.focus_mode
    }

    /// Switches the presentation without changing the review or panel preferences.
    /// Returns whether the presentation changed.
    pub fn toggle_focus_mode(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.focus_mode && !self.has_pull_request() {
            return false;
        }
        self.focus_mode = !self.focus_mode;
        self.sync_checks_visibility(cx);
        cx.notify();
        true
    }

    /// Whether the side panel is visible in the current presentation.
    pub fn is_side_panel_shown(&self) -> bool {
        !self.focus_mode && self.file_tree.visibility() == PanelVisibility::Shown
    }

    /// The scroll position to save: one still waiting to be applied, or the
    /// list's current one.
    pub fn scroll_position(&self) -> DetailScrollPosition {
        self.pending_scroll.unwrap_or_else(|| {
            let top = self.list.logical_scroll_top();
            DetailScrollPosition {
                row: top.item_ix,
                offset_in_row: top.offset_in_item.into(),
            }
        })
    }

    pub(super) fn file_tree(&self) -> &FileTreeState {
        &self.file_tree
    }

    pub(super) fn summary_scroll(&self) -> &ScrollHandle {
        &self.summary_scroll
    }
}

#[cfg(test)]
mod refresh_tests {
    use super::super::action_panel::PanelAction;
    use super::super::comment_editor::{CommentEditor, EditorTarget};
    use super::super::operation_status::OperationStatus;
    use super::*;
    use crate::pull_request_detail::tests::cached_review::{cached_services, pull_request};
    use crate::shared_cache::SharedCache;
    use crate::text_area::{TextArea, TextAreaEvent};

    /// Reopens a cached diff before the viewed-mark writer runs, then lets the writer finish.
    fn check_viewed_reload(cx: &mut gpui::TestAppContext, initially_viewed: bool) {
        use qd_cache::{FileSetRevision, PullRequestListKey, ReviewScope};
        use qd_github::{PullRequestFile, PullRequestRevision};

        // Both PRs contain the same file, so navigating also checks that marks stay scoped.
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("cache");
        let recorder = UiStateRecorder::open(&database).unwrap();
        let store = recorder.shared_store();
        let mut cache = qd_cache::GitHubCache::open(&database).unwrap();
        let first = pull_request(1, "current");
        let second = pull_request(2, "current");
        let scope = ReviewScope::PullRequest(first.id);
        let file = PullRequestFile {
            filename: "a.rs".into(),
            previous_filename: None,
            status: "added".into(),
            additions: 1,
            deletions: 0,
            patch: Some("@@ -0,0 +1 @@\n+hello".into()),
            sha: None,
        };
        let revision = FileSetRevision {
            commits: PullRequestRevision {
                head_sha: "head".into(),
                base_sha: "base".into(),
            },
            etag: None,
        };
        cache
            .replace_list(
                PullRequestListKey::REVIEW_REQUESTED,
                &[first.clone(), second.clone()],
            )
            .unwrap();
        for pull_request in [&first, &second] {
            cache
                .put_files(pull_request, std::slice::from_ref(&file), &revision)
                .unwrap();
        }
        let services = AppServices {
            github: Err("offline".into()),
            token_source: None,
            cache: SharedCache::new(cache),
            database,
        };
        cx.update(|app| {
            app.set_global(AppSettings::load(&recorder));
            app.set_global(recorder);
            AvatarImages::install(app);
        });
        let detail = cx.new(|cx| PullRequestDetail::new(services, cx));
        let show = |pull_request: &PullRequest, cx: &mut gpui::TestAppContext| {
            detail.update(cx, |detail, cx| {
                detail.show(Some(pull_request.clone()), None, cx);
            });
            cx.run_until_parked();
            detail.update(cx, |detail, _| assert_eq!(detail.loaded_files().len(), 1));
        };
        let flush = |cx: &mut gpui::TestAppContext| {
            cx.run_until_parked();
            cx.executor()
                .advance_clock(std::time::Duration::from_secs(1));
            cx.run_until_parked();
        };
        let saved = || !store.lock().viewed_files(&scope).unwrap().is_empty();

        // Start the unmark case with a durable mark, then make the edit under test.
        show(&first, cx);
        if initially_viewed {
            detail.update(cx, |detail, cx| detail.toggle_file_viewed(0, cx));
            flush(cx);
        }
        detail.update(cx, |detail, cx| {
            detail.toggle_file_viewed(0, cx);
            if !detail.file_tree().filter().hide_viewed {
                detail.toggle_hide_viewed(cx);
            }
        });
        show(&second, cx);
        detail.update(cx, |detail, _| {
            assert_eq!(detail.file_review.viewed_count(), 0);
        });
        show(&first, cx);
        assert_eq!(
            saved(),
            initially_viewed,
            "reload finished before the write"
        );

        // The checkbox, fold and Hide viewed filter agree on either side of the write.
        let assert_display = |cx: &mut gpui::TestAppContext| {
            detail.update(cx, |detail, _| {
                let state = detail.file_review.state_of(&detail.loaded_files()[0]);
                assert_eq!(state.is_viewed, !initially_viewed);
                assert_eq!(state.is_collapsed, !initially_viewed);
                assert_eq!(detail.rows().file_start(0).is_some(), initially_viewed);
            });
        };
        assert_display(cx);
        flush(cx);
        assert_eq!(saved(), !initially_viewed);
        assert_display(cx);
    }

    /// Returning before the batched save keeps a newly viewed file checked and hidden.
    #[gpui::test]
    fn viewed_reload_keeps_pending_mark(cx: &mut gpui::TestAppContext) {
        check_viewed_reload(cx, false);
    }

    /// Returning before the batched save keeps a newly unviewed file open and visible.
    #[gpui::test]
    fn viewed_reload_keeps_pending_unmark(cx: &mut gpui::TestAppContext) {
        check_viewed_reload(cx, true);
    }

    /// Opens a detail pane backed by an isolated cache, with no network client.
    fn cached_detail(
        cx: &mut gpui::TestAppContext,
        directory: &std::path::Path,
    ) -> (Entity<PullRequestDetail>, SharedCache) {
        let services = cached_services(cx, directory);
        let cache = services.cache.clone();
        (cx.new(|cx| PullRequestDetail::new(services, cx)), cache)
    }

    /// A later timestamp keeps the reader's diff and search until a commit actually changes.
    #[gpui::test]
    fn metadata_refresh_keeps_the_open_diff(cx: &mut gpui::TestAppContext) {
        use qd_cache::{FileSetRevision, PullRequestListKey};
        use qd_github::{PullRequestFile, PullRequestRevision};

        // Open an ordinary cached review, then collapse a file and search for it.
        let directory = tempfile::tempdir().unwrap();
        let (detail, cache) = cached_detail(cx, directory.path());
        let mut pull_request = pull_request(1, "t1");
        pull_request.body = Some("Original summary".into());
        let files = [PullRequestFile {
            filename: "needle.rs".into(),
            previous_filename: None,
            status: "added".into(),
            additions: 1,
            deletions: 0,
            patch: Some("@@ -0,0 +1 @@\n+hello".into()),
            sha: None,
        }];
        let mut revision = FileSetRevision {
            commits: PullRequestRevision {
                head_sha: "head".into(),
                base_sha: "base".into(),
            },
            etag: None,
        };
        cache
            .lock()
            .replace_list(
                PullRequestListKey::AUTHORED,
                std::slice::from_ref(&pull_request),
            )
            .unwrap();
        cache
            .lock()
            .put_files(&pull_request, &files, &revision)
            .unwrap();
        detail.update(cx, |detail, cx| {
            detail.show(Some(pull_request.clone()), None, cx);
        });
        cx.run_until_parked();
        let (diff, summary) = detail.update(cx, |detail, cx| {
            detail.toggle_file_collapsed(0, cx);
            detail
                .focus_drawer
                .input
                .update(cx, |input, cx| input.set_text("needle", cx));
            (
                detail.loaded_files()[0].diff.clone(),
                detail.description().unwrap().clone(),
            )
        });

        // A failed check keeps the diff available. Confirming it on retry preserves the same objects.
        pull_request.updated_at = "t2".into();
        pull_request.title = "Updated title".into();
        detail.update(cx, |detail, cx| {
            detail.show(Some(pull_request.clone()), None, cx);
        });
        cx.run_until_parked();
        detail.update(cx, |detail, _| {
            assert!(detail.layer_refresh_error.is_some());
            assert_eq!(detail.file_tree.search.query, "needle");
        });
        cache
            .lock()
            .confirm_files(&pull_request, &revision)
            .unwrap();
        detail.update(cx, |detail, cx| {
            detail.show(Some(pull_request.clone()), None, cx);
            detail.show(Some(pull_request.clone()), None, cx);
            assert_eq!(detail.file_tree.search.query, "needle");
        });
        cx.run_until_parked();
        detail.update(cx, |detail, _| {
            assert!(detail.layer_refresh_error.is_none());
            assert_eq!(detail.pull_request.as_ref(), Some(&pull_request));
            assert_eq!(detail.file_tree.search.query, "needle");
            let file = &detail.loaded_files()[0];
            assert!(Arc::ptr_eq(&file.diff, &diff));
            assert!(detail.file_review.state_of(file).is_collapsed);
            assert!(Arc::ptr_eq(detail.description().unwrap(), &summary));
        });

        // A base-only change still replaces the displayed revision.
        pull_request.updated_at = "t3".into();
        revision.commits.base_sha = "new-base".into();
        cache
            .lock()
            .put_files(&pull_request, &files, &revision)
            .unwrap();
        detail.update(cx, |detail, cx| {
            detail.show(Some(pull_request.clone()), None, cx);
        });
        cx.run_until_parked();
        detail.update(cx, |detail, _| {
            assert_eq!(detail.shown_base_sha.as_deref(), Some("new-base"));
            assert!(!Arc::ptr_eq(&detail.loaded_files()[0].diff, &diff));
        });
    }

    fn layer(number: u64) -> qd_github::StackLayer {
        qd_github::StackLayer {
            number,
            pull_request_id: number,
            title: String::new(),
            is_open: true,
            is_draft: false,
            is_merged: false,
            html_url: String::new(),
            head_repository: None,
            head_ref: format!("layer-{number}"),
            head_sha: format!("head-{number}"),
            base_ref: "main".into(),
            base_sha: "base".into(),
            author_login: String::new(),
        }
    }

    /// An inferred path drives the displayed compare even when the selected PR has no native stack.
    #[gpui::test]
    fn inferred_review_follows_the_selected_ancestry_on_refresh(cx: &mut gpui::TestAppContext) {
        let directory = tempfile::tempdir().unwrap();
        let (detail, cache) = cached_detail(cx, directory.path());
        let leaf = pull_request(4, "t1");
        let mut path = vec![layer(1), layer(4)];
        for base in ["base", "new-base"] {
            cache
                .lock()
                .put_compare_files(&leaf.repo, base, "head-4", &[])
                .unwrap();
        }
        detail.update(cx, |detail, cx| {
            detail.prepare_combined_stack_review(cx);
            detail.show_queue_selection(Some(leaf.clone()), None, Some(path.clone()), cx);
        });
        cx.run_until_parked();
        detail.update(cx, |detail, cx| {
            assert_eq!(detail.shown_base_sha.as_deref(), Some("base"));
            assert_eq!(detail.shown_head_sha.as_deref(), Some("head-4"));
            assert!(detail.stack().is_none());
            assert!(!detail.start_combined_at(&leaf.repo, 2, cx));

            path[0].base_sha = "new-base".into();
            detail.set_queue_path(Some(path), cx);
        });
        cx.run_until_parked();
        detail.update(cx, |detail, cx| {
            assert_eq!(detail.shown_base_sha.as_deref(), Some("new-base"));
            assert_eq!(detail.shown_head_sha.as_deref(), Some("head-4"));
            detail.set_queue_path(None, cx);
            assert_eq!(detail.target, ReviewTarget::Layer);
        });
    }

    /// A combined review keeps its range on metadata updates and follows a changed lower base.
    #[gpui::test]
    fn stack_refresh_follows_the_combined_range(cx: &mut gpui::TestAppContext) {
        use qd_github::StackMembership;

        let directory = tempfile::tempdir().unwrap();
        let (detail, cache) = cached_detail(cx, directory.path());
        let mut top = pull_request(2, "t1");
        top.stack = Some(StackMembership {
            number: 9,
            position: 2,
            size: 2,
            base_ref: "main".into(),
        });
        let mut stack = PullRequestStack {
            repo: top.repo.clone(),
            number: 9,
            base_ref: "main".into(),
            is_open: true,
            layers: vec![layer(1), layer(2)],
        };

        // Open the actual combined comparison and give the reader a search to preserve.
        cache.lock().put_stack(&stack, None).unwrap();
        for base in ["base", "new-base"] {
            cache
                .lock()
                .put_compare_files(&top.repo, base, "head-2", &[])
                .unwrap();
        }
        detail.update(cx, |detail, cx| {
            detail.prepare_combined_stack_review(cx);
            detail.show(Some(top.clone()), None, cx);
        });
        cx.run_until_parked();
        detail.update(cx, |detail, cx| {
            assert_eq!(detail.shown_base_sha.as_deref(), Some("base"));
            detail.file_tree.search.query = "needle".into();

            // The same range is confirmed, even with a newer title on its lower layer.
            top.updated_at = "t2".into();
            detail.show(Some(top.clone()), None, cx);
            detail.show_refreshed_stack(&stack, cx);
            stack.layers[0].title = "New title".into();
            detail.show_refreshed_stack(&stack, cx);
            assert_eq!(detail.file_tree.search.query, "needle");

            // The selected PR stays at t2 while the stack's comparison base changes.
            stack.layers[0].base_sha = "new-base".into();
            detail.show_refreshed_stack(&stack, cx);
        });
        cx.run_until_parked();
        detail.update(cx, |detail, _| {
            assert_eq!(detail.shown_base_sha.as_deref(), Some("new-base"));
            assert_eq!(detail.shown_head_sha.as_deref(), Some("head-2"));
        });
    }

    #[gpui::test]
    fn automatic_refresh_keeps_the_composer_and_navigation_keeps_the_operation(
        cx: &mut gpui::TestAppContext,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let (detail, _) = cached_detail(cx, directory.path());
        detail.update(cx, |detail, cx| {
            detail.pull_request = Some(pull_request(1, "old"));
            let text = cx.new(|cx| {
                let mut area = TextArea::new("", cx);
                area.set_text("unsaved comment", cx);
                area
            });
            detail.review.editor = Some(CommentEditor {
                target: EditorTarget::Reply {
                    path: "a.rs".into(),
                    thread_id: "thread".into(),
                },
                _subscription: cx.subscribe(&text, |_, _, _: &TextAreaEvent, _| {}),
                suggestion_source: None,
                text: text.clone(),
            });
            detail.show(Some(pull_request(1, "new")), None, cx);
            assert_eq!(
                detail.review.editor.as_ref().unwrap().text.read(cx).text(),
                "unsaved comment"
            );
            assert_eq!(detail.deferred_reload.as_ref().unwrap().updated_at, "new");
            detail.review.editor = None;
            detail.resume_deferred_reload(cx);
            assert_eq!(detail.pull_request.as_ref().unwrap().updated_at, "new");

            let mut panel = ActionPanel::new(PanelAction::Close);
            panel.progress = OperationStatus::Working("still closing".into());
            panel.task = Some(cx.spawn(async |_, _| futures::future::pending::<()>().await));
            detail.set_action_panel(Some(panel));
            detail.show(Some(pull_request(2, "new")), None, cx);
            assert!(detail.action_panel().is_none());
            let background = detail.panel_for_operation(1).unwrap();
            assert!(background.task.is_some());
            detail.show(Some(pull_request(1, "new")), None, cx);
            assert!(detail.action_panel().is_some_and(ActionPanel::is_working));
        });
    }
}
