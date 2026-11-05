use crate::{PanelVisibility, StackReviewMode};

/// How the review pane is laid out and filtered, kept across launches:
/// whether the file tree shows, the hide toggles of its filter, and how
/// stacked pull requests are reviewed.
/// Extension filters are per pull request and not kept.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReviewLayoutState {
    pub file_tree: PanelVisibility,
    pub hide_viewed: bool,
    pub hide_tests: bool,
    pub hide_generated: bool,
    pub stack_review: StackReviewMode,
}
