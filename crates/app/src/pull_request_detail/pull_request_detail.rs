use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui::{
    ClipboardItem, Context, EventEmitter, FocusHandle, ListAlignment, ListOffset, ListState,
    Pixels, Point, ScrollHandle, ScrollWheelEvent, SharedString, Subscription, Task, Window, px,
};
use qd_cache::{
    DetailScrollPosition, DiffViewSettings, LineWrapping, PanelVisibility, ReviewLayoutState,
    ReviewScope, StackReviewMode, ViewedFile,
};
use qd_diff::FileDiff;
use qd_file_tree::{FileKind, FileTree, NodeId};
use qd_github::{
    GitHubClient, PullRequest, PullRequestFile, PullRequestStack, RepoRef, StackMembership,
};
use qd_markdown::MarkdownDocument;
use qd_syntax::DiffSyntax;

use super::action_panel::ActionPanel;
use super::changed_files::ChangedFiles;
use super::changed_rows::ChangedRows;
use super::chip_click::ChipClick;
use super::detail_content::DetailContent;
use super::detail_event::{DetailEvent, LayerRequest};
use super::detail_row_list::DetailRowList;
use super::diff_selection::DiffSelection;
use super::fetch_changed_files::fetch_changed_files;
use super::fetch_combined_files::fetch_combined_files;
use super::file_review_state::FileReviewState;
use super::file_tree_state::FileTreeState;
use super::folder_viewed_state::FolderViewedState;
use super::load_progress::LoadProgress;
use super::loaded_file::LoadedFile;
use super::parsed_description::ParsedDescription;
use super::review_pane::ReviewPane;
use super::review_target::ReviewTarget;
use super::row_height_estimate::estimate_row_tops;
use super::selected_diff_text::{row_code_text, selected_diff_text};
use super::side_panel_tab::SidePanelTab;
use super::viewed_toggle::ViewedToggle;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::diff_view::{CodeCells, CodeSegments, DiffPosition, SideSelection};
use crate::fetch_stack::{cached_stack, refresh_stack};
use crate::scrollbar::ListRowTops;
use crate::shared_cache::SharedCache;
use crate::ui_state::{UiStateRecorder, read_review_notes, read_viewed_files};
use crate::word_range_at::word_range_at;

/// How far beyond the visible area rows are laid out, so fast scrolling
/// rarely shows rows before they are drawn.
const LIST_OVERDRAW: f32 = 600.;
/// Batches when the number of cores is unknown.
const DEFAULT_HIGHLIGHT_BATCHES: usize = 4;
/// Code width that stays visible when a file is scrolled sideways to its end.
const MIN_VISIBLE_CODE_WIDTH: f32 = 240.;
/// How a press selects, by click count.
const SINGLE_CLICK: usize = 1;
const DOUBLE_CLICK: usize = 2;
/// Dragging past the pane's edge scrolls by this share of the overshoot per mouse move.
const AUTOSCROLL_SHARE: f32 = 0.5;

/// A download is reported again only after this many more bytes, so the
/// pane redraws a few times a second rather than for every chunk.
const DOWNLOAD_REPORT_BYTES: u64 = 256 * 1024;

/// Which way to move between files or stack layers.
#[derive(Clone, Copy)]
enum FileDirection {
    Next,
    Previous,
}

/// Where the list scrolls after its rows are rebuilt.
#[derive(Clone, Copy)]
enum ScrollAnchor {
    Top,
    /// The header of this file, which was at the top before.
    File(usize),
}

/// A pull request's changed files as loaded on the background executor,
/// with the viewed marks saved for it.
struct FetchedChanges {
    /// In tree order: `tree`'s file indexes are positions here.
    files: Vec<LoadedFile>,
    tree: FileTree,
    viewed_marks: Vec<ViewedFile>,
    /// Whose marks those are, for marks set from now on.
    scope: ReviewScope,
    /// The user's review notes of that scope, and the GitHub review's summary.
    notes: Vec<qd_cache::ReviewNote>,
    summary: Option<qd_cache::ReviewSummary>,
}

/// The syntax colors computed for one diff, matched back by the diff's `Arc`.
struct HighlightedDiff {
    diff: Arc<FileDiff>,
    syntax: Option<Arc<DiffSyntax>>,
}

