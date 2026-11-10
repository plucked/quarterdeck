use qd_github::{CheckDetails, CheckOutcome, MergeConflict, PullRequestChecks};

use super::check_tree::CheckTree;
use crate::display_date::unix_now;

/// Checks as last read.
pub struct ShownChecks {
    pub head_sha: String,
    pub rollup: Option<CheckOutcome>,
    pub total_count: usize,
    pub content: CheckContent,
    /// Where the single checks came from.
    pub origin: CheckOrigin,
    /// The merge conflict that keeps workflows from running, if any.
    pub conflict: Option<MergeConflict>,
    /// When they were read, in Unix seconds.
    pub read_at: i64,
}

/// Where the single checks of [`ShownChecks`] came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckOrigin {
    /// GitHub's rollup: every check, with which are required.
    Rollup,
    /// GitHub Actions jobs and commit statuses, read because the token may
    /// not read check runs: no required marks, no other apps' checks.
    ActionsJobs,
}

/// The checks arranged for reading, or why only the rollup is known.
pub enum CheckContent {
    Tree(Box<CheckTree>),
    /// The token may not read single checks.
    NotPermitted,
}

impl ShownChecks {
    /// The checks as read just now, arranged for reading.
    pub fn arrange(checks: PullRequestChecks) -> Self {
        let origin = match checks.details {
            CheckDetails::FromActions(_) => CheckOrigin::ActionsJobs,
            CheckDetails::Listed(_) | CheckDetails::NotPermitted => CheckOrigin::Rollup,
        };
        Self {
            origin,
            conflict: checks.conflict,
            head_sha: checks.head_sha,
            rollup: checks.rollup,
            total_count: checks.total_count,
            content: match checks.details {
                CheckDetails::Listed(listed) | CheckDetails::FromActions(listed) => {
                    CheckContent::Tree(Box::new(CheckTree::build(listed)))
                }
                CheckDetails::NotPermitted => CheckContent::NotPermitted,
            },
            read_at: unix_now(),
        }
    }
}
