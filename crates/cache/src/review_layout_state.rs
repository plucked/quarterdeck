use std::collections::BTreeSet;

use qc_file_tree::FileKind;

use crate::{PanelVisibility, StackReviewMode};

/// How the review pane is laid out and filtered, kept across launches:
/// whether the file tree shows, whether viewed files hide, the file kinds
/// hidden (an option-click on a kind chip), and how stacked pull requests
/// are reviewed. Picked chips and file types are per pull request and not kept.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewLayoutState {
    pub file_tree: PanelVisibility,
    pub hide_viewed: bool,
    pub hidden_kinds: BTreeSet<FileKind>,
    pub stack_review: StackReviewMode,
}
