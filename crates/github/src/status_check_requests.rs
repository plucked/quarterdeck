use crate::expected_checks::{RequiredMarks, add_expected_checks};
use crate::github_client::graphql_error;
use crate::graphql_response::GraphQlData;
use crate::layer_conflicts_query::{LayerConflictsData, layer_conflicts_query};
use crate::status_checks_query::{
    CHECKS_PAGE_SIZE, STATUS_CHECKS_QUERY, StatusChecksData, StatusChecksPage,
};
use crate::{
    CheckDetails, ConflictLocation, GitHubClient, GitHubError, MergeConflict, PullRequestChecks,
    RepoRef,
};

/// Pages of checks read at most: 1000 checks.
const MAX_CHECK_PAGES: usize = 10;
/// The statuses a token gets for what it may not read.
const FORBIDDEN: u16 = 403;
const NOT_FOUND: u16 = 404;
/// GitHub's `mergeable` values the conflict lookup reads.
const MERGEABLE: &str = "MERGEABLE";
const CONFLICTING: &str = "CONFLICTING";

impl GitHubClient {
    /// The checks on pull request `number`'s head commit: one GraphQL
    /// request per 100 checks. A token that may not read them (a
    /// fine-grained token without the Checks permission) gets the commit's
    /// GitHub Actions jobs and statuses instead ([`CheckDetails::FromActions`]),
    /// or, without the Actions permission either, the rollup and count only
    /// ([`CheckDetails::NotPermitted`]).
    ///
    /// Also says whether a merge conflict keeps workflows from running: the
    /// pull request's own, or, while GitHub has not worked out its own,
    /// the lowest of `layers_below` (open stack layers under it, bottom
    /// first) that conflicts; that look is one more GraphQL request.
    pub fn pull_request_checks(
        &self,
        repo: &RepoRef,
        number: u64,
        layers_below: &[u64],
    ) -> Result<PullRequestChecks, GitHubError> {
        let mut checks = Vec::new();
        let mut after: Option<String> = None;
        for _ in 0..MAX_CHECK_PAGES {
            // Read one page; withheld checks come back as gaps, not as a failure.
            let data: GraphQlData<StatusChecksData> = self.graphql_allowing_forbidden(
                STATUS_CHECKS_QUERY,
                &serde_json::json!({
                    "owner": repo.owner,
                    "name": repo.name,
                    "number": number,
                    "pageSize": CHECKS_PAGE_SIZE,
                    "after": after,
                }),
            )?;
            let data = match data {
                GraphQlData::Complete(data) | GraphQlData::PartlyForbidden(data) => data,
            };
            let mut page = data.into_page().map_err(graphql_error)?;

            // A token that cannot read one check reads none: ask GitHub Actions instead.
            if page.unreadable_count > 0 {
                let details = match self.workflow_job_checks(repo, &page.head_sha) {
                    Ok(checks) => CheckDetails::FromActions(checks),
                    Err(GitHubError::Api {
                        status: FORBIDDEN | NOT_FOUND,
                        ..
                    }) => CheckDetails::NotPermitted,
                    Err(error) => return Err(error),
                };
                return self.finish_checks(repo, number, layers_below, page, details);
            }
            checks.append(&mut page.checks);
            after.clone_from(&page.next_cursor);
            if after.is_none() {
                return self.finish_checks(
                    repo,
                    number,
                    layers_below,
                    page,
                    CheckDetails::Listed(checks),
                );
            }
        }
        Err(graphql_error(format!(
            "more than {MAX_CHECK_PAGES} pages of checks"
        )))
    }

    /// The checks read, with the rollup of the last page and the merge
    /// conflict that keeps workflows from running, if any; the checks the
    /// base branch requires are marked, and those not reported added.
    fn finish_checks(
        &self,
        repo: &RepoRef,
        number: u64,
        layers_below: &[u64],
        page: StatusChecksPage,
        details: CheckDetails,
    ) -> Result<PullRequestChecks, GitHubError> {
        let details = match details {
            CheckDetails::Listed(mut checks) => {
                add_expected_checks(&mut checks, &page.required_contexts, RequiredMarks::AsRead);
                CheckDetails::Listed(checks)
            }
            CheckDetails::FromActions(mut checks) => {
                add_expected_checks(&mut checks, &page.required_contexts, RequiredMarks::ByName);
                CheckDetails::FromActions(checks)
            }
            CheckDetails::NotPermitted => CheckDetails::NotPermitted,
        };
        let conflict = match page.mergeable.as_str() {
            CONFLICTING => Some(MergeConflict {
                number,
                base_ref: page.base_ref,
                location: ConflictLocation::Shown,
            }),
            MERGEABLE => None,
            _ => self.conflicting_layer(repo, layers_below)?,
        };
        Ok(PullRequestChecks {
            head_sha: page.head_sha,
            rollup: page.rollup,
            total_count: page.total_count,
            details,
            conflict,
        })
    }

    /// The lowest of `layers_below` that conflicts with its base: nothing
    /// to ask without layers, else one GraphQL request.
    fn conflicting_layer(
        &self,
        repo: &RepoRef,
        layers_below: &[u64],
    ) -> Result<Option<MergeConflict>, GitHubError> {
        if layers_below.is_empty() {
            return Ok(None);
        }
        let data: LayerConflictsData = self.graphql(
            &layer_conflicts_query(layers_below),
            &serde_json::json!({ "owner": repo.owner, "name": repo.name }),
        )?;
        Ok(data
            .first_conflicting(layers_below)
            .map(|layer| MergeConflict {
                number: layer.number,
                base_ref: layer.base_ref,
                location: ConflictLocation::LayerBelow,
            }))
    }
}
