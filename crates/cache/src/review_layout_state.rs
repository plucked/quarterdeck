use crate::PanelVisibility;

/// How the review pane is laid out and filtered, kept across launches:
/// whether the file tree shows, and the hide toggles of its filter.
/// Extension filters are per pull request and not kept.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReviewLayoutState {
    pub file_tree: PanelVisibility,
    pub hide_viewed: bool,
    pub hide_tests: bool,
    pub hide_generated: bool,
}
