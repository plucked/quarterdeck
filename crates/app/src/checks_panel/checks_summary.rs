use qd_github::{CheckOutcome, MergeConflict};

use super::check_notes::conflict_sentence;
use super::check_tally::CheckTally;
use super::check_tree::CheckTree;
use super::shown_checks::ShownChecks;
use crate::display_text::counted;

/// What the summary says about the checks.
pub(super) struct Summary {
    pub(super) outcome: Option<CheckOutcome>,
    pub(super) verdict: String,
    pub(super) counts: String,
    /// The outcome counts the bar shows; none when single checks are unknown.
    pub(super) bar: Option<CheckTally>,
    /// The required checks' counts, when there are any.
    pub(super) required: Option<CheckTally>,
}

impl Summary {
    /// The summary of checks read one by one.
    pub(super) fn of_tree(tree: &CheckTree) -> Self {
        let tally = *tree.tally();
        let outcome = tally.outcome();
        let verdict = match outcome {
            None => "No checks on this commit".to_owned(),
            Some(CheckOutcome::Failed) => counted(
                tally.count(CheckOutcome::Failed),
                "check failed",
                "checks failed",
            ),
            Some(CheckOutcome::Cancelled) => {
                counted(tally.broken(), "check cancelled", "checks cancelled")
            }
            Some(CheckOutcome::Running | CheckOutcome::Queued) => counted(
                tally.pending(),
                "check still running",
                "checks still running",
            ),
            Some(CheckOutcome::Expected) => counted(
                tally.count(CheckOutcome::Expected),
                "required check not reported",
                "required checks not reported",
            ),
            Some(CheckOutcome::Passed | CheckOutcome::Neutral) => "All checks passed".to_owned(),
            Some(CheckOutcome::Skipped) => "Every check was skipped".to_owned(),
        };
        let counts = tally
            .nonzero()
            .map(|(outcome, count)| format!("{count} {}", outcome_word(outcome)))
            .collect::<Vec<_>>()
            .join(" · ");
        Self {
            outcome,
            verdict,
            counts,
            bar: (tally.ran() > 0).then_some(tally),
            required: (tree.required().total() > 0).then_some(*tree.required()),
        }
    }

    /// The summary when only GitHub's overall verdict and count are known.
    pub(super) fn of_rollup(shown: &ShownChecks) -> Self {
        let verdict = match shown.rollup {
            None => "No checks on this commit",
            Some(CheckOutcome::Failed | CheckOutcome::Cancelled) => "Checks are failing",
            Some(CheckOutcome::Running | CheckOutcome::Queued | CheckOutcome::Expected) => {
                "Checks are running"
            }
            Some(CheckOutcome::Passed | CheckOutcome::Neutral | CheckOutcome::Skipped) => {
                "Checks passed"
            }
        };
        Self {
            outcome: shown.rollup,
            verdict: verdict.to_owned(),
            counts: counted(shown.total_count, "check", "checks"),
            bar: None,
            required: None,
        }
    }

    /// The summary of a commit on which nothing reported (no checks, or
    /// only required ones waiting) says why, when a merge conflict is the reason.
    pub(super) fn with_conflict(self, conflict: Option<&MergeConflict>) -> Self {
        match (self.outcome, conflict) {
            (None | Some(CheckOutcome::Expected), Some(conflict)) => Self {
                outcome: Some(CheckOutcome::Failed),
                verdict: "No checks: merge conflict".to_owned(),
                counts: conflict_sentence(conflict),
                ..self
            },
            _ => self,
        }
    }
}

/// The word an outcome is counted with.
fn outcome_word(outcome: CheckOutcome) -> &'static str {
    match outcome {
        CheckOutcome::Failed => "failed",
        CheckOutcome::Running => "running",
        CheckOutcome::Queued => "queued",
        CheckOutcome::Expected => "expected",
        CheckOutcome::Cancelled => "cancelled",
        CheckOutcome::Neutral => "neutral",
        CheckOutcome::Passed => "passed",
        CheckOutcome::Skipped => "skipped",
    }
}

#[cfg(test)]
mod tests {
    use qd_github::{ConflictLocation, StatusCheck};

    use super::*;

    /// A commit without checks names the stack layer whose conflict holds them up.
    #[test]
    fn a_conflict_explains_missing_checks() {
        let conflict = MergeConflict {
            number: 36410,
            base_ref: "develop".to_owned(),
            location: ConflictLocation::LayerBelow,
        };
        let summary =
            Summary::of_tree(&CheckTree::build(Vec::new())).with_conflict(Some(&conflict));
        assert_eq!(summary.verdict, "No checks: merge conflict");
        assert_eq!(
            summary.counts,
            "#36410, lower in this stack, conflicts with develop."
        );
    }

    /// Required checks waiting on a conflicting commit are explained by the conflict.
    #[test]
    fn a_conflict_explains_waiting_required_checks() {
        let conflict = MergeConflict {
            number: 36410,
            base_ref: "develop".to_owned(),
            location: ConflictLocation::Shown,
        };
        let expected = StatusCheck {
            name: "check-lfs".to_owned(),
            source: qd_github::CheckSource::NotReported,
            outcome: CheckOutcome::Expected,
            is_required: true,
            details_url: None,
            started_at: None,
            completed_at: None,
        };
        let tree = CheckTree::build(vec![expected]);
        assert_eq!(
            Summary::of_tree(&tree).verdict,
            "1 required check not reported"
        );
        let summary = Summary::of_tree(&tree).with_conflict(Some(&conflict));
        assert_eq!(summary.verdict, "No checks: merge conflict");
    }
}
