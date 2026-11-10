//! The GraphQL query of the checks on a pull request's head commit, one
//! page of 100 at a time, and its answer.

use serde::Deserialize;

use crate::{CheckOutcome, CheckSource, StatusCheck};

/// Checks per page: GraphQL's largest.
pub(crate) const CHECKS_PAGE_SIZE: u32 = 100;

/// The head commit's rollup and one page of its checks (`$after` is the
/// previous page's end, `null` for the first), and the checks the base
/// branch requires (its branch protection rule and the rulesets that apply
/// to it), so the ones never reported can be shown. Workflow names come
/// with the check runs, so a page is one request (one point).
pub(crate) const STATUS_CHECKS_QUERY: &str = r"
query StatusChecks($owner: String!, $name: String!, $number: Int!, $pageSize: Int!, $after: String) {
  repository(owner: $owner, name: $name) {
    pullRequest(number: $number) {
      mergeable baseRefName
      baseRef {
        branchProtectionRule { requiredStatusChecks { context } }
        rules(first: 100) {
          nodes {
            parameters {
              ... on RequiredStatusChecksParameters { requiredStatusChecks { context } }
            }
          }
        }
      }
      commits(last: 1) {
        nodes {
          commit {
            oid
            statusCheckRollup {
              state
              contexts(first: $pageSize, after: $after) {
                totalCount
                pageInfo { hasNextPage endCursor }
                nodes {
                  __typename
                  ... on CheckRun {
                    name status conclusion detailsUrl startedAt completedAt
                    isRequired(pullRequestNumber: $number)
                    checkSuite { app { name } workflowRun { workflow { name } } }
                  }
                  ... on StatusContext {
                    context state targetUrl createdAt
                    isRequired(pullRequestNumber: $number)
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}
";

/// The data of [`STATUS_CHECKS_QUERY`].
#[derive(Deserialize)]
pub(crate) struct StatusChecksData {
    repository: Option<RepositoryNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepositoryNode {
    pull_request: Option<PullRequestNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PullRequestNode {
    /// `MERGEABLE`, `CONFLICTING` or `UNKNOWN` (not worked out yet).
    mergeable: String,
    base_ref_name: String,
    /// `None` when the base branch is gone.
    base_ref: Option<BaseRefNode>,
    commits: CommitConnection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BaseRefNode {
    /// `None` without a classic branch protection rule, or when the token
    /// may not read it.
    branch_protection_rule: Option<RequiredChecksNode>,
    rules: Option<RuleConnection>,
}

#[derive(Deserialize)]
struct RuleConnection {
    nodes: Vec<Option<RuleNode>>,
}

#[derive(Deserialize)]
struct RuleNode {
    /// Only a required status checks rule has the fields asked for; other
    /// rules come as `{}` or `null`.
    parameters: Option<RequiredChecksNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequiredChecksNode {
    #[serde(default)]
    required_status_checks: Option<Vec<RequiredCheckNode>>,
}

#[derive(Deserialize)]
struct RequiredCheckNode {
    context: String,
}

impl BaseRefNode {
    /// The names of the checks the branch requires, each once, rules first.
    fn required_contexts(self) -> Vec<String> {
        let from_rules = self
            .rules
            .into_iter()
            .flat_map(|rules| rules.nodes)
            .flatten()
            .filter_map(|rule| rule.parameters);
        let mut contexts: Vec<String> = Vec::new();
        for required in from_rules.chain(self.branch_protection_rule) {
            for check in required.required_status_checks.into_iter().flatten() {
                if !contexts.contains(&check.context) {
                    contexts.push(check.context);
                }
            }
        }
        contexts
    }
}

#[derive(Deserialize)]
struct CommitConnection {
    nodes: Vec<CommitNode>,
}

#[derive(Deserialize)]
struct CommitNode {
    commit: Commit,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Commit {
    oid: String,
    /// `None` when the commit has no checks at all.
    status_check_rollup: Option<Rollup>,
}

#[derive(Deserialize)]
struct Rollup {
    state: String,
    contexts: ContextConnection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContextConnection {
    total_count: usize,
    page_info: PageInfo,
    /// `None` entries are checks the token may not read.
    nodes: Vec<Option<ContextNode>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageInfo {
    has_next_page: bool,
    end_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "__typename")]
enum ContextNode {
    CheckRun(CheckRunNode),
    StatusContext(StatusContextNode),
}

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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CheckSuiteNode {
    app: Option<NameNode>,
    workflow_run: Option<WorkflowRunNode>,
}

#[derive(Deserialize)]
struct WorkflowRunNode {
    workflow: NameNode,
}

#[derive(Deserialize)]
struct NameNode {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusContextNode {
    context: String,
    state: String,
    target_url: Option<String>,
    created_at: Option<String>,
    is_required: bool,
}

/// One page of a head commit's checks, as read.
pub(crate) struct StatusChecksPage {
    /// Whether the pull request can merge into its base, as GitHub's
    /// `mergeable` says (`MERGEABLE`, `CONFLICTING` or `UNKNOWN`).
    pub mergeable: String,
    /// The branch the pull request targets.
    pub base_ref: String,
    /// The names of the checks the base branch requires.
    pub required_contexts: Vec<String>,
    pub head_sha: String,
    /// GitHub's verdict over all checks; `None` when there are none.
    pub rollup: Option<CheckOutcome>,
    pub total_count: usize,
    /// The readable checks of the page.
    pub checks: Vec<StatusCheck>,
    /// How many checks of the page the token may not read.
    pub unreadable_count: usize,
    /// Where the next page starts, when there is one.
    pub next_cursor: Option<String>,
}

impl StatusChecksData {
    /// The page, or why there is none (no such pull request or commit).
    pub fn into_page(self) -> Result<StatusChecksPage, String> {
        let pull_request = self
            .repository
            .ok_or("GitHub knows no such repository")?
            .pull_request
            .ok_or("GitHub knows no such pull request")?;
        let (mergeable, base_ref) = (pull_request.mergeable, pull_request.base_ref_name);
        let required_contexts = pull_request
            .base_ref
            .map(BaseRefNode::required_contexts)
            .unwrap_or_default();
        let commit = pull_request
            .commits
            .nodes
            .into_iter()
            .next()
            .ok_or("the pull request has no commits")?
            .commit;
        let Some(rollup) = commit.status_check_rollup else {
            return Ok(StatusChecksPage {
                mergeable,
                base_ref,
                required_contexts,
                head_sha: commit.oid,
                rollup: None,
                total_count: 0,
                checks: Vec::new(),
                unreadable_count: 0,
                next_cursor: None,
            });
        };

        // Readable entries become checks; withheld ones are only counted.
        let contexts = rollup.contexts;
        let unreadable_count = contexts.nodes.iter().filter(|node| node.is_none()).count();
        let checks = contexts
            .nodes
            .into_iter()
            .flatten()
            .map(ContextNode::into_check)
            .collect();
        let next_cursor = contexts
            .page_info
            .end_cursor
            .filter(|_| contexts.page_info.has_next_page);
        Ok(StatusChecksPage {
            mergeable,
            base_ref,
            required_contexts,
            head_sha: commit.oid,
            rollup: Some(CheckOutcome::from_status_state(&rollup.state)),
            total_count: contexts.total_count,
            checks,
            unreadable_count,
            next_cursor,
        })
    }
}

impl ContextNode {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphql_response::{GraphQlData, GraphQlResponse};

    /// A page with a workflow job, another app's check and a commit status.
    const MIXED_PAGE: &str = r#"{"data":{"repository":{"pullRequest":{"mergeable":"MERGEABLE","baseRefName":"main",
        "baseRef":{"branchProtectionRule":{"requiredStatusChecks":[{"context":"Unit Tests / test"},{"context":"lfs"}]},
          "rules":{"nodes":[
            {"parameters":{"requiredStatusChecks":[{"context":"lfs"},{"context":"Build Result"}]}},
            {"parameters":{}},{"parameters":null}]}},
        "commits":{"nodes":[{"commit":{
        "oid":"abc123",
        "statusCheckRollup":{"state":"PENDING","contexts":{
          "totalCount":3,
          "pageInfo":{"hasNextPage":false,"endCursor":"Mw"},
          "nodes":[
            {"__typename":"CheckRun","name":"Unit Tests / test","status":"IN_PROGRESS","conclusion":null,
             "detailsUrl":"https://github.com/o/r/actions/runs/1/job/2","startedAt":"2026-11-09T15:32:36Z",
             "completedAt":null,"isRequired":true,
             "checkSuite":{"app":{"name":"GitHub Actions"},"workflowRun":{"workflow":{"name":"CI"}}}},
            {"__typename":"CheckRun","name":"Preview","status":"COMPLETED","conclusion":"FAILURE",
             "detailsUrl":null,"startedAt":null,"completedAt":null,"isRequired":false,
             "checkSuite":{"app":{"name":"Vercel"},"workflowRun":null}},
            {"__typename":"StatusContext","context":"ci/legacy","state":"SUCCESS",
             "targetUrl":"https://ci.example.com/1","createdAt":"2026-11-09T15:30:00Z","isRequired":false}
          ]}}}}]}}}}}"#;

    /// A page whose checks the token may not read.
    const FORBIDDEN_PAGE: &str = r#"{"data":{"repository":{"pullRequest":{"mergeable":"MERGEABLE","baseRefName":"main","commits":{"nodes":[{"commit":{
        "oid":"abc123",
        "statusCheckRollup":{"state":"FAILURE","contexts":{
          "totalCount":148,"pageInfo":{"hasNextPage":true,"endCursor":"MTAw"},"nodes":[null,null]}}}}]}}}},
        "errors":[
          {"type":"FORBIDDEN","message":"Resource not accessible by personal access token"},
          {"type":"FORBIDDEN","message":"Resource not accessible by personal access token"}]}"#;

    /// Decodes `json` as the query's answer, withheld parts allowed.
    fn page_of(json: &str) -> (StatusChecksPage, bool) {
        let answer: GraphQlResponse<StatusChecksData> = serde_json::from_str(json).unwrap();
        match answer.into_data_or_forbidden().unwrap() {
            GraphQlData::Complete(data) => (data.into_page().unwrap(), false),
            GraphQlData::PartlyForbidden(data) => (data.into_page().unwrap(), true),
        }
    }

    /// Check runs name their workflow or app; a commit status is its own source.
    #[test]
    fn checks_keep_their_source_and_outcome() {
        let (page, is_forbidden) = page_of(MIXED_PAGE);
        assert!(!is_forbidden);
        assert_eq!(page.head_sha, "abc123");
        assert_eq!(page.rollup, Some(CheckOutcome::Running));
        assert_eq!(page.next_cursor, None);
        let summary: Vec<_> = page
            .checks
            .iter()
            .map(|check| (check.name.as_str(), check.source.clone(), check.outcome))
            .collect();
        assert_eq!(
            summary,
            [
                (
                    "Unit Tests / test",
                    CheckSource::Workflow("CI".to_owned()),
                    CheckOutcome::Running
                ),
                (
                    "Preview",
                    CheckSource::App("Vercel".to_owned()),
                    CheckOutcome::Failed
                ),
                ("ci/legacy", CheckSource::CommitStatus, CheckOutcome::Passed),
            ]
        );
        assert!(page.checks[0].is_required);
        assert_eq!(
            page.required_contexts,
            ["lfs", "Build Result", "Unit Tests / test"]
        );
    }

    /// Withheld checks leave the rollup, the count and the next page readable.
    #[test]
    fn forbidden_checks_keep_the_rollup() {
        let (page, is_forbidden) = page_of(FORBIDDEN_PAGE);
        assert!(is_forbidden);
        assert_eq!(page.rollup, Some(CheckOutcome::Failed));
        assert_eq!(page.total_count, 148);
        assert_eq!(page.unreadable_count, 2);
        assert_eq!(page.checks.len(), 0);
        assert_eq!(page.next_cursor.as_deref(), Some("MTAw"));
    }

    /// A commit without any check has no rollup.
    #[test]
    fn a_commit_without_checks_has_no_rollup() {
        let json = r#"{"data":{"repository":{"pullRequest":{"mergeable":"CONFLICTING","baseRefName":"develop",
            "commits":{"nodes":[{"commit":{"oid":"abc123","statusCheckRollup":null}}]}}}}}"#;
        let (page, _) = page_of(json);
        assert_eq!(page.rollup, None);
        assert_eq!(page.total_count, 0);
        assert_eq!(page.mergeable, "CONFLICTING");
        assert_eq!(page.base_ref, "develop");
    }
}
