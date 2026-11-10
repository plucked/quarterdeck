use gpui::{ClipboardItem, Context, Pixels, Point, Window};

use super::diff_selection::DiffSelection;
use super::pull_request_detail::PullRequestDetail;
use super::selected_diff_text::{row_code_text, selected_diff_text};
use super::selection_granularity::SelectionGranularity;
use crate::diff_view::{CodeCells, DiffPosition, SideSelection};
use crate::word_range_at::word_range_at;

/// Dragging past the pane's edge scrolls by this share of the overshoot per mouse move.
const AUTOSCROLL_SHARE: f32 = 0.5;

impl PullRequestDetail {
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
        let granularity = SelectionGranularity::of(click_count);
        let text = self
            .rows
            .get(position.row)
            .and_then(|row| row_code_text(row, side, self.loaded_files()))
            .unwrap_or_default();
        let range = match granularity {
            SelectionGranularity::Drag => position.byte..position.byte,
            SelectionGranularity::Word => word_range_at(text, position.byte),
            SelectionGranularity::Line => 0..text.len(),
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
        self.is_selecting = granularity == SelectionGranularity::Drag;
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

    /// Drops the diff selection (a double click opening a comment selected a word).
    pub(super) fn clear_diff_selection(&mut self) {
        self.diff_selection = None;
    }
}
