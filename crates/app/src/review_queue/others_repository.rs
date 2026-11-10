use qd_github::RepoRef;

use super::others_group::OthersGroup;

/// A repository the Others tab shows, in order, with its [`OthersGroup`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OthersRepository {
    pub repo: RepoRef,
    pub group: OthersGroup,
}
