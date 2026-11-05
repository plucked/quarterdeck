use std::collections::BTreeSet;

use qc_file_tree::FileKind;

use crate::{NoteDestination, PanelVisibility, StackReviewMode};

/// How the review pane is laid out and filtered, kept across launches:
/// whether the file tree shows, whether viewed files hide, the file kinds
/// hidden (an option-click on a kind chip), how stacked pull requests are
/// reviewed, and where new review comments go. Picked chips and file types are per pull request and not kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewLayoutState {
    pub file_tree: PanelVisibility,
    pub hide_viewed: bool,
    pub hidden_kinds: BTreeSet<FileKind>,
    pub stack_review: StackReviewMode,
    pub comment_destination: NoteDestination,
}

impl Default for ReviewLayoutState {
    fn default() -> Self {
        Self {
            file_tree: PanelVisibility::default(),
            hide_viewed: false,
            hidden_kinds: BTreeSet::new(),
            stack_review: StackReviewMode::default(),
            comment_destination: NoteDestination::GitHub,
        }
    }
}
