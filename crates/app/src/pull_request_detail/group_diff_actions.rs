use gpui::Context;

use super::changed_files::ChangedFiles;
use super::detail_row_list::DetailRowList;
use super::pull_request_detail::PullRequestDetail;

impl PullRequestDetail {
    /// Opens or closes a member diff without leaving its group or changing the full diff's folds.
    pub(super) fn toggle_group_file(&mut self, file: usize, cx: &mut Context<Self>) {
        let Some(item) = self
            .walkthrough
            .selected_group
            .and_then(|id| self.walkthrough.item(id))
        else {
            return;
        };
        if !item.files.contains(&file) {
            return;
        }
        if !self.walkthrough.group_diff.expanded.remove(&file) {
            self.walkthrough.group_diff.expanded.insert(file);
        }
        self.refresh_group_diff();
        self.select_diff_file(file);
        cx.notify();
    }

    /// Updates the selected step after file data, comments, settings or viewed marks change.
    pub(super) fn refresh_group_diff(&mut self) {
        let files = self
            .changed_files
            .as_ref()
            .map_or(&[][..], ChangedFiles::loaded);
        let Some(item) = self
            .walkthrough
            .selected_group
            .and_then(|id| self.walkthrough.buckets.get(id.bucket)?.items.get(id.item))
        else {
            return;
        };
        // A checkbox changes only this guided disclosure; ordinary diff folding stays independent.
        self.walkthrough
            .group_diff
            .reconcile_viewed(&item.files, files, &self.file_review);
        let rows = DetailRowList::group(
            &item.files,
            &self.walkthrough.group_diff.expanded,
            files,
            &self.file_review,
            self.diff_settings,
            &self.review.placement,
        );
        // Selection indexes and height estimates belonged to the previous row layout.
        self.clear_diff_selection();
        self.row_tops_width = None;
        self.walkthrough.group_diff.replace_rows(rows);
        if self.guided_diff_active() {
            self.invalidate_diff_find();
        }
    }
}
