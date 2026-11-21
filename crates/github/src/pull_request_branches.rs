/// The two branches of a pull request: the one its changes come from
/// (head) and the one they go into (base). Read from GraphQL
/// (`headRefName`, `baseRefName`); the search and issues APIs that list pull
/// requests lack them.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PullRequestBranches {
    /// The branch the changes come from, e.g. `feat/login`.
    pub head_ref: String,
    /// The branch they go into, e.g. `main`.
    pub base_ref: String,
    /// Qualified head repository, absent for a deleted fork or an older cached row.
    pub head_repository: Option<String>,
    /// Default branch of the base repository, the boundary for inferred stacks.
    pub default_branch: Option<String>,
    pub revision: Option<crate::PullRequestRevision>,
}
