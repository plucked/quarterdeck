use serde::de::DeserializeOwned;
use ureq::Agent;

use crate::search_issue::SearchIssue;
use crate::search_response::SearchResponse;
use crate::{GitHubError, PullRequest, PullRequestFile, RepoRef};

const API_BASE: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";
const USER_AGENT: &str = concat!("quick-check/", env!("CARGO_PKG_VERSION"));
/// GitHub's maximum page size; only the first page is read for now.
const SEARCH_PAGE_SIZE: &str = "100";
const REVIEW_REQUESTED_QUERY: &str = "is:pr is:open archived:false review-requested:@me";

/// Blocking, authenticated GitHub REST client. Cheap to clone: clones share
/// the connection pool.
#[derive(Clone)]
pub struct GitHubClient {
    agent: Agent,
    token: String,
}

impl GitHubClient {
    /// A client that sends `token` as a bearer token.
    pub fn new(token: impl Into<String>) -> Self {
        // Non-success statuses come back as responses so their body can be reported.
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            token: token.into(),
        }
    }

    /// A client using the token [`crate::resolve_token`] finds.
    pub fn from_env() -> Result<Self, GitHubError> {
        Ok(Self::new(crate::resolve_token()?))
    }

    /// Open pull requests that request the viewer's review, most recently updated first.
    pub fn review_requested(&self) -> Result<Vec<PullRequest>, GitHubError> {
        let response: SearchResponse<SearchIssue> = self.get_json(
            "/search/issues",
            &[
                ("q", REVIEW_REQUESTED_QUERY),
                ("sort", "updated"),
                ("order", "desc"),
                ("per_page", SEARCH_PAGE_SIZE),
            ],
        )?;
        Ok(response
            .items
            .into_iter()
            .filter_map(SearchIssue::into_pull_request)
            .collect())
    }

    /// The files a pull request changes, with their patches (first page only).
    pub fn pull_request_files(
        &self,
        repo: &RepoRef,
        number: u64,
    ) -> Result<Vec<PullRequestFile>, GitHubError> {
        let path = format!("/repos/{}/{}/pulls/{number}/files", repo.owner, repo.name);
        self.get_json(&path, &[("per_page", SEARCH_PAGE_SIZE)])
    }

    /// Sends an authenticated GET to `path` and decodes the JSON body.
    fn get_json<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, GitHubError> {
        // Build the request with the headers GitHub requires for REST v3.
        let mut request = self
            .agent
            .get(format!("{API_BASE}{path}"))
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", USER_AGENT)
            .header("X-GitHub-Api-Version", API_VERSION);
        for (key, value) in query {
            request = request.query(*key, *value);
        }

        // Read the whole body first so an error status can carry GitHub's message.
        let mut response = request.call()?;
        let status = response.status().as_u16();
        let body = response.body_mut().read_to_string()?;
        if !response.status().is_success() {
            return Err(GitHubError::Api { status, body });
        }
        Ok(serde_json::from_str(&body)?)
    }
}
