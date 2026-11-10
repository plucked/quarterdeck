use gpui::{Context, ListOffset, ScrollWheelEvent, Window, px};
use qd_cache::LineWrapping;

use super::detail_mode::DetailMode;
use super::detail_row::DetailRow;
use super::file_direction::FileDirection;
use super::file_lookup::file_index_of;
use super::pull_request_detail::PullRequestDetail;
use crate::diff_line_sides::{line_number_on, line_side_of};
use qd_studio::{CodeAnchor, CodeSide};

/// Code width that stays visible when a file is scrolled sideways to its end.
const MIN_VISIBLE_CODE_WIDTH: f32 = 240.;

impl PullRequestDetail {
    /// The file the reader is on, highlighted in the tree: the one last
    /// jumped to while its header is in view, else the one whose rows are at
    /// the top of the diff.
    pub(super) fn current_file(&self) -> Option<usize> {
        let jumped = self
            .jumped_to_file
            .filter(|file| self.is_file_header_in_view(*file));
        jumped.or_else(|| {
            self.rows
                .file_at_row(self.list.logical_scroll_top().item_ix)
        })
    }

    /// Whether file `file`'s header row lay wholly inside the diff's
    /// viewport in the last frame.
    fn is_file_header_in_view(&self, file: usize) -> bool {
        let Some(row) = self.rows.file_start(file) else {
            return false;
        };
        let viewport = self.list.viewport_bounds();
        self.list.bounds_for_item(row).is_some_and(|header| {
            header.top() >= viewport.top() && header.bottom() <= viewport.bottom()
        })
    }

    /// Scrolls the diff to file `file`'s header (as far as the diff goes)
    /// and makes it the current file; nothing while it is hidden.
    pub(super) fn reveal_file(&mut self, file: usize, cx: &mut Context<Self>) {
        if let Some(row) = self.rows.file_start(file) {
            self.list.scroll_to(ListOffset {
                item_ix: row,
                offset_in_item: px(0.),
            });
            self.jumped_to_file = Some(file);
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
        self.file_review.toggle_viewed(&loaded).record(scope, cx);
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

    /// Shows file `file`'s header in the diff (a file clicked in the file
    /// tree), switching from the brief to the diff first.
    pub(super) fn show_file_in_diff(&mut self, file: usize, cx: &mut Context<Self>) {
        if self.brief().mode == DetailMode::Brief {
            self.set_detail_mode(DetailMode::Diff, cx);
        }
        self.reveal_file(file, cx);
    }

    /// Switches to the diff with `anchor`'s first line at the top: the file
    /// is expanded when collapsed; when the line has no row (a filter hides
    /// the file) the diff shows the file's header, or stays where it was.
    pub(super) fn show_anchor_in_diff(&mut self, anchor: &CodeAnchor, cx: &mut Context<Self>) {
        self.set_detail_mode(DetailMode::Diff, cx);
        let Some(file) = file_index_of(self.loaded_files(), &anchor.path) else {
            return;
        };
        // Expand the file, so its lines have rows.
        let loaded = &self.loaded_files()[file];
        if self.file_review().state_of(loaded).is_collapsed {
            self.toggle_file_collapsed(file, cx);
        }
        match self.row_of_line(file, anchor.side, anchor.first_line) {
            Some(row) => {
                self.scroll_to_row(row);
                cx.notify();
            }
            None => self.reveal_file(file, cx),
        }
    }

    /// The diff row showing line `number` on `side` of file `file`.
    fn row_of_line(&self, file: usize, side: CodeSide, number: u32) -> Option<usize> {
        let range = self.rows().file_range(file)?;
        let diff = &self.loaded_files().get(file)?.diff;
        let side = line_side_of(side);
        let number_at = |hunk: usize, line: usize| {
            line_number_on(diff.hunks().get(hunk)?.lines.get(line)?, side)
        };
        let start = range.start;
        self.rows()
            .rows_in(range)
            .iter()
            .position(|row| match *row {
                DetailRow::UnifiedLine { hunk, line, .. } => number_at(hunk, line) == Some(number),
                DetailRow::SplitLine { hunk, row, .. } => [row.old, row.new]
                    .into_iter()
                    .flatten()
                    .any(|line| number_at(hunk, line) == Some(number)),
                _ => false,
            })
            .map(|offset| start + offset)
    }
}
