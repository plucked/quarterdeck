use std::collections::BTreeSet;

use qc_file_tree::FileGroup;

use crate::{PanelVisibility, StackReviewMode};

/// How the review pane is laid out and filtered, kept across launches:
/// whether the file tree shows, the file groups its filter hides, and how
/// stacked pull requests are reviewed. Extension filters are per pull
/// request and not kept.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReviewLayoutState {
    pub file_tree: PanelVisibility,
    pub hidden_groups: BTreeSet<FileGroup>,
    pub stack_review: StackReviewMode,
}