/// The right pane: a sticky header with the shown pull request's title (and
/// its description when expanded), the changed files as one virtualized
/// list of rows, and the file tree panel beside them. Only rows in view are
/// rendered, so its cost does not grow with the pull request. The tree and
/// the diff share one file order and one filter.
///
/// Files are read from the cache when current for the pull request's
/// `updated_at`, otherwise fetched and cached; diffs are built and colored
/// off the main thread. Emits [`DetailEvent::Scrolled`] when it scrolls.
pub struct PullRequestDetail {
    services: AppServices,
    pull_request: Option<PullRequest>,
    description: Option<ParsedDescription>,
    changed_files: Option<ChangedFiles>,
    /// How diffs show, as last applied from [`AppSettings`].
    diff_settings: DiffViewSettings,
    /// Which loaded files are viewed and collapsed.
    file_review: FileReviewState,
    rows: DetailRowList,
    list: ListState,
    /// How far each file's lines are scrolled sideways when lines do not wrap.
    horizontal_offsets: HashMap<usize, f32>,
    /// A scroll position to apply once the files are loaded (a restore after
    /// relaunch, or the position kept across a reload of the same pull request).
    pending_scroll: Option<DetailScrollPosition>,
    /// The in-flight file load; replacing it cancels the previous one.
    files_task: Option<Task<()>>,
    /// The rebuild of the loaded diffs after a settings change.
    diff_task: Option<Task<()>>,
    /// The syntax highlighting of the loaded diffs.
    syntax_task: Option<Task<()>>,
    /// Focused when code is pressed, so cmd-c copies the diff selection.
    focus: FocusHandle,
    /// The code texts painted this frame, for mouse hit testing.
    code_segments: CodeSegments,
    diff_selection: Option<DiffSelection>,
    /// Whether a press in the code is being dragged.
    is_selecting: bool,
    /// The file tree panel: the files as a tree, the filter, folders, visibility.
    file_tree: FileTreeState,
    /// Whether the sticky header shows the description; collapsed for each new pull request.
    is_description_expanded: bool,
    /// The scroll of the description inside the expanded header.
    description_scroll: ScrollHandle,
    /// Whether stacked pull requests show one layer or combined layers.
    review_mode: StackReviewMode,
    /// What the shown (or loading) files are of.
    target: ReviewTarget,
    /// Whose viewed marks the shown files use.
    viewed_scope: Option<ReviewScope>,
    /// The shown pull request's stack with its layers, once loaded.
    stack: Option<PullRequestStack>,
    /// Whether the stack is being read or checked with GitHub.
    is_stack_loading: bool,
    /// Why the stack could not be checked with GitHub, if it could not.
    stack_error: Option<SharedString>,
    /// The first layer of a combined review (a pull request number), when
    /// chosen; else the lowest unmerged layer.
    combined_start: Option<u64>,
    /// The stack being loaded; replacing it cancels the previous one.
    stack_task: Option<Task<()>>,
    /// Syntax batches colored of all started, while highlighting runs.
    highlight_progress: Option<(usize, usize)>,
    /// The side panel's width, as dragged (and saved).
    side_panel_width: f32,
    /// Where each row starts, estimated for the scrollbar (the list lays
    /// out only the rows on screen), and the pane width it was estimated
    /// for; `None` once the rows changed.
    row_tops: ListRowTops,
    row_tops_width: Option<f32>,
    /// Review comments, the comment being written, and the review tab.
    review: ReviewPane,
    /// The merge or close panel under the header, while open.
    action_panel: Option<ActionPanel>,
    /// Keeps the observer of [`AppSettings`] registered for the view's lifetime.
    _settings_subscription: Subscription,
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
                .update(cx, |_, cx| cx.emit(DetailEvent::Scrolled))
                .ok();
        });

        let layout = cx
            .global::<UiStateRecorder>()
            .saved_review_layout()
            .unwrap_or_default();
        let mut detail = Self {
            services,
            pull_request: None,
            description: None,
            changed_files: None,
            diff_settings,
            file_review: FileReviewState::default(),
            rows: DetailRowList::default(),
            list,
            horizontal_offsets: HashMap::new(),
            pending_scroll: None,
            files_task: None,
            diff_task: None,
            syntax_task: None,
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
            stack: None,
            is_stack_loading: false,
            stack_error: None,
            combined_start: None,
            stack_task: None,
            highlight_progress: None,
            row_tops: ListRowTops::from_heights([]),
            row_tops_width: None,
            action_panel: None,
            review: ReviewPane {
                destination: layout.comment_destination,
                ..ReviewPane::default()
            },
            side_panel_width: cx
                .global::<UiStateRecorder>()
                .saved_panel_widths()
                .unwrap_or_default()
                .side_panel,
            _settings_subscription: settings_subscription,
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

    /// Drops the diff selection (a double click opening a comment selected a word).
    pub(super) fn clear_diff_selection(&mut self) {
        self.diff_selection = None;
    }

    pub(super) fn code_segments(&self) -> &CodeSegments {
        &self.code_segments
    }

    pub(super) fn is_selecting(&self) -> bool {
        self.is_selecting
    }

    /// What row `row`'s code cells need: where to register and their selected part.
    pub(super) fn code_cells(&self, row: usize) -> CodeCells {
        CodeCells {
            row,
            segments: self.code_segments.clone(),
            selection: self.diff_selection.and_then(|selection| {
                Some(SideSelection {
                    side: selection.side,
                    range: selection.range_in_row(row)?,
                })
            }),
        }
    }

    /// Starts a selection in the code under `point` for a press with
    /// `click_count` clicks (one: drag; two: a word; three: the line). A
    /// press outside the code drops the selection and leaves focus alone,
    /// so the description keeps its own selection and copy.
    pub(super) fn begin_diff_selection(
        &mut self,
        point: Point<Pixels>,
        click_count: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((side, position)) = self.code_segments.hit(point) else {
            if self.diff_selection.take().is_some() {
                cx.notify();
            }
            return;
        };
        window.focus(&self.focus);

        // One click starts a drag; more select a word or the line at once.
        let text = self
            .rows
            .get(position.row)
            .and_then(|row| row_code_text(row, side, self.loaded_files()))
            .unwrap_or_default();
        let range = match click_count {
            0 | SINGLE_CLICK => position.byte..position.byte,
            DOUBLE_CLICK => word_range_at(text, position.byte),
            _ => 0..text.len(),
        };
        let at = |byte| DiffPosition {
            row: position.row,
            byte,
        };
        self.diff_selection = Some(DiffSelection {
            side,
            anchor: at(range.start),
            head: at(range.end),
        });
        self.is_selecting = click_count <= SINGLE_CLICK;
        cx.notify();
    }

    /// Moves the selection's head to the code nearest `point` in its column;
    /// past the pane's top or bottom edge, also scrolls that way.
    pub(super) fn extend_diff_selection(&mut self, point: Point<Pixels>, cx: &mut Context<Self>) {
        if !self.is_selecting {
            return;
        }
        let Some(selection) = self.diff_selection else {
            return;
        };

        // Scroll toward the pointer when it left the pane vertically.
        let viewport = self.list.viewport_bounds();
        if point.y < viewport.top() {
            self.list
                .scroll_by((point.y - viewport.top()) * AUTOSCROLL_SHARE);
        } else if point.y > viewport.bottom() {
            self.list
                .scroll_by((point.y - viewport.bottom()) * AUTOSCROLL_SHARE);
        }

        if let Some(head) = self.code_segments.nearest_on_side(selection.side, point)
            && head != selection.head
        {
            self.diff_selection = Some(DiffSelection { head, ..selection });
        }
        cx.notify();
    }

    /// Ends a drag; a press released where it started selected nothing.
    pub(super) fn finish_diff_selection(&mut self, cx: &mut Context<Self>) {
        if !self.is_selecting {
            return;
        }
        self.is_selecting = false;
        if self
            .diff_selection
            .is_some_and(|selection| selection.is_empty())
        {
            self.diff_selection = None;
        }
        cx.notify();
    }

    /// Copies the selected code to the clipboard (cmd-c), if any is selected.
    pub(super) fn copy_diff_selection(&self, cx: &mut Context<Self>) {
        let Some(selection) = self
            .diff_selection
            .filter(|selection| !selection.is_empty())
        else {
            return;
        };
        let text = selected_diff_text(&selection, &self.rows, self.loaded_files());
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    pub(super) fn pull_request(&self) -> Option<&PullRequest> {
        self.pull_request.as_ref()
    }

    pub(super) fn description(&self) -> Option<&Arc<MarkdownDocument>> {
        self.description
            .as_ref()
            .and_then(ParsedDescription::document)
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

    /// How far file `file`'s lines are scrolled sideways.
    pub(super) fn horizontal_offset(&self, file: usize) -> f32 {
        self.horizontal_offsets
            .get(&file)
            .copied()
            .unwrap_or_default()
    }

    /// The loaded files, or none while loading or after a failure.
    pub(super) fn loaded_files(&self) -> &[LoadedFile] {
        match &self.changed_files {
            Some(ChangedFiles::Loaded(files)) => files,
            Some(ChangedFiles::Loading(_) | ChangedFiles::Failed(_)) | None => &[],
        }
    }

    /// How far the files' load has come, while they load.
    pub(super) fn load_progress(&self) -> Option<LoadProgress> {
        match &self.changed_files {
            Some(ChangedFiles::Loading(progress)) => Some(*progress),
            Some(ChangedFiles::Loaded(_) | ChangedFiles::Failed(_)) | None => None,
        }
    }

    /// Syntax batches colored of all, while highlighting runs.
    pub(super) fn highlight_progress(&self) -> Option<(usize, usize)> {
        self.highlight_progress
    }

    pub(super) fn review_mode(&self) -> StackReviewMode {
        self.review_mode
    }

    pub(super) fn target(&self) -> &ReviewTarget {
        &self.target
    }

    pub(super) fn stack(&self) -> Option<&PullRequestStack> {
        self.stack.as_ref()
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
        self.is_stack_loading
    }

    pub(super) fn stack_error(&self) -> Option<&SharedString> {
        self.stack_error.as_ref()
    }

    /// The position of the shown pull request in its loaded stack.
    pub(super) fn current_layer_index(&self) -> Option<usize> {
        let number = self.pull_request.as_ref()?.number;
        self.stack
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

    /// Rebuilds every row, keeping the file at the top in view.
    pub(super) fn rebuild_rows_in_place(&mut self) {
        self.rebuild_rows(self.anchor_at_top());
    }

    /// Scrolls the diff so row `row` is at the top.
    pub(super) fn scroll_to_row(&mut self, row: usize) {
        self.list.scroll_to(ListOffset {
            item_ix: row,
            offset_in_item: px(0.),
        });
    }

    /// The rows' estimated tops for the scrollbar, estimated again when the
    /// rows changed or the pane's width did (wrapped lines depend on it).
    pub(super) fn current_row_tops(&mut self) -> ListRowTops {
        let width = f32::from(self.list.viewport_bounds().size.width);
        let is_current = self
            .row_tops_width
            .is_some_and(|estimated| (estimated - width).abs() < 1.);
        if !is_current {
            self.row_tops =
                estimate_row_tops(&self.rows, self.loaded_files(), self.diff_settings, width);
            self.row_tops_width = Some(width);
        }
        self.row_tops.clone()
    }

    /// Measures row `row` again (a growing comment editor).
    pub(super) fn remeasure_row(&mut self, row: usize) {
        self.list.splice(row..row + 1, 1);
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
            self.stack = None;
            self.stack_task = None;
            self.is_stack_loading = false;
            self.rebuild_rows(ScrollAnchor::Top);
            cx.notify();
            return;
        };
        let is_other_stack = !is_same_stack(self.stack.as_ref(), &pull_request);

        // Decide by what was shown before.
        let shown = self.pull_request.as_ref();
        let is_same_version = shown.is_some_and(|shown| {
            shown.id == pull_request.id && shown.updated_at == pull_request.updated_at
        });
        let is_same_pull_request = shown.is_some_and(|shown| shown.id == pull_request.id);
        if is_same_version {
            // The stack membership can arrive later than the pull request.
            let gained_stack = is_other_stack && pull_request.stack.is_some();
            self.pull_request = Some(pull_request);
            if gained_stack {
                self.load_stack(cx);
            }
            cx.notify();
            return;
        }
        self.pending_scroll = if is_same_pull_request {
            restore.or_else(|| Some(self.scroll_position()))
        } else {
            restore
        };

        // Show the header and description now; the files follow.
        let is_described = self
            .description
            .as_ref()
            .is_some_and(|description| description.is_of(&pull_request));
        if !is_described {
            self.description = Some(ParsedDescription::parse(&pull_request));
        }
        if !is_same_pull_request {
            self.action_panel = None;
            self.is_description_expanded = false;
            self.description_scroll
                .set_offset(gpui::point(px(0.), px(0.)));
        }
        self.pull_request = Some(pull_request);
        self.horizontal_offsets.clear();

        // Another stack's layers do not apply; the same stack's stay while they are checked.
        if is_other_stack {
            self.stack = None;
            self.combined_start = None;
        }
        // New commits can move the layers' heads, so the stack is checked again (free when unchanged).
        self.target = self.decide_target();
        cx.emit(DetailEvent::TargetChanged);
        self.load_changed_files(cx);
        self.load_stack(cx);
    }

    /// The target for the shown pull request, mode, stack and range start.
    fn decide_target(&self) -> ReviewTarget {
        self.pull_request
            .as_ref()
            .map_or(ReviewTarget::Layer, |pull_request| {
                ReviewTarget::decide(
                    self.review_mode,
                    pull_request,
                    self.stack.as_ref(),
                    self.combined_start,
                )
            })
    }

    /// Recomputes the target and reloads the files when it changed: after
    /// the stack arrived or changed, the mode flipped or the range moved.
    fn retarget(&mut self, cx: &mut Context<Self>) {
        let target = self.decide_target();
        if target == self.target {
            return;
        }
        self.target = target;
        self.pending_scroll = None;
        cx.emit(DetailEvent::TargetChanged);
        self.load_changed_files(cx);
    }

    /// Reads the shown pull request's stack from the cache, then checks it
    /// with GitHub (free when unchanged); each answer may change the target.
    fn load_stack(&mut self, cx: &mut Context<Self>) {
        let Some((repo, number)) = self.pull_request.as_ref().and_then(|pull_request| {
            Some((
                pull_request.repo.clone(),
                pull_request.stack.as_ref()?.number,
            ))
        }) else {
            self.stack_task = None;
            self.is_stack_loading = false;
            return;
        };
        let cache = self.services.cache.clone();
        let client = self.services.github.clone();
        self.is_stack_loading = true;
        self.stack_error = None;
        cx.notify();
        self.stack_task = Some(cx.spawn(async move |detail, cx| {
            // The cached layers first, so a combined review starts at once.
            let read_cache = cache.clone();
            let read_repo = repo.clone();
            let cached = cx
                .background_executor()
                .spawn(async move { cached_stack(&read_cache, &read_repo, number) })
                .await;
            let applied = detail.update(cx, |detail, cx| {
                if let Ok(Some(stack)) = cached {
                    detail.apply_stack(stack, cx);
                }
            });
            if applied.is_err() {
                return;
            }

            // Then GitHub's current layers.
            let refreshed = cx
                .background_executor()
                .spawn(async move { refresh_stack(client, &cache, &repo, number) })
                .await;
            detail
                .update(cx, |detail, cx| {
                    detail.is_stack_loading = false;
                    match refreshed {
                        Ok(stack) => detail.apply_stack(stack, cx),
                        Err(error) => {
                            detail.stack_error = Some(format!("{error:#}").into());
                            // Without layers a combined review cannot start; show the layer.
                            if detail.target == ReviewTarget::AwaitingStack {
                                detail.review_mode = StackReviewMode::Layer;
                                detail.retarget(cx);
                            }
                        }
                    }
                    cx.notify();
                })
                .ok();
        }));
    }

    /// Shows `stack`'s layers if it is still the shown pull request's stack.
    fn apply_stack(&mut self, stack: PullRequestStack, cx: &mut Context<Self>) {
        let belongs = self.pull_request.as_ref().is_some_and(|pull_request| {
            pull_request.repo == stack.repo
                && pull_request
                    .stack
                    .as_ref()
                    .is_some_and(|membership| membership.number == stack.number)
        });
        if !belongs || self.stack.as_ref() == Some(&stack) {
            return;
        }
        self.stack = Some(stack);
        self.retarget(cx);
        cx.notify();
    }

    /// Switches between one layer and combined layers (`c`), and saves it.
    pub fn toggle_review_mode(&mut self, cx: &mut Context<Self>) {
        let is_stacked = self
            .pull_request
            .as_ref()
            .is_some_and(|pull_request| pull_request.stack.is_some());
        if !is_stacked {
            return;
        }
        self.review_mode = self.review_mode.toggled();
        UiStateRecorder::record_review_layout(cx, self.review_layout());
        self.retarget(cx);
        cx.notify();
    }

    /// Reviews the shown stack combined from its lowest unmerged layer up to
    /// the shown one (a click on a stack in the sidebar), and saves the mode.
    pub fn review_combined_stack(&mut self, cx: &mut Context<Self>) {
        self.prepare_combined_stack_review(cx);
        self.retarget(cx);
        cx.notify();
    }

    /// Sets the combined review from the lowest unmerged layer without
    /// reloading, for a pull request about to be shown, and saves the mode.
    pub fn prepare_combined_stack_review(&mut self, cx: &mut Context<Self>) {
        self.combined_start = None;
        if self.review_mode != StackReviewMode::Combined {
            self.review_mode = StackReviewMode::Combined;
            UiStateRecorder::record_review_layout(cx, self.review_layout());
        }
    }

    /// Whether the files shown (or awaited) are a combined stack review.
    pub fn is_combined_review(&self) -> bool {
        match self.target {
            ReviewTarget::Combined(_) | ReviewTarget::AwaitingStack => true,
            ReviewTarget::Layer => false,
        }
    }

    /// Sets the review mode, from the header's control.
    pub(super) fn set_review_mode(&mut self, mode: StackReviewMode, cx: &mut Context<Self>) {
        if self.review_mode != mode {
            self.toggle_review_mode(cx);
        }
    }

    /// Starts the combined range at layer `number` of `repo` (a shift-click
    /// on a layer in the sidebar) when this stack is shown; `false` when it
    /// is another stack's layer, which the caller opens instead.
    pub fn start_combined_at(
        &mut self,
        repo: &RepoRef,
        number: u64,
        cx: &mut Context<Self>,
    ) -> bool {
        let index = self
            .stack
            .as_ref()
            .filter(|stack| stack.repo == *repo)
            .and_then(|stack| stack.layers.iter().position(|layer| layer.number == number));
        let Some(index) = index else {
            return false;
        };
        self.set_combined_start(index, cx);
        true
    }

    /// Whether a pull request is shown (the side panel can be toggled).
    pub fn has_pull_request(&self) -> bool {
        self.pull_request.is_some()
    }

    /// Whether the side panel (files and review) is set to show.
    pub fn is_side_panel_shown(&self) -> bool {
        self.file_tree.visibility() == PanelVisibility::Shown
    }

    /// Starts the combined range at layer `index`, switching to the combined
    /// review; a layer above the shown one is not a start.
    fn set_combined_start(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(number) = self
            .stack
            .as_ref()
            .and_then(|stack| stack.layers.get(index))
            .map(|layer| layer.number)
        else {
            return;
        };
        if self
            .current_layer_index()
            .is_none_or(|current| index > current)
        {
            return;
        }
        self.combined_start = Some(number);
        if self.review_mode == StackReviewMode::Layer {
            self.review_mode = StackReviewMode::Combined;
            UiStateRecorder::record_review_layout(cx, self.review_layout());
        }
        self.retarget(cx);
        cx.notify();
    }

    /// Asks the owner to show layer `index` of the stack.
    pub(super) fn open_layer(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(stack) = &self.stack else {
            return;
        };
        let Some(layer) = stack.layers.get(index) else {
            return;
        };
        let request = LayerRequest {
            repo: stack.repo.clone(),
            number: layer.number,
            membership: StackMembership {
                number: stack.number,
                position: u64::try_from(index + 1).unwrap_or(u64::MAX),
                size: u64::try_from(stack.layers.len()).unwrap_or(u64::MAX),
                base_ref: stack.base_ref.clone(),
            },
        };
        cx.emit(DetailEvent::OpenLayer(request));
    }

    /// Shows the layer above the shown one (`}`).
    pub fn open_next_layer(&mut self, cx: &mut Context<Self>) {
        self.open_neighbour_layer(FileDirection::Next, cx);
    }

    /// Shows the layer below the shown one (`{`).
    pub fn open_previous_layer(&mut self, cx: &mut Context<Self>) {
        self.open_neighbour_layer(FileDirection::Previous, cx);
    }

    /// Shows the layer above or below the shown one.
    fn open_neighbour_layer(&mut self, direction: FileDirection, cx: &mut Context<Self>) {
        let Some(current) = self.current_layer_index() else {
            return;
        };
        let target = match direction {
            FileDirection::Next => current + 1,
            FileDirection::Previous => match current.checked_sub(1) {
                Some(below) => below,
                None => return,
            },
        };
        self.open_layer(target, cx);
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

    /// Rebuilds every row and resets the list (which forgets row heights),
    /// then scrolls to the pending position once the files are in, or else
    /// to `anchor`.
    fn rebuild_rows(&mut self, anchor: ScrollAnchor) {
        // Row numbers change, so a selection would point at other text.
        self.diff_selection = None;
        self.refresh_comment_layout();
        let shown_files = self
            .file_tree
            .shown_files(self.loaded_files(), &self.file_review);
        self.rows = DetailRowList::build(&DetailContent {
            has_pull_request: self.pull_request.is_some(),
            changed_files: self.changed_files.as_ref(),
            shown_files: &shown_files,
            file_review: &self.file_review,
            settings: self.diff_settings,
            comments: &self.review.placement,
        });
        self.list.reset(self.rows.len());
        self.row_tops_width = None;
        self.refresh_tree_rows();

        let files_settled = !matches!(self.changed_files, Some(ChangedFiles::Loading(_)));
        let pending = if files_settled {
            self.pending_scroll.take()
        } else {
            None
        };
        let target = match (pending, anchor) {
            (Some(position), _) => Some(ListOffset {
                item_ix: position.row.min(self.rows.len().saturating_sub(1)),
                offset_in_item: px(position.offset_in_row),
            }),
            (None, ScrollAnchor::File(file)) => self.rows.file_start(file).map(|row| ListOffset {
                item_ix: row,
                offset_in_item: px(0.),
            }),
            (None, ScrollAnchor::Top) => None,
        };
        if let Some(target) = target {
            self.list.scroll_to(target);
        }
    }

    /// The file whose rows are at the top of the pane, to keep in view
    /// across a rebuild; the top when above the files.
    fn anchor_at_top(&self) -> ScrollAnchor {
        let top_row = self.list.logical_scroll_top().item_ix;
        self.rows
            .file_at_row(top_row)
            .map_or(ScrollAnchor::Top, ScrollAnchor::File)
    }

    /// Rebuilds the rows of file `file` alone (none when the filter now hides
    /// it), keeping the scroll position and the measured heights of every
    /// other row.
    pub(super) fn rebuild_file_rows(&mut self, file: usize) {
        let Some(loaded) = self.loaded_files().get(file) else {
            return;
        };
        let is_shown = self.file_tree.shows(loaded, &self.file_review);
        let rows = if is_shown {
            DetailRowList::file_rows(
                file,
                loaded,
                self.file_review.state_of(loaded),
                self.diff_settings,
                &self.review.placement,
            )
        } else {
            Vec::new()
        };
        // Splice only the rows that changed, so the scroll position stays.
        let old_rows = self
            .rows
            .file_range(file)
            .map(|range| self.rows.rows_in(range).to_vec())
            .unwrap_or_default();
        let change = ChangedRows::between(&old_rows, &rows);
        let top = self.list.logical_scroll_top();
        self.diff_selection = None;
        let new_rows = rows.clone();
        self.row_tops_width = None;
        if let Some(old_range) = self.rows.replace_file(file, rows) {
            let start = old_range.start + change.start;
            if change.old_len > 0 || change.new_len > 0 {
                self.list
                    .splice(start..start + change.old_len, change.new_len);

                // Keep the row that was at the top there, wherever it moved.
                let old_end = start + change.old_len;
                let kept_top = match (top.item_ix < start, top.item_ix >= old_end) {
                    (true, _) => Some(top),
                    (false, true) => Some(ListOffset {
                        item_ix: top.item_ix - change.old_len + change.new_len,
                        ..top
                    }),
                    (false, false) => old_rows
                        .get(top.item_ix - old_range.start)
                        .and_then(|row| {
                            new_rows[change.start..change.start + change.new_len]
                                .iter()
                                .position(|candidate| candidate == row)
                        })
                        .map(|position| ListOffset {
                            item_ix: start + position,
                            ..top
                        }),
                };
                if let Some(kept_top) = kept_top {
                    self.list.scroll_to(kept_top);
                }
            }
        }
        self.refresh_tree_rows();
    }

    /// Recomputes the tree's visible rows from the files, filter and marks.
    fn refresh_tree_rows(&mut self) {
        let files = match &self.changed_files {
            Some(ChangedFiles::Loaded(files)) => files.as_slice(),
            Some(ChangedFiles::Loading(_) | ChangedFiles::Failed(_)) | None => &[],
        };
        self.file_tree.refresh_rows(files, &self.file_review);
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

    /// Shows or hides the file tree panel (cmd-b), and saves it.
    pub fn toggle_file_tree(&mut self, cx: &mut Context<Self>) {
        self.file_tree.toggle_visibility();
        UiStateRecorder::record_review_layout(cx, self.review_layout());
        cx.notify();
    }

    /// Hides viewed files or shows them again (the switch under the
    /// progress bar); the tree and the diff follow, and it is saved.
    pub(super) fn toggle_hide_viewed(&mut self, cx: &mut Context<Self>) {
        self.file_tree.toggle_hide_viewed();
        UiStateRecorder::record_review_layout(cx, self.review_layout());
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }

    /// Picks or hides a kind chip; hidden kinds are saved.
    pub(super) fn click_kind(&mut self, kind: FileKind, click: ChipClick, cx: &mut Context<Self>) {
        self.file_tree.click_kind(kind, click);
        UiStateRecorder::record_review_layout(cx, self.review_layout());
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }

    /// Picks or hides a file type chip, for this pull request.
    pub(super) fn click_file_type(
        &mut self,
        file_type: &str,
        click: ChipClick,
        cx: &mut Context<Self>,
    ) {
        self.file_tree.click_file_type(file_type, click);
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }

    /// Clears both chip rows (the summary's Reset), and saves that no kind is hidden.
    pub(super) fn clear_filter_chips(&mut self, cx: &mut Context<Self>) {
        self.file_tree.clear_chips();
        UiStateRecorder::record_review_layout(cx, self.review_layout());
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }

    /// Shows every type chip, or only the most common again.
    pub(super) fn toggle_types_expanded(&mut self, cx: &mut Context<Self>) {
        self.file_tree.toggle_types_expanded();
        cx.notify();
    }

    /// Collapses or expands a folder of the tree.
    pub(super) fn toggle_folder(&mut self, path: &str, cx: &mut Context<Self>) {
        self.file_tree.toggle_folder(path);
        self.refresh_tree_rows();
        cx.notify();
    }

    /// Marks every file a folder covers as viewed, or, when all already
    /// are, as not viewed; saves the marks.
    pub(super) fn toggle_folder_viewed(&mut self, folder: NodeId, cx: &mut Context<Self>) {
        let Some(scope) = self.viewed_scope.clone() else {
            return;
        };
        let files = self.loaded_files().to_vec();
        let mark_viewed = self
            .file_tree
            .folder_viewed_state(folder, &files, &self.file_review)
            != FolderViewedState::All;
        for file in self.file_tree.folder_files(folder, &files) {
            match self.file_review.set_viewed(&files[file], mark_viewed) {
                Some(ViewedToggle::Marked(mark)) => {
                    UiStateRecorder::record_file_viewed(cx, scope.clone(), mark);
                }
                Some(ViewedToggle::Unmarked(filename)) => {
                    UiStateRecorder::record_file_unviewed(cx, scope.clone(), filename);
                }
                None => {}
            }
        }
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }

    /// The file whose rows are at the top of the diff, highlighted in the tree.
    pub(super) fn current_file(&self) -> Option<usize> {
        self.rows
            .file_at_row(self.list.logical_scroll_top().item_ix)
    }

    /// Scrolls the diff to file `file`'s header; nothing while it is hidden.
    pub(super) fn reveal_file(&mut self, file: usize, cx: &mut Context<Self>) {
        if let Some(row) = self.rows.file_start(file) {
            self.list.scroll_to(ListOffset {
                item_ix: row,
                offset_in_item: px(0.),
            });
            cx.notify();
        }
    }

    /// The shown file after (or before) the current one, in diff order.
    fn neighbour_file(&self, direction: FileDirection) -> Option<usize> {
        let shown: Vec<usize> = (0..self.loaded_files().len())
            .filter(|file| self.rows.file_start(*file).is_some())
            .collect();
        let current = self.current_file();
        match direction {
            FileDirection::Next => shown
                .iter()
                .copied()
                .find(|file| current.is_none_or(|current| *file > current)),
            FileDirection::Previous => shown
                .iter()
                .rev()
                .copied()
                .find(|file| current.is_some_and(|current| *file < current)),
        }
    }

    /// Scrolls to the next shown file (`]`).
    pub fn reveal_next_file(&mut self, cx: &mut Context<Self>) {
        if let Some(file) = self.neighbour_file(FileDirection::Next) {
            self.reveal_file(file, cx);
        }
    }

    /// Scrolls to the previous shown file (`[`).
    pub fn reveal_previous_file(&mut self, cx: &mut Context<Self>) {
        if let Some(file) = self.neighbour_file(FileDirection::Previous) {
            self.reveal_file(file, cx);
        }
    }

    /// Marks the current file viewed and moves on to the next file not yet
    /// viewed (`v`); on a viewed file, only unmarks it.
    pub fn mark_current_viewed_and_advance(&mut self, cx: &mut Context<Self>) {
        let Some(file) = self.current_file() else {
            return;
        };
        let was_viewed = self
            .loaded_files()
            .get(file)
            .is_some_and(|loaded| self.file_review.is_viewed(loaded.filename()));
        self.toggle_file_viewed(file, cx);
        if was_viewed {
            return;
        }
        let next_unviewed = (file + 1..self.loaded_files().len()).find(|next| {
            self.rows.file_start(*next).is_some()
                && !self
                    .file_review
                    .is_viewed(self.loaded_files()[*next].filename())
        });
        if let Some(next) = next_unviewed {
            self.reveal_file(next, cx);
        }
    }

    /// Applies diff settings changed in [`AppSettings`]. Layout and wrapping
    /// rebuild the rows (heights change); whitespace and highlight rebuild
    /// the diffs off the main thread first.
    fn apply_diff_settings(&mut self, settings: DiffViewSettings, cx: &mut Context<Self>) {
        let previous = self.diff_settings;
        self.diff_settings = settings;
        if previous.diff_options() == settings.diff_options() {
            if previous != settings {
                self.rebuild_rows(self.anchor_at_top());
            }
            cx.notify();
            return;
        }

        let files = self.loaded_files().to_vec();
        let options = settings.diff_options();
        self.diff_task = Some(cx.spawn(async move |detail, cx| {
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
            detail
                .update(cx, |detail, cx| {
                    let is_same_files = detail.loaded_files().len() == rebuilt.len()
                        && detail
                            .loaded_files()
                            .iter()
                            .zip(&rebuilt)
                            .all(|(shown, built)| shown.fingerprint == built.fingerprint);
                    if is_same_files {
                        let anchor = detail.anchor_at_top();
                        detail.changed_files = Some(ChangedFiles::Loaded(rebuilt));
                        detail.rebuild_rows(anchor);
                        detail.highlight_syntax(cx);
                        cx.notify();
                    }
                })
                .ok();
        }));
    }

    /// Collapses or expands file `file`.
    pub(super) fn toggle_file_collapsed(&mut self, file: usize, cx: &mut Context<Self>) {
        let Some(filename) = self
            .loaded_files()
            .get(file)
            .map(|loaded| loaded.filename().to_owned())
        else {
            return;
        };
        self.file_review.toggle_collapsed(&filename);
        self.rebuild_file_rows(file);
        cx.notify();
    }

    /// Marks file `file` viewed or not, and saves the mark.
    pub(super) fn toggle_file_viewed(&mut self, file: usize, cx: &mut Context<Self>) {
        let Some(scope) = self.viewed_scope.clone() else {
            return;
        };
        let Some(loaded) = self.loaded_files().get(file).cloned() else {
            return;
        };
        match self.file_review.toggle_viewed(&loaded) {
            ViewedToggle::Marked(mark) => {
                UiStateRecorder::record_file_viewed(cx, scope, mark);
            }
            ViewedToggle::Unmarked(filename) => {
                UiStateRecorder::record_file_unviewed(cx, scope, filename);
            }
        }
        self.rebuild_file_rows(file);
        cx.notify();
    }

    pub(super) fn collapse_all_files(&mut self, cx: &mut Context<Self>) {
        let files = self.loaded_files().to_vec();
        self.file_review.collapse_all(&files);
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }

    pub(super) fn expand_all_files(&mut self, cx: &mut Context<Self>) {
        self.file_review.expand_all();
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }

    /// Scrolls file `file`'s lines sideways on a mostly horizontal wheel or
    /// trackpad gesture over them, when lines do not wrap. The vertical
    /// list scroll does not see a gesture handled here.
    pub(super) fn scroll_file_horizontally(
        &mut self,
        file: usize,
        event: &ScrollWheelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.diff_settings.wrapping != LineWrapping::Scroll {
            return;
        }
        let delta = event.delta.pixel_delta(window.line_height());
        let horizontal = f32::from(delta.x);
        if horizontal.abs() <= f32::from(delta.y).abs() {
            return;
        }
        let Some(loaded) = self.loaded_files().get(file) else {
            return;
        };
        let max_offset = (loaded.longest_line_width - MIN_VISIBLE_CODE_WIDTH).max(0.);
        let offset = (self.horizontal_offset(file) - horizontal).clamp(0., max_offset);
        self.horizontal_offsets.insert(file, offset);
        cx.stop_propagation();
        cx.notify();
    }

    /// Loads the files of the review target: the pull request's (from the
    /// cache when current, otherwise from GitHub) or a combined range's (from
    /// the cache, or one diff download), caching what GitHub sent. Shows the
    /// progress while it runs; a combined review still waiting for the stack
    /// only shows that.
    fn load_changed_files(&mut self, cx: &mut Context<Self>) {
        let Some(pull_request) = self.pull_request.clone() else {
            return;
        };
        let target = self.target.clone();
        let scope = target.viewed_scope(&pull_request);
        let cache = self.services.cache.clone();
        let client = self.services.github.clone();
        let ui_state = cx.global::<UiStateRecorder>().shared_store();
        let options = self.diff_settings.diff_options();

        // Clear what was shown; a combined review without its layers only waits.
        let first_step = match target {
            ReviewTarget::AwaitingStack => LoadProgress::LoadingStack,
            ReviewTarget::Layer | ReviewTarget::Combined(_) => LoadProgress::ReadingCache,
        };
        self.changed_files = Some(ChangedFiles::Loading(first_step));
        self.file_review = FileReviewState::default();
        self.viewed_scope = None;
        self.reset_review();
        self.syntax_task = None;
        self.highlight_progress = None;
        self.rebuild_rows(ScrollAnchor::Top);
        cx.notify();
        if target == ReviewTarget::AwaitingStack {
            self.files_task = None;
            return;
        }

        self.files_task = Some(cx.spawn(async move |detail, cx| {
            // Load on the background executor, reporting progress through a channel.
            let (progress_sender, mut progress) = mpsc::unbounded();
            let load = cx.background_executor().spawn(async move {
                let report = |step| {
                    progress_sender.unbounded_send(step).ok();
                };
                let files = fetch_target_files(client, &cache, &pull_request, &target, &report)?;

                // Build every diff here too: word comparisons can take a moment on big files.
                report(LoadProgress::BuildingDiffs { files: files.len() });
                let built: Vec<LoadedFile> = files
                    .into_iter()
                    .map(|file| LoadedFile::build(file, options))
                    .collect();

                // Put the files in tree order, so the tree and the diff agree.
                let paths: Vec<&str> = built.iter().map(LoadedFile::filename).collect();
                let ordered: Vec<LoadedFile> = FileTree::build(&paths)
                    .file_order()
                    .into_iter()
                    .map(|index| built[index].clone())
                    .collect();
                let ordered_paths: Vec<&str> = ordered.iter().map(LoadedFile::filename).collect();
                let tree = FileTree::build(&ordered_paths);
                let notes_and_summary = read_review_notes(&ui_state, &scope);
                anyhow::Ok(FetchedChanges {
                    files: ordered,
                    tree,
                    viewed_marks: read_viewed_files(&ui_state, &scope),
                    notes: notes_and_summary.0,
                    summary: notes_and_summary.1,
                    scope,
                })
            });

            // Show each step as it is reported; the channel ends with the load.
            while let Some(step) = progress.next().await {
                let shown = detail.update(cx, |detail, cx| {
                    if let Some(ChangedFiles::Loading(current)) = &mut detail.changed_files {
                        *current = step;
                        cx.notify();
                    }
                });
                if shown.is_err() {
                    return;
                }
            }
            let loaded = load.await;

            // Show the files at the pending position, then color them; a closed view drops it.
            detail
                .update(cx, |detail, cx| {
                    detail.changed_files = Some(match loaded {
                        Ok(fetched) => {
                            detail.file_review =
                                FileReviewState::for_loaded(&fetched.files, &fetched.viewed_marks);
                            detail.viewed_scope = Some(fetched.scope);
                            detail.review.comments.notes = fetched.notes;
                            detail.review.comments.summary = fetched.summary.unwrap_or_default();
                            if detail.review.tab == SidePanelTab::Review {
                                detail.show_side_panel_tab(SidePanelTab::Review, cx);
                            }
                            detail.file_tree.reset_for(fetched.tree);
                            ChangedFiles::Loaded(fetched.files)
                        }
                        Err(error) => ChangedFiles::Failed(format!("{error:#}").into()),
                    });
                    detail.rebuild_rows(ScrollAnchor::Top);
                    detail.highlight_syntax(cx);
                    detail.load_github_review(cx);
                    cx.notify();
                })
                .ok();
        }));
    }

    /// Colors the loaded diffs by language off the main thread, one batch of
    /// files per core, showing each batch's colors as it finishes. Files show
    /// plain until then, and colors change no row height, so rows stay.
    fn highlight_syntax(&mut self, cx: &mut Context<Self>) {
        let files = self.loaded_files().to_vec();
        if files.is_empty() {
            self.syntax_task = None;
            self.highlight_progress = None;
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

        // Show batches in order, counting them for the indicator; dropping this task cancels the rest.
        let batch_count = batches.len();
        self.highlight_progress = Some((0, batch_count));
        self.syntax_task = Some(cx.spawn(async move |detail, cx| {
            for (index, batch) in batches.into_iter().enumerate() {
                let highlighted = batch.await;
                let applied = detail.update(cx, |detail, cx| {
                    detail.apply_syntax(highlighted);
                    let done = index + 1;
                    detail.highlight_progress = (done < batch_count).then_some((done, batch_count));
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
}

/// Whether `pull_request` belongs to `stack` (the stack shown so far); no
/// stack and no membership count as the same.
fn is_same_stack(stack: Option<&PullRequestStack>, pull_request: &PullRequest) -> bool {
    match (stack, &pull_request.stack) {
        (Some(stack), Some(membership)) => {
            stack.repo == pull_request.repo && stack.number == membership.number
        }
        (None, None) => true,
        (Some(_), None) | (None, Some(_)) => false,
    }
}

/// Fetches the files of `target` for `pull_request`, reporting each step;
/// downloads are reported in steps of [`DOWNLOAD_REPORT_BYTES`].
///
/// Blocking: runs on the background executor.
fn fetch_target_files(
    client: Result<GitHubClient, String>,
    cache: &SharedCache,
    pull_request: &PullRequest,
    target: &ReviewTarget,
    report: &dyn Fn(LoadProgress),
) -> anyhow::Result<Vec<PullRequestFile>> {
    match target {
        ReviewTarget::Layer | ReviewTarget::AwaitingStack => {
            fetch_changed_files(client, cache, pull_request, &mut |step| report(step))
        }
        ReviewTarget::Combined(range) => {
            let mut reported_bytes = 0;
            fetch_combined_files(client, cache, range, &mut |download| {
                let is_done = download.total_bytes == Some(download.received_bytes);
                if is_done || download.received_bytes >= reported_bytes + DOWNLOAD_REPORT_BYTES {
                    reported_bytes = download.received_bytes;
                    report(LoadProgress::DownloadingDiff(download));
                }
            })
        }
    }
}
