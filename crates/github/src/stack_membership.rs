/// Where a pull request sits in a GitHub stack: a chain of pull requests
/// where each targets the branch of the one below it. Read from GraphQL
/// (`PullRequest.stack`, `PullRequest.stackEntry`); the search API that lists
/// pull requests does not carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackMembership {
    /// The stack's number in its repository (`/repos/{o}/{r}/stacks/{number}`).
    pub number: u64,
    /// 1 for the layer on the trunk, counting up the stack.
    pub position: u64,
    /// How many pull requests the stack has.
    pub size: u64,
    /// The branch the bottom layer targets, e.g. `main`.
    pub base_ref: String,
}
