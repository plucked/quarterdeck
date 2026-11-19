use gpui::Context;
use qd_cache::FilesMode;

use super::detail_mode::DetailMode;
use super::pull_request_detail::PullRequestDetail;
use super::side_panel_tab::SidePanelTab;
use super::walkthrough_item_id::WalkthroughItemId;
use crate::ui_state::UiStateRecorder;

impl PullRequestDetail {
    /// Main navigation gives each review surface its own content and matching navigator.
    pub(super) fn select_detail_mode(&mut self, mode: DetailMode, cx: &mut Context<Self>) {
        self.focus_drawer.preview_task = None;
        self.focus_drawer.preview = None;
        match mode {
            DetailMode::Diff => self.return_to_file_diff(cx),
            DetailMode::GuidedReview => {
                self.set_detail_mode(mode, cx);
                self.resume_guided_review(cx);
            }
            DetailMode::Summary | DetailMode::Brief => self.set_detail_mode(mode, cx),
        }
        if mode.shows_diff() {
            self.show_side_panel_tab(SidePanelTab::Files, cx);
        }
        if mode == DetailMode::GuidedReview && self.file_tree.show_panel() {
            UiStateRecorder::record_review_layout(cx, self.review_layout());
        }
        cx.notify();
    }

    /// Changes the active surface without discarding either review's disclosure or scroll state.
    pub(super) fn set_detail_mode(&mut self, mode: DetailMode, cx: &mut Context<Self>) {
        if self.brief().mode != mode {
            self.diff_find = None;
            self.clear_diff_selection();
            self.row_tops_width = None;
            self.selected_diff_file = None;
        }
        self.brief_mut().mode = mode;
        let files_mode = match mode {
            DetailMode::Diff => FilesMode::Tree,
            DetailMode::GuidedReview => FilesMode::Plan,
            DetailMode::Summary | DetailMode::Brief => self.focus_drawer.mode,
        };
        if self.focus_drawer.mode != files_mode {
            self.focus_drawer.mode = files_mode;
            UiStateRecorder::record_review_layout(cx, self.review_layout());
        }
        self.refresh_brief_rows();
        cx.notify();
    }

    /// Reopens the selected step, or starts at the first visible unfinished step after loading.
    pub(super) fn resume_guided_review(&mut self, cx: &mut Context<Self>) {
        if self.brief().mode != DetailMode::GuidedReview {
            return;
        }
        let Some(id) = self.walkthrough.entry_item() else {
            return;
        };
        self.walkthrough.open_bucket = Some(self.walkthrough.buckets[id.bucket].order);
        self.refresh_tree_rows();
        self.select_walkthrough_item(id, cx);
    }

    /// Opens a member inside its owning step even when the same file belongs to another bucket.
    pub(super) fn open_guided_file_in_item(
        &mut self,
        id: WalkthroughItemId,
        file: usize,
        cx: &mut Context<Self>,
    ) {
        if !self
            .walkthrough
            .item(id)
            .is_some_and(|item| item.files.contains(&file))
        {
            return;
        }
        self.select_walkthrough_item(id, cx);
        self.walkthrough.group_diff.expanded.insert(file);
        self.refresh_group_diff();
        self.reveal_file(file, cx);
        cx.notify();
    }

    /// Finds a file in the selected step first, then in the saved reading order.
    pub(super) fn open_guided_file(&mut self, file: usize, cx: &mut Context<Self>) -> bool {
        let selected = self.walkthrough.selected_group.filter(|id| {
            self.walkthrough
                .item(*id)
                .is_some_and(|item| item.files.contains(&file))
        });
        let id = selected.or_else(|| {
            self.walkthrough
                .buckets
                .iter()
                .enumerate()
                .find_map(|(bucket, contents)| {
                    contents
                        .items
                        .iter()
                        .position(|item| item.files.contains(&file))
                        .map(|item| WalkthroughItemId { bucket, item })
                })
        });
        let Some(id) = id else {
            return false;
        };
        self.open_guided_file_in_item(id, file, cx);
        true
    }
}
