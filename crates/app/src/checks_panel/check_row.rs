use qd_github::CheckOutcome;

use super::check_tree::{CheckTree, TreeEntry};
use super::unfolded_checks::UnfoldedChecks;

/// One row of the Checks tab's list, by index into the [`CheckTree`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckRow {
    /// The small label above a section, with its count.
    Heading(CheckSection),
    /// A failed, cancelled, running, queued or expected check listed above
    /// the tree, with where it sits.
    Flagged(usize),
    /// A workflow, folded or not.
    Workflow(usize),
    /// A group of a workflow, by the workflow's and the entry's index.
    Group { workflow: usize, entry: usize },
    /// A check in the tree.
    Job { check: usize, depth: TreeDepth },
}

/// A section of the Checks tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckSection {
    /// The failed and cancelled checks, flat.
    Failed,
    /// The running and queued checks, flat.
    InProgress,
    /// The required checks nothing reported yet, flat.
    Expected,
    /// Every check as a tree, with the skipped toggle.
    All,
}

/// How deep a check sits in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeDepth {
    InWorkflow,
    InGroup,
}

/// Whether the tree shows the checks that did not run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SkippedChecks {
    #[default]
    Hidden,
    Shown,
}

impl CheckRow {
    /// The rows of `tree`: the failed checks, then the pending ones, then
    /// the required ones not reported yet (each section only when it has
    /// any), then the tree, unfolded where the
    /// reader opened it, without what did not run unless `skipped` shows it.
    pub fn build_all(
        tree: &CheckTree,
        unfolded: &UnfoldedChecks,
        skipped: SkippedChecks,
    ) -> Vec<Self> {
        let mut rows = Vec::new();

        // The flat sections answer "what needs me" without opening anything.
        for (section, checks) in [
            (CheckSection::Failed, tree.broken()),
            (CheckSection::InProgress, tree.pending()),
            (CheckSection::Expected, tree.expected()),
        ] {
            if !checks.is_empty() {
                rows.push(Self::Heading(section));
                rows.extend(checks.iter().copied().map(Self::Flagged));
            }
        }

        // The tree, folded by default.
        rows.push(Self::Heading(CheckSection::All));
        let is_shown = |ran: usize| ran > 0 || skipped == SkippedChecks::Shown;
        for (workflow_index, workflow) in tree.workflows().iter().enumerate() {
            if !is_shown(workflow.tally.ran()) {
                continue;
            }
            rows.push(Self::Workflow(workflow_index));
            if !unfolded.contains(&workflow.name, None) {
                continue;
            }
            for (entry_index, entry) in workflow.entries.iter().enumerate() {
                match entry {
                    TreeEntry::Group(group) if is_shown(group.tally.ran()) => {
                        rows.push(Self::Group {
                            workflow: workflow_index,
                            entry: entry_index,
                        });
                        if unfolded.contains(&workflow.name, Some(&group.name)) {
                            rows.extend(
                                group
                                    .jobs
                                    .iter()
                                    .filter(|check| is_job_shown(tree, **check, skipped))
                                    .map(|check| Self::Job {
                                        check: *check,
                                        depth: TreeDepth::InGroup,
                                    }),
                            );
                        }
                    }
                    TreeEntry::Group(_) => {}
                    TreeEntry::Job(check) => {
                        if is_job_shown(tree, *check, skipped) {
                            rows.push(Self::Job {
                                check: *check,
                                depth: TreeDepth::InWorkflow,
                            });
                        }
                    }
                }
            }
        }
        rows
    }
}

/// Whether check `index` shows in the tree: it ran, or skipped ones show.
fn is_job_shown(tree: &CheckTree, index: usize, skipped: SkippedChecks) -> bool {
    skipped == SkippedChecks::Shown
        || tree
            .check(index)
            .is_some_and(|check| check.outcome != CheckOutcome::Skipped)
}

#[cfg(test)]
mod tests {
    use qd_github::{CheckSource, StatusCheck};

    use super::*;

    /// A job of workflow `Tests` named `name` with `outcome`.
    fn job(name: &str, outcome: CheckOutcome) -> StatusCheck {
        StatusCheck {
            name: name.to_owned(),
            source: CheckSource::Workflow("Tests".to_owned()),
            outcome,
            is_required: false,
            details_url: None,
            started_at: None,
            completed_at: None,
        }
    }

    /// A tree with a failure, a running job and a skipped group.
    fn sample_tree() -> CheckTree {
        CheckTree::build(vec![
            job("Lint", CheckOutcome::Failed),
            job("Unit / run", CheckOutcome::Running),
            job("Unit / check-cache", CheckOutcome::Passed),
            job("E2E / run", CheckOutcome::Skipped),
            job("E2E / check-cache", CheckOutcome::Skipped),
        ])
    }

    /// Folded, the tab lists what needs attention and one row per workflow.
    #[test]
    fn folded_rows_flag_what_needs_attention() {
        let rows = CheckRow::build_all(
            &sample_tree(),
            &UnfoldedChecks::default(),
            SkippedChecks::Hidden,
        );
        assert_eq!(
            rows,
            [
                CheckRow::Heading(CheckSection::Failed),
                CheckRow::Flagged(0),
                CheckRow::Heading(CheckSection::InProgress),
                CheckRow::Flagged(1),
                CheckRow::Heading(CheckSection::All),
                CheckRow::Workflow(0),
            ]
        );
    }

    /// Required checks nothing reported get their own section and node.
    #[test]
    fn expected_checks_are_flagged() {
        let mut checks = vec![job("Lint", CheckOutcome::Passed)];
        checks.push(StatusCheck {
            name: "check-lfs".to_owned(),
            source: CheckSource::NotReported,
            outcome: CheckOutcome::Expected,
            is_required: true,
            details_url: None,
            started_at: None,
            completed_at: None,
        });
        let tree = CheckTree::build(checks);
        let rows = CheckRow::build_all(&tree, &UnfoldedChecks::default(), SkippedChecks::Hidden);
        assert_eq!(
            rows,
            [
                CheckRow::Heading(CheckSection::Expected),
                CheckRow::Flagged(1),
                CheckRow::Heading(CheckSection::All),
                CheckRow::Workflow(0),
                CheckRow::Workflow(1),
            ]
        );
        assert_eq!(tree.workflows()[0].name, "Expected");
        assert_eq!(tree.path_of(1), "Required, waiting for its status");
    }

    /// Unfolding shows entries; a group that only skipped shows on request.
    #[test]
    fn unfolded_rows_hide_skipped_groups_until_asked() {
        let tree = sample_tree();
        let mut unfolded = UnfoldedChecks::default();
        unfolded.toggle("Tests", None);
        let hidden = CheckRow::build_all(&tree, &unfolded, SkippedChecks::Hidden);
        let groups = |rows: &[CheckRow]| {
            rows.iter()
                .filter(|row| matches!(row, CheckRow::Group { .. }))
                .count()
        };
        assert_eq!(groups(&hidden), 1);
        let shown = CheckRow::build_all(&tree, &unfolded, SkippedChecks::Shown);
        assert_eq!(groups(&shown), 2);
    }
}
