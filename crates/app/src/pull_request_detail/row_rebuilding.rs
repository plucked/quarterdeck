use gpui::{ListOffset, px};

use super::changed_rows::ChangedRows;
use super::detail_content::DetailContent;
use super::detail_row_list::DetailRowList;
use super::pull_request_detail::PullRequestDetail;
use super::row_height_estimate::estimate_row_tops;
use super::scroll_anchor::ScrollAnchor;
use crate::scrollbar::ListRowTops;

impl PullRequestDetail {
    /// Rebuilds every row and resets the list (which forgets row heights),
    /// then scrolls to the pending position once the files are in, or else
    /// to `anchor`.
    pub(super) fn rebuild_rows(&mut self, anchor: ScrollAnchor) {
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
        self.jumped_to_file = None;
        self.row_tops_width = None;
        self.refresh_tree_rows();
        // The shown pull request or target may have changed, and with it the brief.
        self.refresh_brief_rows();

        let files_settled = self.load_progress().is_none();
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

    /// Rebuilds every row, keeping the file at the top in view.
    pub(super) fn rebuild_rows_in_place(&mut self) {
        self.rebuild_rows(self.anchor_at_top());
    }

    /// The file whose rows are at the top of the pane, to keep in view
    /// across a rebuild; the top when above the files.
    pub(super) fn anchor_at_top(&self) -> ScrollAnchor {
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

    /// Scrolls the diff so row `row` is at the top.
    pub(super) fn scroll_to_row(&mut self, row: usize) {
        self.list.scroll_to(ListOffset {
            item_ix: row,
            offset_in_item: px(0.),
        });
    }
}
