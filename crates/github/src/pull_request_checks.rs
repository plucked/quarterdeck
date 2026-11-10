use crate::{CheckOutcome, MergeConflict, StatusCheck};

/// The checks on a pull request's head commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequestChecks {
    /// The commit the checks ran on.
    pub head_sha: String,
    /// GitHub's verdict over all checks; `None` when the commit has none.
    pub rollup: Option<CheckOutcome>,
    /// How many checks GitHub counts, readable or not.
    pub total_count: usize,
    pub details: CheckDetails,
    /// The merge conflict that keeps `pull_request` workflows from running
    /// on it (its own, or a stack layer's below it), when GitHub reports one.
    pub conflict: Option<MergeConflict>,
}

/// The single checks, or why they cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckDetails {
    /// Every check, as GitHub's rollup lists it, with which are required,
    /// then the required ones not reported yet ([`CheckOutcome::Expected`]).
    Listed(Vec<StatusCheck>),
    /// The token may not read check runs (GitHub refuses them to
    /// fine-grained tokens without the Checks permission), so the jobs of the
    /// commit's GitHub Actions runs and its commit statuses stand in, marked
    /// required by name from the base branch's rules, then the required ones
    /// not reported yet; checks of other apps are missing.
    FromActions(Vec<StatusCheck>),
    /// The token may read neither the check runs nor the Actions runs: only
    /// the rollup and the count are known.
    NotPermitted,
}
