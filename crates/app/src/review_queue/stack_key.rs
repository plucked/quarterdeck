use qd_github::{PullRequest, RepoRef};

/// Identifies a stack: its repository and its number there.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StackKey {
    pub repo: RepoRef,
    pub number: u64,
    /// Whether the number identifies a root PR rather than a GitHub stack.
    pub branch_root: bool,
}

impl StackKey {
    /// The stack `pull_request` belongs to; `None` for a standalone one.
    pub fn of(pull_request: &PullRequest) -> Option<Self> {
        Some(Self::native(
            &pull_request.repo,
            pull_request.stack.as_ref()?.number,
        ))
    }

    pub fn native(repo: &RepoRef, number: u64) -> Self {
        Self {
            repo: repo.clone(),
            number,
            branch_root: false,
        }
    }
}
