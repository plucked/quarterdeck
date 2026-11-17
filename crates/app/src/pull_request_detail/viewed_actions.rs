use gpui::Context;

use super::pull_request_detail::PullRequestDetail;
use super::viewed_change::ViewedChange;

impl PullRequestDetail {
    /// Applies one explicit bulk action and remembers only the files it changes.
    pub(super) fn set_files_viewed(
        &mut self,
        indices: &[usize],
        viewed: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(scope) = self.review_scope().cloned() else {
            return;
        };
        self.viewed_undo.clear();
        for index in indices {
            let Some(file) = self.loaded_files().get(*index).cloned() else {
                continue;
            };
            let before = self.file_review.state_of(&file);
            if let Some(toggle) = self.file_review.set_viewed(&file, viewed) {
                self.viewed_undo.push(ViewedChange {
                    filename: file.filename().into(),
                    fingerprint: file.fingerprint,
                    was_viewed: before.is_viewed,
                    was_collapsed: before.is_collapsed,
                });
                toggle.record(scope.clone(), cx);
            }
        }
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }

    /// Restores marks and folds, ignoring any file replaced by a newer patch.
    pub(super) fn undo_viewed_files(&mut self, cx: &mut Context<Self>) {
        let Some(scope) = self.review_scope().cloned() else {
            return;
        };
        let changes = std::mem::take(&mut self.viewed_undo);
        for change in changes {
            let file = self
                .loaded_files()
                .iter()
                .find(|file| {
                    file.filename() == change.filename && file.fingerprint == change.fingerprint
                })
                .cloned();
            if let Some(file) = file
                && let Some(toggle) = change.restore(&file, &mut self.file_review)
            {
                toggle.record(scope.clone(), cx);
            }
        }
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }
}
