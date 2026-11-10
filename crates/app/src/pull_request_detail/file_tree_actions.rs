use gpui::{Context, ScrollStrategy};
use qd_file_tree::{FileKind, NodeId};

use super::changed_files::ChangedFiles;
use super::folder_viewed_state::FolderViewedState;
use super::pull_request_detail::PullRequestDetail;
use super::side_panel_tab::SidePanelTab;
use crate::facet_chips::ChipClick;
use crate::ui_state::UiStateRecorder;

impl PullRequestDetail {
    /// Shows or hides the side panel (cmd-b), and saves it; the Checks tab
    /// stops or resumes reading with it.
    pub fn toggle_file_tree(&mut self, cx: &mut Context<Self>) {
        self.file_tree.toggle_visibility();
        UiStateRecorder::record_review_layout(cx, self.review_layout());
        self.sync_checks_visibility(cx);
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

    /// Collapses every folder of the tree while one is expanded, else expands them all.
    pub(super) fn toggle_all_folders(&mut self, cx: &mut Context<Self>) {
        if self.file_tree.has_expanded_folder() {
            self.file_tree.collapse_all_folders();
        } else {
            self.file_tree.expand_all_folders();
        }
        self.refresh_tree_rows();
        cx.notify();
    }

    /// Collapses or expands the folder at `path` in the tree.
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
            if let Some(toggle) = self.file_review.set_viewed(&files[file], mark_viewed) {
                toggle.record(scope.clone(), cx);
            }
        }
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }

    /// Shows file `file` in the file tree (the file header's tree button):
    /// opens the side panel on its Files tab, expands the folders above the
    /// file, scrolls the diff to its header so the tree highlights it, and
    /// scrolls the tree to its row.
    pub(super) fn show_file_in_tree(&mut self, file: usize, cx: &mut Context<Self>) {
        let Some(filename) = self
            .loaded_files()
            .get(file)
            .map(|loaded| loaded.filename().to_owned())
        else {
            return;
        };
        // Open the panel (saved, as cmd-b would) on the Files tab.
        if self.file_tree.show_panel() {
            UiStateRecorder::record_review_layout(cx, self.review_layout());
        }
        if self.review().tab != SidePanelTab::Files {
            self.show_side_panel_tab(SidePanelTab::Files, cx);
        }
        // Unfold the folders above the file, so its row exists.
        self.file_tree.expand_folders_above(&filename);
        self.refresh_tree_rows();
        // The tree highlights the file at the top of the diff: put this one there.
        self.reveal_file(file, cx);
        if let Some(row) = self.file_tree.row_of_file(file) {
            self.file_tree
                .scroll()
                .scroll_to_item(row, ScrollStrategy::Center);
        }
        cx.notify();
    }

    /// Recomputes the tree's visible rows from the files, filter and marks.
    pub(super) fn refresh_tree_rows(&mut self) {
        let files = self
            .changed_files
            .as_ref()
            .map_or(&[][..], ChangedFiles::loaded);
        self.file_tree.refresh_rows(files, &self.file_review);
    }
}
