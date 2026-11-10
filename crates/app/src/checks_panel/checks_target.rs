use qd_github::RepoRef;

/// The pull request whose checks the Checks tab shows: one pull request
/// (in a stack, the shown layer), and the version it was shown at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChecksTarget {
    pub repo: RepoRef,
    pub number: u64,
    /// The pull request's `updated_at` when shown: it moves on new commits.
    pub updated_at: String,
    /// The open stack layers below it, bottom first (none outside a stack
    /// or before the stack is known): a conflict in one keeps workflows
    /// from running on this one.
    pub layers_below: Vec<u64>,
}

impl ChecksTarget {
    /// Whether `other` is the same pull request, at any version.
    pub fn is_same_pull_request(&self, other: &Self) -> bool {
        self.repo == other.repo && self.number == other.number
    }
}
