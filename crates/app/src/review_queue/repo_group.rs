use qd_github::RepoRef;

use super::others_group::OthersGroup;

/// What a repository's header shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoGroup {
    pub repo: RepoRef,
    /// Pull requests of it in the shown list.
    pub pull_requests: usize,
    pub is_collapsed: bool,
    /// Its filter, in the Others tab; `None` in the other tabs.
    pub others: Option<OthersGroup>,
}
