/// One prepared stop: its ordered current file indexes and derived progress.
pub(super) struct WalkthroughItem {
    pub label: String,
    pub description: String,
    pub files: Vec<usize>,
    pub viewed: usize,
    pub shown: usize,
    pub unviewed_shown: usize,
}
