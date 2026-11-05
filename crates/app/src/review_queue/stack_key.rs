use qc_github::{PullRequest, RepoRef};

/// Identifies a stack: its repository and its number there.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StackKey {
    pub repo: RepoRef,
    pub number: u64,
}

impl StackKey {
    /// The stack `pull_request` belongs to; `None` for a standalone one.
    pub fn of(pull_request: &PullRequest) -> Option<Self> {
        Some(Self {
            repo: pull_request.repo.clone(),
            number: pull_request.stack.as_ref()?.number,
        })
    }
}
