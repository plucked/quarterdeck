use ureq::http::StatusCode;

use crate::github_client::graphql_error;
use crate::merge_queries::{MERGE_OPTIONS_QUERY, MergeAsyncResponse, MergeOptionsData};
use crate::{GitHubClient, GitHubError, MergeMethod, MergeOptions, MergeOutcome, RepoRef};

/// The API version of the asynchronous merge endpoints.
const ASYNC_MERGE_API_VERSION: &str = "2026-03-10";

impl GitHubClient {
    /// What merging or closing pull request `number` can do now: one GraphQL request.
    pub fn merge_options(&self, repo: &RepoRef, number: u64) -> Result<MergeOptions, GitHubError> {
        let data: MergeOptionsData = self.graphql(
            MERGE_OPTIONS_QUERY,
            &serde_json::json!({ "owner": repo.owner, "name": repo.name, "number": number }),
        )?;
        data.into_options().map_err(graphql_error)
    }

    /// Starts merging pull request `number` at `head_sha` with `method`
    /// (asynchronously: the only way to merge stacked pull requests, which
    /// merges every layer below too; a merge queue takes it when the base
    /// uses one). `bypass_rules` merges past requirements the viewer may
    /// bypass (an admin merge). Another merge already pending is joined.
    pub fn start_merge(
        &self,
        repo: &RepoRef,
        number: u64,
        method: MergeMethod,
        head_sha: &str,
        bypass_rules: bool,
    ) -> Result<MergeOutcome, GitHubError> {
        let url = format!("{}/pulls/{number}/merge-async", repo.api_url());
        let body = serde_json::json!({
            "merge_method": method.as_rest(),
            "sha": head_sha,
            "bypass_rules": bypass_rules,
        });
        let answer = match self.send_json(self.agent().put(&url), &body, ASYNC_MERGE_API_VERSION) {
            Ok(answer) => answer,
            // A merge already pending answers 409 with that request: follow it.
            Err(GitHubError::Api { status, body }) if status == StatusCode::CONFLICT.as_u16() => {
                body
            }
            Err(error) => return Err(error),
        };
        let response: MergeAsyncResponse = serde_json::from_str(&answer)?;
        Ok(response.into_outcome())
    }

    /// How the asynchronous merge request `uuid` stands.
    pub fn merge_outcome(
        &self,
        repo: &RepoRef,
        number: u64,
        uuid: &str,
    ) -> Result<MergeOutcome, GitHubError> {
        let url = format!("{}/pulls/{number}/merge-async/{uuid}", repo.api_url());
        let answer = self.send_versioned_get(&url, ASYNC_MERGE_API_VERSION)?;
        let response: MergeAsyncResponse = serde_json::from_str(&answer)?;
        Ok(response.into_outcome())
    }

    /// Closes pull request `number` without merging it.
    pub fn close_pull_request(&self, repo: &RepoRef, number: u64) -> Result<(), GitHubError> {
        let url = format!("{}/pulls/{number}", repo.api_url());
        self.send_json(
            self.agent().patch(&url),
            &serde_json::json!({ "state": "closed" }),
            crate::github_client::API_VERSION,
        )?;
        Ok(())
    }

    /// Deletes branch `branch` of `repo`.
    pub fn delete_branch(&self, repo: &RepoRef, branch: &str) -> Result<(), GitHubError> {
        let dependents: Vec<serde_json::Value> = self.get_json(
            &format!("{}/pulls", repo.api_url()),
            &[("state", "open"), ("base", branch), ("per_page", "1")],
        )?;
        if !dependents.is_empty() {
            return Err(graphql_error(
                "An open pull request is based on this branch.".into(),
            ));
        }
        let url = branch_ref_url(repo, branch);
        self.send_delete(&url)
    }
}

/// Keep branch hierarchy intact while escaping URL delimiters inside each segment.
fn branch_ref_url(repo: &RepoRef, branch: &str) -> String {
    use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
    const REF_PATH: &percent_encoding::AsciiSet = &NON_ALPHANUMERIC
        .remove(b'/')
        .remove(b'-')
        .remove(b'_')
        .remove(b'.')
        .remove(b'~');
    format!(
        "{}/git/refs/heads/{}",
        repo.api_url(),
        utf8_percent_encode(branch, REF_PATH)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deletion_uses_the_entire_checked_branch_name() {
        let repo = RepoRef::parse_input("owner/repo").unwrap();
        assert_eq!(
            branch_ref_url(&repo, "feature/100%#ready"),
            "https://api.github.com/repos/owner/repo/git/refs/heads/feature/100%25%23ready"
        );
    }
}
