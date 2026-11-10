//! The REST answers the Actions fallback of a commit's checks reads: its
//! workflow runs, their jobs, and the commit's statuses.

use serde::Deserialize;

use crate::paged_requests::NumberedPage;
use crate::{CheckOutcome, CheckSource, StatusCheck};

/// A page of `GET /repos/{o}/{r}/actions/runs?head_sha=…`.
#[derive(Deserialize)]
pub(crate) struct WorkflowRunsPage {
    pub total_count: usize,
    pub workflow_runs: Vec<WorkflowRun>,
}

impl NumberedPage for WorkflowRunsPage {
    fn item_count(&self) -> usize {
        self.workflow_runs.len()
    }

    fn total_count(&self) -> usize {
        self.total_count
    }
}

/// One workflow run on the commit.
#[derive(Deserialize)]
pub(crate) struct WorkflowRun {
    pub id: u64,
    pub workflow_id: u64,
    /// The run's name: the workflow's, unless the workflow names its runs.
    pub name: Option<String>,
}

/// A page of `GET /repos/{o}/{r}/actions/runs/{id}/jobs`.
#[derive(Deserialize)]
pub(crate) struct WorkflowJobsPage {
    pub total_count: usize,
    pub jobs: Vec<WorkflowJob>,
}

impl NumberedPage for WorkflowJobsPage {
    fn item_count(&self) -> usize {
        self.jobs.len()
    }

    fn total_count(&self) -> usize {
        self.total_count
    }
}

/// One job of a run: what a check run of GitHub Actions stands for.
#[derive(Deserialize)]
pub(crate) struct WorkflowJob {
    name: String,
    /// `queued`, `in_progress`, `completed`, `waiting`, `requested` or `pending`.
    status: String,
    /// Set once completed: `success`, `failure`, `cancelled`, `skipped`, …
    conclusion: Option<String>,
    html_url: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
}

/// `GET /repos/{o}/{r}/commits/{sha}/status`: the commit's statuses, latest per context.
#[derive(Deserialize)]
pub(crate) struct CombinedStatus {
    statuses: Vec<CommitStatus>,
}

#[derive(Deserialize)]
struct CommitStatus {
    context: String,
    /// `success`, `failure`, `error` or `pending`.
    state: String,
    target_url: Option<String>,
    created_at: Option<String>,
}

impl WorkflowJob {
    /// The check this job stands for in workflow `workflow`. Which checks
    /// are required is not known here.
    pub fn into_check(self, workflow: &str) -> StatusCheck {
        let status = self.status.to_uppercase();
        let conclusion = self.conclusion.map(|conclusion| conclusion.to_uppercase());
        StatusCheck {
            outcome: CheckOutcome::from_check_run(&status, conclusion.as_deref()),
            name: self.name,
            source: CheckSource::Workflow(workflow.to_owned()),
            is_required: false,
            details_url: self.html_url,
            started_at: self.started_at,
            completed_at: self.completed_at,
        }
    }
}

impl CombinedStatus {
    /// The commit's statuses as checks.
    pub fn into_checks(self) -> impl Iterator<Item = StatusCheck> {
        self.statuses.into_iter().map(|status| StatusCheck {
            outcome: CheckOutcome::from_status_state(&status.state.to_uppercase()),
            name: status.context,
            source: CheckSource::CommitStatus,
            is_required: false,
            details_url: status.target_url,
            started_at: status.created_at,
            completed_at: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A job reads like the check run it stands for, under its run's workflow.
    #[test]
    fn jobs_become_workflow_checks() {
        let page: WorkflowJobsPage = serde_json::from_str(
            r#"{"total_count":2,"jobs":[
              {"name":"Unit / run","status":"completed","conclusion":"skipped",
               "html_url":"https://github.com/o/r/actions/runs/1/job/2",
               "started_at":"2026-11-09T15:32:35Z","completed_at":"2026-11-09T15:32:35Z"},
              {"name":"lint","status":"in_progress","conclusion":null,
               "html_url":null,"started_at":"2026-11-09T15:32:35Z","completed_at":null}]}"#,
        )
        .unwrap();
        let checks: Vec<StatusCheck> = page
            .jobs
            .into_iter()
            .map(|job| job.into_check("CI"))
            .collect();
        assert_eq!(checks[0].outcome, CheckOutcome::Skipped);
        assert_eq!(checks[0].source, CheckSource::Workflow("CI".to_owned()));
        assert_eq!(checks[1].outcome, CheckOutcome::Running);
    }

    /// Commit statuses keep their context and state.
    #[test]
    fn statuses_become_commit_status_checks() {
        let status: CombinedStatus = serde_json::from_str(
            r#"{"state":"failure","statuses":[
              {"context":"ci/legacy","state":"error","target_url":null,"created_at":null}]}"#,
        )
        .unwrap();
        let checks: Vec<StatusCheck> = status.into_checks().collect();
        assert_eq!(checks[0].name, "ci/legacy");
        assert_eq!(checks[0].outcome, CheckOutcome::Failed);
    }
}
