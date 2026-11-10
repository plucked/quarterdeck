use qd_github::{CheckDetails, CheckOutcome, MergeConflict, PullRequestChecks};

use super::check_tree::CheckTree;
use crate::display_text::unix_now;

/// Checks as last read.
pub struct ShownChecks {
    pub head_sha: String,
    pub rollup: Option<CheckOutcome>,
    pub total_count: usize,
    pub content: CheckContent,
    /// The merge conflict that keeps workflows from running, if any.
    pub conflict: Option<MergeConflict>,
    /// When they were read, in Unix seconds.
    pub read_at: i64,
}

/// The single checks arranged for reading and where they came from, or
/// why only the rollup is known.
pub enum CheckContent {
    /// GitHub's rollup: every check, with which are required.
    Listed(Box<CheckTree>),
    /// GitHub Actions jobs and commit statuses, read because the token may
    /// not read check runs: no required marks, no other apps' checks.
    FromActions(Box<CheckTree>),
    /// The token may not read single checks.
    NotPermitted,
}

impl CheckContent {
    /// The single checks, wherever they came from; `None` when the token
    /// may not read them.
    pub fn tree(&self) -> Option<&CheckTree> {
        match self {
            Self::Listed(tree) | Self::FromActions(tree) => Some(tree),
            Self::NotPermitted => None,
        }
    }
}

impl ShownChecks {
    /// The checks as read just now, arranged for reading.
    pub fn arrange(checks: PullRequestChecks) -> Self {
        Self {
            conflict: checks.conflict,
            head_sha: checks.head_sha,
            rollup: checks.rollup,
            total_count: checks.total_count,
            content: match checks.details {
                CheckDetails::Listed(listed) => {
                    CheckContent::Listed(Box::new(CheckTree::build(listed)))
                }
                CheckDetails::FromActions(jobs) => {
                    CheckContent::FromActions(Box::new(CheckTree::build(jobs)))
                }
                CheckDetails::NotPermitted => CheckContent::NotPermitted,
            },
            read_at: unix_now(),
        }
    }
}
