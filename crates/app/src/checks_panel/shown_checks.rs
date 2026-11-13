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
    /// Whether the checks need another refresh while the panel is visible.
    pub fn has_pending_checks(&self) -> bool {
        match self.content.tree() {
            Some(tree) => tree.tally().pending() > 0,
            None => self.rollup.is_some_and(CheckOutcome::is_pending),
        }
    }

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

#[cfg(test)]
mod tests {
    use qd_github::{CheckSource, StatusCheck};

    use super::*;

    /// Listed running checks keep refreshing when superseded runs failed the rollup.
    #[test]
    fn pending_checks_refresh_despite_failed_rollup() {
        let check = StatusCheck {
            name: "Current job".to_owned(),
            source: CheckSource::Workflow("CI".to_owned()),
            outcome: CheckOutcome::Running,
            is_required: true,
            details_url: None,
            started_at: None,
            completed_at: None,
        };
        for details in [
            CheckDetails::Listed(vec![check.clone()]),
            CheckDetails::FromActions(vec![check.clone()]),
        ] {
            let shown = ShownChecks::arrange(PullRequestChecks {
                head_sha: "head".to_owned(),
                rollup: Some(CheckOutcome::Failed),
                total_count: 2,
                details,
                conflict: None,
            });
            assert!(shown.has_pending_checks());
        }
    }

    /// The rollup drives refreshing only when individual checks cannot be read.
    #[test]
    fn unreadable_checks_use_the_rollup_for_refreshing() {
        let mut shown = ShownChecks::arrange(PullRequestChecks {
            head_sha: "head".to_owned(),
            rollup: Some(CheckOutcome::Running),
            total_count: 1,
            details: CheckDetails::NotPermitted,
            conflict: None,
        });
        assert!(shown.has_pending_checks());
        shown.rollup = Some(CheckOutcome::Passed);
        assert!(!shown.has_pending_checks());
    }

    /// Superseded pending runs do not keep refreshing completed current checks.
    #[test]
    fn current_checks_override_a_stale_pending_rollup() {
        let shown = ShownChecks::arrange(PullRequestChecks {
            head_sha: "head".to_owned(),
            rollup: Some(CheckOutcome::Running),
            total_count: 1,
            details: CheckDetails::Listed(Vec::new()),
            conflict: None,
        });
        assert!(!shown.has_pending_checks());
    }
}
