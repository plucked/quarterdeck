use qd_cache::PatchFingerprint;

/// A bulk action's original mark and fold, bound to the exact patch for Undo.
pub(super) struct ViewedChange {
    pub filename: String,
    pub fingerprint: PatchFingerprint,
    pub was_viewed: bool,
    pub was_collapsed: bool,
}

impl ViewedChange {
    /// Restores only this patch's original mark and fold, leaving newer patches untouched.
    pub fn restore(
        &self,
        file: &super::loaded_file::LoadedFile,
        review: &mut super::file_review_state::FileReviewState,
    ) -> Option<super::viewed_toggle::ViewedToggle> {
        if file.filename() != self.filename || file.fingerprint != self.fingerprint {
            return None;
        }
        // Restore the mark first; marking also changes the fold, which is restored separately.
        let toggle = review.set_viewed(file, self.was_viewed);
        if review.state_of(file).is_collapsed != self.was_collapsed {
            review.toggle_collapsed(file.filename());
        }
        toggle
    }
}

#[cfg(test)]
mod tests {
    use qd_diff::DiffOptions;
    use qd_github::PullRequestFile;

    use super::*;
    use crate::pull_request_detail::file_review_state::FileReviewState;
    use crate::pull_request_detail::loaded_file::LoadedFile;

    /// Undo restores a preexisting fold and refuses to apply a mark to a changed patch.
    #[test]
    fn undo_preserves_original_fold_and_rejects_replaced_patch() {
        let source = PullRequestFile {
            filename: "a.rs".into(),
            previous_filename: None,
            status: "modified".into(),
            additions: 1,
            deletions: 1,
            patch: Some("@@ -1 +1 @@\n-old\n+new\n".into()),
            sha: None,
        };
        let file = LoadedFile::build(source.clone(), DiffOptions::default());
        let change = ViewedChange {
            filename: "a.rs".into(),
            fingerprint: file.fingerprint,
            was_viewed: false,
            was_collapsed: true,
        };
        let mut review = FileReviewState::default();
        review.set_viewed(&file, true);

        // A group acknowledgement can be undone without expanding an originally folded file.
        assert!(change.restore(&file, &mut review).is_some());
        assert!(!review.is_viewed("a.rs"));
        assert!(review.state_of(&file).is_collapsed);

        // The same path at a new patch never inherits this Undo's old state.
        let changed = LoadedFile::build(
            PullRequestFile {
                patch: Some("@@ -1 +1 @@\n-old\n+newer\n".into()),
                ..source
            },
            DiffOptions::default(),
        );
        review.set_viewed(&changed, true);
        assert!(change.restore(&changed, &mut review).is_none());
        assert!(review.is_viewed("a.rs"));
    }
}
