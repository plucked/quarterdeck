use std::collections::HashMap;

use crate::github_client::PAGE_SIZE;
use crate::workflow_jobs_response::{
    CombinedStatus, WorkflowJobsPage, WorkflowRun, WorkflowRunsPage,
};
use crate::{GitHubClient, GitHubError, RepoRef, StatusCheck};

/// Pages of runs, or of one run's jobs, read at most (100 each).
const MAX_PAGES: usize = 10;

impl GitHubClient {
    /// The checks on commit `head_sha` as GitHub Actions and the statuses
    /// API tell them: every job of the latest run of each workflow, then the
    /// commit statuses. Core REST requests: one per 100 runs, one per run
    /// (per 100 jobs), one for the statuses. Needs the Actions and Commit
    /// statuses read permissions instead of the Checks permission; which
    /// checks are required is not known.
    pub fn workflow_job_checks(
        &self,
        repo: &RepoRef,
        head_sha: &str,
    ) -> Result<Vec<StatusCheck>, GitHubError> {
        let base = repo.api_url();

        // The runs on the commit; a workflow run again (another event) counts once, by its newest run.
        let runs_url = format!("{base}/actions/runs");
        let mut runs: Vec<WorkflowRun> = Vec::new();
        self.numbered_pages(
            &runs_url,
            &[("head_sha", head_sha)],
            MAX_PAGES,
            &mut |page: WorkflowRunsPage| runs.extend(page.workflow_runs),
        )?;
        let mut newest: HashMap<u64, WorkflowRun> = HashMap::new();
        for run in runs {
            match newest.get(&run.workflow_id) {
                Some(known) if known.id >= run.id => {}
                _ => {
                    newest.insert(run.workflow_id, run);
                }
            }
        }

        // Every job of each run's latest attempt.
        let mut checks = Vec::new();
        for run in newest.into_values() {
            let workflow = run.name.unwrap_or_else(|| "Workflow".to_owned());
            let jobs_url = format!("{base}/actions/runs/{}/jobs", run.id);
            self.numbered_pages(
                &jobs_url,
                &[("filter", "latest")],
                MAX_PAGES,
                &mut |page: WorkflowJobsPage| {
                    checks.extend(page.jobs.into_iter().map(|job| job.into_check(&workflow)));
                },
            )?;
        }

        // Commit statuses from other systems, latest per context.
        let status: CombinedStatus = self.get_json(
            &format!("{base}/commits/{head_sha}/status"),
            &[("per_page", PAGE_SIZE)],
        )?;
        checks.extend(status.into_checks());
        Ok(checks)
    }
}
