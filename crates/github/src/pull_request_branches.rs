/// The two branches of a pull request: the one its changes come from
/// (head) and the one they go into (base). Read from GraphQL
/// (`headRefName`, `baseRefName`); the search and issues APIs that list pull
/// requests lack them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestBranches {
    /// The branch the changes come from, e.g. `feat/login`.
    pub head_ref: String,
    /// The branch they go into, e.g. `main`.
    pub base_ref: String,
}
