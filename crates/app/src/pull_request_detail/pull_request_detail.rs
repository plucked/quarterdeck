use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    Context, EventEmitter, ListAlignment, ListOffset, ListState, ScrollWheelEvent, Subscription,
    Task, Window, px,
};
use qc_cache::{DetailScrollPosition, DiffViewSettings, LineWrapping, ViewedFile};
use qc_diff::FileDiff;
use qc_github::PullRequest;
use qc_markdown::MarkdownDocument;
use qc_syntax::DiffSyntax;

use super::changed_files::ChangedFiles;
use super::detail_content::DetailContent;
use super::detail_event::DetailEvent;
use super::detail_row_list::DetailRowList;
use super::file_review_state::FileReviewState;
use super::loaded_file::LoadedFile;
use super::parsed_description::ParsedDescription;
use super::viewed_toggle::ViewedToggle;
use crate::app_services::AppServices;
use crate::app_settings::AppSettings;
use crate::ui_state::{UiStateRecorder, read_viewed_files};

/// How far beyond the visible area rows are laid out, so fast scrolling
/// rarely shows rows before they are drawn.
const LIST_OVERDRAW: f32 = 600.;
/// Batches when the number of cores is unknown.
const DEFAULT_HIGHLIGHT_BATCHES: usize = 4;
/// Code width that stays visible when a file is scrolled sideways to its end.
const MIN_VISIBLE_CODE_WIDTH: f32 = 240.;

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
    files: Vec<LoadedFile>,
    viewed_marks: Vec<ViewedFile>,
}

/// The syntax colors computed for one diff, matched back by the diff's `Arc`.
struct HighlightedDiff {
    diff: Arc<FileDiff>,
    syntax: Option<Arc<DiffSyntax>>,
}

/// The right pane: the shown pull request's header, description, diff
/// summary and changed files, as one virtualized list of rows. Only rows in
/// view are rendered, so its cost does not grow with the pull request.
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

        // Measure every row once, so the scrollbar knows the full height.
        let list = ListState::new(0, ListAlignment::Top, px(LIST_OVERDRAW)).measure_all();
        let detail = cx.entity().downgrade();
        list.set_scroll_handler(move |_, _, cx| {
            detail
                .update(cx, |_, cx| cx.emit(DetailEvent::Scrolled))
                .ok();
        });

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
            Some(ChangedFiles::Loading | ChangedFiles::Failed(_)) | None => &[],
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
            self.rebuild_rows(ScrollAnchor::Top);
            cx.notify();
            return;
        };

        // Decide by what was shown before.
        let shown = self.pull_request.as_ref();
        let is_same_version = shown.is_some_and(|shown| {
            shown.id == pull_request.id && shown.updated_at == pull_request.updated_at
        });
        let is_same_pull_request = shown.is_some_and(|shown| shown.id == pull_request.id);
        if is_same_version {
            self.pull_request = Some(pull_request);
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
        self.pull_request = Some(pull_request);
        self.horizontal_offsets.clear();
        self.load_changed_files(cx);
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
        self.rows = DetailRowList::build(&DetailContent {
            has_pull_request: self.pull_request.is_some(),
            has_description: self.description().is_some(),
            changed_files: self.changed_files.as_ref(),
            file_review: &self.file_review,
            settings: self.diff_settings,
        });
        self.list.reset(self.rows.len());

        let files_settled = !matches!(self.changed_files, Some(ChangedFiles::Loading));
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

    /// Rebuilds the rows of file `file` alone, keeping the scroll position
    /// and the measured heights of every other row.
    fn rebuild_file_rows(&mut self, file: usize) {
        let Some(loaded) = self.loaded_files().get(file) else {
            return;
        };
        let rows = DetailRowList::file_rows(
            file,
            loaded,
            self.file_review.state_of(loaded),
            self.diff_settings,
        );
        let new_length = rows.len();
        if let Some(old_range) = self.rows.replace_file(file, rows) {
            self.list.splice(old_range, new_length);
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
        let Some(pull_request_id) = self
            .pull_request
            .as_ref()
            .map(|pull_request| pull_request.id)
        else {
            return;
        };
        let Some(loaded) = self.loaded_files().get(file).cloned() else {
            return;
        };
        match self.file_review.toggle_viewed(&loaded) {
            ViewedToggle::Marked(mark) => {
                UiStateRecorder::record_file_viewed(cx, pull_request_id, mark);
            }
            ViewedToggle::Unmarked(filename) => {
                UiStateRecorder::record_file_unviewed(cx, pull_request_id, filename);
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

    /// Loads the shown pull request's changed files: from the cache when
    /// current, otherwise from GitHub, caching the result.
    fn load_changed_files(&mut self, cx: &mut Context<Self>) {
        let Some(pull_request) = self.pull_request.clone() else {
            return;
        };
        let cache = self.services.cache.clone();
        let client = self.services.github.clone();
        let ui_state = cx.global::<UiStateRecorder>().shared_store();
        let options = self.diff_settings.diff_options();

        self.changed_files = Some(ChangedFiles::Loading);
        self.file_review = FileReviewState::default();
        self.syntax_task = None;
        self.rebuild_rows(ScrollAnchor::Top);
        cx.notify();
        self.files_task = Some(cx.spawn(async move |detail, cx| {
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

            // Show the files at the pending position, then color them; a closed view drops it.
            detail
                .update(cx, |detail, cx| {
                    detail.changed_files = Some(match loaded {
                        Ok(fetched) => {
                            detail.file_review =
                                FileReviewState::for_loaded(&fetched.files, &fetched.viewed_marks);
                            ChangedFiles::Loaded(fetched.files)
                        }
                        Err(error) => ChangedFiles::Failed(format!("{error:#}").into()),
                    });
                    detail.rebuild_rows(ScrollAnchor::Top);
                    detail.highlight_syntax(cx);
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
        self.syntax_task = Some(cx.spawn(async move |detail, cx| {
            for batch in batches {
                let highlighted = batch.await;
                let applied = detail.update(cx, |detail, cx| {
                    detail.apply_syntax(highlighted);
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
