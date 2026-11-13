//! Check runs and commit statuses read from GraphQL, accumulated across pages.

use std::collections::HashMap;

use serde::Deserialize;

use crate::{CheckOutcome, CheckSource, StatusCheck};

/// One check run or commit status as returned by the rollup.
#[derive(Deserialize)]
#[serde(tag = "__typename")]
enum ContextNode {
    CheckRun(CheckRunNode),
    StatusContext(StatusContextNode),
}

impl ContextNode {
    /// The Actions run that produced this job, if it is a workflow check.
    fn workflow_run(&self) -> Option<&WorkflowRunNode> {
        match self {
            Self::CheckRun(run) => run.check_suite.as_ref()?.workflow_run.as_ref(),
            Self::StatusContext(_) => None,
        }
    }

    /// The check this entry describes.
    fn into_check(self) -> StatusCheck {
        match self {
            Self::CheckRun(run) => {
                // An Actions job names its workflow; another app's check names the app.
                let source = match run.check_suite {
                    Some(CheckSuiteNode {
                        workflow_run: Some(workflow_run),
                        ..
                    }) => CheckSource::Workflow(workflow_run.workflow.name),
                    Some(CheckSuiteNode { app: Some(app), .. }) => CheckSource::App(app.name),
                    _ => CheckSource::App("Checks".to_owned()),
                };
                StatusCheck {
                    outcome: CheckOutcome::from_check_run(&run.status, run.conclusion.as_deref()),
                    name: run.name,
                    source,
                    is_required: run.is_required,
                    details_url: run.details_url,
                    started_at: run.started_at,
                    completed_at: run.completed_at,
                }
            }
            Self::StatusContext(status) => StatusCheck {
                outcome: CheckOutcome::from_status_state(&status.state),
                name: status.context,
                source: CheckSource::CommitStatus,
                is_required: status.is_required,
                details_url: status.target_url,
                started_at: status.created_at,
                completed_at: None,
            },
        }
    }
}

/// Raw check contexts, kept until every page has been read.
#[derive(Default, Deserialize)]
#[serde(transparent)]
pub(crate) struct CheckContexts(Vec<Option<ContextNode>>);

impl CheckContexts {
    /// How many contexts GitHub withheld from this token.
    pub fn unreadable_count(&self) -> usize {
        self.0.iter().filter(|node| node.is_none()).count()
    }

    /// Adds a page without losing the workflow run identities.
    pub fn append(&mut self, page: &mut Self) {
        self.0.append(&mut page.0);
    }

    /// Jobs from the newest run of each workflow, plus other apps' checks
    /// and commit statuses. Selection happens after all pages: a newer run
    /// may appear on a different page from the failures it supersedes.
    pub fn into_checks(self) -> Vec<StatusCheck> {
        // A workflow's stable ID keeps equally named workflows separate.
        let contexts: Vec<_> = self.0.into_iter().flatten().collect();
        let mut newest: HashMap<String, u64> = HashMap::new();
        for context in &contexts {
            if let Some(run) = context.workflow_run()
                && let Some(database_id) = run.database_id
            {
                let known = newest.entry(run.workflow.id.clone()).or_default();
                *known = (*known).max(database_id);
            }
        }

        // Keep every job of the selected run, including skipped jobs and
        // matrix jobs. Without a run ID there is no proof of supersession.
        contexts
            .into_iter()
            .filter(|context| {
                context.workflow_run().is_none_or(|run| {
                    run.database_id.is_none_or(|database_id| {
                        newest.get(&run.workflow.id) == Some(&database_id)
                    })
                })
            })
            .map(ContextNode::into_check)
            .collect()
    }
}

/// A job or app check, with its owning suite.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckRunNode {
    name: String,
    status: String,
    conclusion: Option<String>,
    details_url: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    is_required: bool,
    check_suite: Option<CheckSuiteNode>,
}

/// The app and optional Actions run that produced a check.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckSuiteNode {
    app: Option<NameNode>,
    workflow_run: Option<WorkflowRunNode>,
}

/// An Actions execution; a missing database ID cannot be ordered.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkflowRunNode {
    database_id: Option<u64>,
    workflow: WorkflowNode,
}

/// A workflow's stable identity and its display name.
#[derive(Deserialize)]
struct WorkflowNode {
    id: String,
    name: String,
}

/// The display name of a GitHub App.
#[derive(Deserialize)]
struct NameNode {
    name: String,
}

