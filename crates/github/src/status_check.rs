use crate::CheckOutcome;

/// One check on a pull request's head commit: a check run (a GitHub Actions
/// job or another app's check) or a commit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCheck {
    /// The check run's name (`Group / job` for a job of a called workflow),
    /// or the commit status's context.
    pub name: String,
    pub source: CheckSource,
    pub outcome: CheckOutcome,
    /// Whether branch protection requires it to pass before merging.
    pub is_required: bool,
    /// The page of the check's run or log, as GitHub links it.
    pub details_url: Option<String>,
    /// When it started (ISO 8601), once it did.
    pub started_at: Option<String>,
    /// When it finished (ISO 8601), once it did.
    pub completed_at: Option<String>,
}

/// What produced a check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckSource {
    /// A job of a GitHub Actions workflow run, by the workflow's name.
    Workflow(String),
    /// A check run of another GitHub App, by the app's name.
    App(String),
    /// A commit status, set through the statuses API.
    CommitStatus,
    /// Nothing yet: a check the base branch requires that no check run or
    /// commit status has reported.
    NotReported,
}
