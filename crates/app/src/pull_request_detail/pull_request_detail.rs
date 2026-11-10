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
use super::file_review_state::FileReviewState;
use super::file_tree_state::FileTreeState;
use super::load_progress::LoadProgress;
use super::loaded_file::LoadedFile;
use super::parsed_description::ParsedDescription;
use super::review_pane::ReviewPane;
use super::review_target::ReviewTarget;
use super::scroll_anchor::ScrollAnchor;
use super::show_change::ShowChange;
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

/// The right pane: a sticky header with the shown pull request's title (and
/// its description when expanded), the changed files as one virtualized
/// list of rows (or the AI-written brief in their place), and the side
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
    description: Option<ParsedDescription>,
    pub(super) changed_files: Option<ChangedFiles>,
    /// How diffs show, as last applied from [`AppSettings`].
    pub(super) diff_settings: DiffViewSettings,
    /// Which loaded files are viewed and collapsed.
    pub(super) file_review: FileReviewState,
    pub(super) rows: DetailRowList,
    pub(super) list: ListState,
    /// The file the reader last jumped to (a click in the tree, `]` / `[`).
    /// It is the current file while its header is in view, so a jump the
    /// diff cannot scroll for (a file in the last screenful) still shows in
    /// the tree and moves `]` on; scrolling by hand or a rebuild drops it.
    pub(super) jumped_to_file: Option<usize>,
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
    /// Whether a press in the code is being dragged.
    pub(super) is_selecting: bool,
    /// The file tree panel: the files as a tree, the filter, folders, visibility.
    pub(super) file_tree: FileTreeState,
    /// Whether the sticky header shows the description; collapsed for each new pull request.
    is_description_expanded: bool,
    /// The scroll of the description inside the expanded header.
    description_scroll: ScrollHandle,
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
    /// The AI-written brief: diff or brief mode, briefs written, the run in flight.
    pub(super) brief: BriefPane,
    /// The head commit of the shown files, when known: what a brief's
    /// version is compared with.
    pub(super) shown_head_sha: Option<String>,
    /// The side panel's Checks tab, following the shown pull request.
    pub(super) checks: Entity<ChecksPanel>,
    /// Keep the observers of [`AppSettings`] and the avatars (header and
    /// comment authors) registered for the view's lifetime.
    _subscriptions: [Subscription; 2],
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
                    detail.jumped_to_file = None;
                    cx.emit(DetailEvent::Scrolled);
                })
                .ok();
        });

        let layout = cx
            .global::<UiStateRecorder>()
            .saved_review_layout()
            .unwrap_or_default();
        let checks = cx.new(|_| ChecksPanel::new(services.clone()));
        let mut detail = Self {
            services,
            pull_request: None,
            description: None,
            changed_files: None,
            diff_settings,
            file_review: FileReviewState::default(),
            rows: DetailRowList::default(),
            list,
            jumped_to_file: None,
            horizontal_offsets: HashMap::new(),
            pending_scroll: None,
            files_task: None,
            diff_task: None,
            syntax_task: None,
            context_loads: ContextLoads::default(),
            focus: cx.focus_handle(),
            code_segments: CodeSegments::default(),
            diff_selection: None,
            is_selecting: false,
            file_tree: FileTreeState::new(layout.clone()),
            is_description_expanded: false,
            description_scroll: ScrollHandle::new(),
            review_mode: layout.stack_review,
            target: ReviewTarget::Layer,
            viewed_scope: None,
            stack: StackState::default(),
            highlight_progress: None,
            row_tops: ListRowTops::from_heights([]),
            row_tops_width: None,
            action_panel: None,
            brief: BriefPane::new(detail_brief_handlers(cx)),
            shown_head_sha: None,
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
        detail.rebuild_rows(ScrollAnchor::Top);
        detail
    }

    pub(super) fn list(&self) -> &ListState {
        &self.list
    }

    pub(super) fn rows(&self) -> &DetailRowList {
        &self.rows
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
            stack_review: self.review_mode,
            comment_destination: self.review.destination,
            ..self.file_tree.layout()
        }
    }

    /// Shows `pull_request`, or nothing. The same version shown again changes
    /// nothing; a new version of the same pull request reloads its files and
    /// keeps the scroll position; another pull request starts at the top, or
    /// at `restore` (a position saved before a relaunch) once its files load.
    pub fn show(
        &mut self,
        pull_request: Option<PullRequest>,
        restore: Option<DetailScrollPosition>,
        cx: &mut Context<Self>,
    ) {
        let Some(pull_request) = pull_request else {
            self.pull_request = None;
            self.description = None;
            self.changed_files = None;
            self.files_task = None;
            self.pending_scroll = None;
            // A new stack state also cancels the stack's load.
            self.stack = StackState::default();
            self.rebuild_rows(ScrollAnchor::Top);
            self.follow_checks(cx);
            cx.notify();
            return;
        };
        // The header shows the author's avatar.
        AvatarImages::load(cx, [pull_request.author.avatar_url.clone()]);

        // Decide by what was shown before, then carry it out.
        let change = ShowChange::decide(
            self.pull_request.as_ref(),
            self.stack.loaded.as_ref(),
            &pull_request,
        );
        match change {
            ShowChange::SameVersion { loads_stack } => {
                // The stack membership can arrive later than the pull request.
                self.pull_request = Some(pull_request);
                if loads_stack {
                    self.load_stack(cx);
                }
                self.follow_checks(cx);
                cx.notify();
            }
            ShowChange::NewVersion { is_other_stack } => {
                // A reload keeps the reader's place.
                self.pending_scroll = restore.or_else(|| Some(self.scroll_position()));
                if is_other_stack {
                    self.stack.drop_layers();
                }
                self.show_new_version(pull_request, cx);
            }
            ShowChange::OtherPullRequest { is_other_stack } => {
                // Another pull request starts with no panel open and its description folded.
                self.pending_scroll = restore;
                self.action_panel = None;
                self.is_description_expanded = false;
                self.description_scroll
                    .set_offset(gpui::point(px(0.), px(0.)));
                if is_other_stack {
                    self.stack.drop_layers();
                }
                self.show_new_version(pull_request, cx);
            }
        }
    }

    /// Shows a version of `pull_request` not shown before: the header and
    /// description now, then its files and stack.
    fn show_new_version(&mut self, pull_request: PullRequest, cx: &mut Context<Self>) {
        // Show the header and description now; the files follow.
        let is_described = self
            .description
            .as_ref()
            .is_some_and(|description| description.is_of(&pull_request));
        if !is_described {
            self.description = Some(ParsedDescription::parse(&pull_request));
        }
        self.pull_request = Some(pull_request);
        self.horizontal_offsets.clear();

        // New commits can move the layers' heads, so the stack is checked again (free when unchanged).
        self.target = self.decide_target();
        cx.emit(DetailEvent::TargetChanged);
        self.load_changed_files(cx);
        self.load_stack(cx);
        self.follow_checks(cx);
    }

    /// The side panel's Checks tab.
    pub(super) fn checks(&self) -> &Entity<ChecksPanel> {
        &self.checks
    }

    /// Whether a pull request is shown (the side panel can be toggled).
    pub fn has_pull_request(&self) -> bool {
        self.pull_request.is_some()
    }

    /// Whether the side panel (files and review) is set to show.
    pub fn is_side_panel_shown(&self) -> bool {
        self.file_tree.visibility() == PanelVisibility::Shown
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

    pub(super) fn is_description_expanded(&self) -> bool {
        self.is_description_expanded
    }

    pub(super) fn description_scroll(&self) -> &ScrollHandle {
        &self.description_scroll
    }

    pub(super) fn toggle_description(&mut self, cx: &mut Context<Self>) {
        self.is_description_expanded = !self.is_description_expanded;
        cx.notify();
    }
}
