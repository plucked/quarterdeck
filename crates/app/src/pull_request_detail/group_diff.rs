use std::collections::{HashMap, HashSet};

use gpui::{ListAlignment, ListOffset, ListState, px};

use super::changed_rows::ChangedRows;
use super::detail_row_list::DetailRowList;
use super::file_review_state::FileReviewState;
use super::loaded_file::LoadedFile;

/// A guided step's diff disclosures and scroll, separate from the full PR diff.
pub(super) struct GroupDiff {
    pub expanded: HashSet<usize>,
    pub rows: DetailRowList,
    pub list: ListState,
    viewed: HashSet<usize>,
    expanded_before_viewed: HashMap<usize, bool>,
}

impl Default for GroupDiff {
    fn default() -> Self {
        Self {
            expanded: HashSet::new(),
            rows: DetailRowList::default(),
            list: ListState::new(0, ListAlignment::Top, px(600.)),
            viewed: HashSet::new(),
            expanded_before_viewed: HashMap::new(),
        }
    }
}

impl GroupDiff {
    /// Opens every member when entering a step, regardless of the ordinary diff's folds or marks.
    pub fn for_members(members: &[usize], files: &[LoadedFile], review: &FileReviewState) -> Self {
        Self {
            expanded: members.iter().copied().collect(),
            viewed: viewed_members(members, files, review),
            ..Self::default()
        }
    }

    /// Collapses newly viewed files and restores their previous disclosure when the mark is removed.
    pub fn reconcile_viewed(
        &mut self,
        members: &[usize],
        files: &[LoadedFile],
        review: &FileReviewState,
    ) {
        let viewed = viewed_members(members, files, review);
        // React only to mark transitions, so refreshes keep manual disclosure choices intact.
        for &file in viewed.difference(&self.viewed) {
            self.expanded_before_viewed
                .insert(file, self.expanded.remove(&file));
        }
        for &file in self.viewed.difference(&viewed) {
            if self.expanded_before_viewed.remove(&file).unwrap_or(true) {
                self.expanded.insert(file);
            } else {
                self.expanded.remove(&file);
            }
        }
        self.viewed = viewed;
    }

    /// Replaces derived rows, retaining the visible row or its file header when it collapses.
    pub fn replace_rows(&mut self, rows: DetailRowList) {
        // Anchor by content, since opening an earlier file shifts all later row indexes.
        let top = self.list.logical_scroll_top();
        let old_row = self.rows.get(top.item_ix);
        let old_file = self.rows.file_at_row(top.item_ix);
        let same_row =
            old_row.and_then(|old| (0..rows.len()).find(|index| rows.get(*index) == Some(old)));
        let target = same_row
            .or_else(|| old_file.and_then(|file| rows.file_start(file)))
            .unwrap_or(0);
        let changed = ChangedRows::between(
            self.rows.rows_in(0..self.rows.len()),
            rows.rows_in(0..rows.len()),
        );
        if changed.old_len > 0 || changed.new_len > 0 {
            self.list.splice(
                changed.start..changed.start + changed.old_len,
                changed.new_len,
            );
        }
        if rows.len() > 0 {
            self.list.scroll_to(ListOffset {
                item_ix: target,
                offset_in_item: if same_row.is_some() {
                    top.offset_in_item
                } else {
                    px(0.)
                },
            });
        }
        self.rows = rows;
    }

    /// Settings and full-data rebuilds invalidate measured heights while keeping the group's scroll.
    pub fn reset_measurements(&self) {
        let top = self.list.logical_scroll_top();
        self.list.reset(self.rows.len());
        if self.rows.len() > 0 {
            self.list.scroll_to(top);
        }
    }
}

/// Resolves viewed marks only for the current step's actual loaded files.
fn viewed_members(
    members: &[usize],
    files: &[LoadedFile],
    review: &FileReviewState,
) -> HashSet<usize> {
    members
        .iter()
        .copied()
        .filter(|file| {
            files
                .get(*file)
                .is_some_and(|loaded| review.is_viewed(loaded.filename()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use qd_diff::DiffOptions;
    use qd_github::PullRequestFile;

    use super::*;

    /// Builds a text diff without any external services or persisted marks.
    fn loaded(filename: &str) -> LoadedFile {
        LoadedFile::build(
            PullRequestFile {
                filename: filename.into(),
                previous_filename: None,
                status: "modified".into(),
                additions: 1,
                deletions: 1,
                patch: Some("@@ -1 +1 @@\n-old\n+new\n".into()),
                sha: None,
            },
            DiffOptions::default(),
        )
    }

    /// Single-file and grouped steps open all members, including previously viewed or folded files.
    #[test]
    fn entering_a_step_expands_every_member() {
        let files = [loaded("first.rs"), loaded("second.rs")];
        let mut review = FileReviewState::default();
        review.set_viewed(&files[0], true);
        review.toggle_collapsed(files[1].filename());

        let mut group = GroupDiff::for_members(&[1, 0], &files, &review);
        group.reconcile_viewed(&[1, 0], &files, &review);
        assert_eq!(group.expanded, HashSet::from([0, 1]));
        let single = GroupDiff::for_members(&[0], &files, &review);
        assert_eq!(single.expanded, HashSet::from([0]));
        assert!(review.state_of(&files[0]).is_collapsed);
        assert!(review.state_of(&files[1]).is_collapsed);
    }

    /// Viewed transitions collapse only their files, while refreshes and undo preserve manual folds.
    #[test]
    fn viewed_transitions_preserve_step_disclosures() {
        let files = [loaded("first.rs"), loaded("second.rs")];
        let mut review = FileReviewState::default();
        let members = [0, 1];
        let mut group = GroupDiff::for_members(&members, &files, &review);

        // A manual fold survives an unrelated refresh and a bulk viewed action.
        group.expanded.remove(&1);
        group.reconcile_viewed(&members, &files, &review);
        assert_eq!(group.expanded, HashSet::from([0]));
        review.set_viewed(&files[0], true);
        review.set_viewed(&files[1], true);
        group.reconcile_viewed(&members, &files, &review);
        assert!(group.expanded.is_empty());

        // Viewed files can still be reopened deliberately without the next refresh closing them.
        group.expanded.insert(0);
        group.reconcile_viewed(&members, &files, &review);
        assert_eq!(group.expanded, HashSet::from([0]));

        // Removing the marks restores the disclosures from before the bulk action.
        review.set_viewed(&files[0], false);
        review.set_viewed(&files[1], false);
        group.reconcile_viewed(&members, &files, &review);
        assert_eq!(group.expanded, HashSet::from([0]));
    }
}
