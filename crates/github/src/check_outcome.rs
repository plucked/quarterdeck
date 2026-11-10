/// Where one check stands, folded from GitHub's status and conclusion (a
/// check run) or state (a commit status) into what a reader acts on.
///
/// The variants are ordered by how much they need attention: sorting by
/// [`CheckOutcome::attention_rank`] puts failures first and skips last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CheckOutcome {
    /// Failed, timed out, needs action or could not start.
    Failed,
    /// Started and not finished.
    Running,
    /// Waiting to start: queued, requested, waiting or pending.
    Queued,
    /// Required by the base branch, but nothing has reported it yet (GitHub's
    /// "Expected — waiting for status to be reported").
    Expected,
    /// Stopped before it finished.
    Cancelled,
    /// Finished without a verdict (`NEUTRAL`) or replaced by a newer run (`STALE`).
    Neutral,
    /// Finished successfully.
    Passed,
    /// Did not run: its condition was false.
    Skipped,
}

impl CheckOutcome {
    /// The outcome of a check run from its `status` and, once completed, its
    /// `conclusion` (GraphQL enum names). A status or conclusion GitHub adds
    /// later counts as queued or neutral.
    pub fn from_check_run(status: &str, conclusion: Option<&str>) -> Self {
        match (status, conclusion) {
            ("COMPLETED", Some(conclusion)) => match conclusion {
                "SUCCESS" => Self::Passed,
                "FAILURE" | "TIMED_OUT" | "ACTION_REQUIRED" | "STARTUP_FAILURE" => Self::Failed,
                "CANCELLED" => Self::Cancelled,
                "SKIPPED" => Self::Skipped,
                _ => Self::Neutral,
            },
            ("IN_PROGRESS", _) => Self::Running,
            _ => Self::Queued,
        }
    }

    /// The outcome of a commit status (or of the whole rollup) from its
    /// `state`: a pending status is running, since GitHub cannot tell
    /// whether it started.
    pub fn from_status_state(state: &str) -> Self {
        match state {
            "SUCCESS" => Self::Passed,
            "FAILURE" | "ERROR" => Self::Failed,
            "EXPECTED" => Self::Expected,
            _ => Self::Running,
        }
    }

    /// How much the outcome needs attention: 0 is the most.
    pub fn attention_rank(self) -> u8 {
        match self {
            Self::Failed => 0,
            Self::Running => 1,
            Self::Queued => 2,
            Self::Expected => 3,
            Self::Cancelled => 4,
            Self::Neutral => 5,
            Self::Passed => 6,
            Self::Skipped => 7,
        }
    }

    /// Whether the check has started or is about to: an expected check may
    /// never be reported, so it does not count.
    pub fn is_pending(self) -> bool {
        matches!(self, Self::Running | Self::Queued)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A completed run takes its conclusion; an unfinished one its status.
    #[test]
    fn check_runs_map_status_and_conclusion() {
        assert_eq!(
            CheckOutcome::from_check_run("COMPLETED", Some("SUCCESS")),
            CheckOutcome::Passed
        );
        assert_eq!(
            CheckOutcome::from_check_run("COMPLETED", Some("TIMED_OUT")),
            CheckOutcome::Failed
        );
        assert_eq!(
            CheckOutcome::from_check_run("COMPLETED", Some("SKIPPED")),
            CheckOutcome::Skipped
        );
        assert_eq!(
            CheckOutcome::from_check_run("IN_PROGRESS", None),
            CheckOutcome::Running
        );
        assert_eq!(
            CheckOutcome::from_check_run("WAITING", None),
            CheckOutcome::Queued
        );
    }

    /// A commit status that errored failed; a pending one runs.
    #[test]
    fn commit_statuses_map_their_state() {
        assert_eq!(
            CheckOutcome::from_status_state("ERROR"),
            CheckOutcome::Failed
        );
        assert_eq!(
            CheckOutcome::from_status_state("PENDING"),
            CheckOutcome::Running
        );
    }
}