/// A commit status reported by another system.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusContextNode {
    context: String,
    state: String,
    target_url: Option<String>,
    created_at: Option<String>,
    is_required: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// A workflow job with the identifiers returned by GitHub's rollup.
    fn workflow_job(workflow: &str, run: u64, name: &str, conclusion: &str) -> serde_json::Value {
        json!({
            "__typename": "CheckRun", "name": name,
            "status": "COMPLETED", "conclusion": conclusion, "isRequired": true,
            "checkSuite": {"workflowRun": {
                "databaseId": run, "workflow": {"id": workflow, "name": "CI"}
            }}
        })
    }

    /// A later page supersedes failed and cancelled jobs of an older workflow run.
    #[test]
    fn newer_workflow_run_supersedes_failures_across_pages() {
        let mut contexts: CheckContexts = serde_json::from_value(json!([
            workflow_job("workflow", 10, "Result", "FAILURE"),
            workflow_job("workflow", 10, "Old job", "CANCELLED")
        ]))
        .unwrap();
        let mut later: CheckContexts =
            serde_json::from_value(json!([workflow_job("workflow", 20, "Result", "SUCCESS")]))
                .unwrap();

        contexts.append(&mut later);
        let checks = contexts.into_checks();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].name, "Result");
        assert_eq!(checks[0].outcome, CheckOutcome::Passed);
        assert!(checks[0].is_required);
    }

    /// A newer failure wins even when the older success is read afterwards.
    #[test]
    fn newer_failure_is_not_hidden_by_an_older_success() {
        let contexts: CheckContexts = serde_json::from_value(json!([
            workflow_job("workflow", 20, "Result", "FAILURE"),
            workflow_job("workflow", 10, "Result", "SUCCESS")
        ]))
        .unwrap();
        let checks = contexts.into_checks();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].outcome, CheckOutcome::Failed);
    }

    /// Workflows with the same display name and job name retain separate results.
    #[test]
    fn equally_named_workflows_are_distinct() {
        let contexts: CheckContexts = serde_json::from_value(json!([
            workflow_job("first", 10, "Result", "FAILURE"),
            workflow_job("second", 15, "Result", "FAILURE"),
            workflow_job("first", 20, "Result", "SUCCESS")
        ]))
        .unwrap();
        let checks = contexts.into_checks();
        assert_eq!(checks.len(), 2);
        assert_eq!(checks[0].outcome, CheckOutcome::Failed);
        assert_eq!(checks[1].outcome, CheckOutcome::Passed);
    }

    /// The latest run keeps all jobs, including skips and jobs still queued.
    #[test]
    fn latest_run_keeps_every_job_and_its_current_state() {
        let mut queued = workflow_job("workflow", 20, "Queued job", "SUCCESS");
        queued["status"] = json!("QUEUED");
        queued["conclusion"] = serde_json::Value::Null;
        let contexts: CheckContexts = serde_json::from_value(json!([
            workflow_job("workflow", 10, "Result", "FAILURE"),
            workflow_job("workflow", 20, "Result", "SUCCESS"),
            workflow_job("workflow", 20, "Optional job", "SKIPPED"),
            queued
        ]))
        .unwrap();
        let outcomes: Vec<_> = contexts
            .into_checks()
            .iter()
            .map(|check| check.outcome)
            .collect();
        assert_eq!(
            outcomes,
            [
                CheckOutcome::Passed,
                CheckOutcome::Skipped,
                CheckOutcome::Queued
            ]
        );
    }

    /// Other apps, commit statuses and unidentifiable runs are never discarded.
    #[test]
    fn checks_without_workflow_run_ids_are_preserved() {
        let mut unidentified = workflow_job("workflow", 10, "Result", "FAILURE");
        unidentified["checkSuite"]["workflowRun"]["databaseId"] = serde_json::Value::Null;
        let contexts: CheckContexts = serde_json::from_value(json!([
            null,
            unidentified,
            workflow_job("workflow", 20, "Result", "SUCCESS"),
            {"__typename": "CheckRun", "name": "Result", "status": "COMPLETED",
             "conclusion": "FAILURE", "isRequired": true,
             "checkSuite": {"app": {"name": "Another app"}}},
            {"__typename": "StatusContext", "context": "Result", "state": "ERROR",
             "isRequired": true}
        ]))
        .unwrap();
        assert_eq!(contexts.unreadable_count(), 1);
        let checks = contexts.into_checks();
        assert_eq!(checks.len(), 4);
        assert_eq!(checks[0].outcome, CheckOutcome::Failed);
        assert_eq!(checks[2].source, CheckSource::App("Another app".to_owned()));
        assert_eq!(checks[2].outcome, CheckOutcome::Failed);
        assert_eq!(checks[3].source, CheckSource::CommitStatus);
        assert_eq!(checks[3].outcome, CheckOutcome::Failed);
    }
}
