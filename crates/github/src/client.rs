use serde::de::DeserializeOwned;
use ureq::Agent;

use crate::models::{SearchIssue, SearchResponse};
use crate::{Error, PullRequest, PullRequestFile, RepoRef, Result, User};

const API_BASE: &str = "https://api.github.com";
const USER_AGENT: &str = concat!("quick-check/", env!("CARGO_PKG_VERSION"));

/// Blocking GitHub REST client. Cheap to clone.
#[derive(Clone)]
pub struct Client {
    agent: Agent,
    token: String,
    base: String,
}

impl Client {
    pub fn new(token: impl Into<String>) -> Self {
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            agent,
            token: token.into(),
            base: API_BASE.to_owned(),
        }
    }

    /// Build a client using [`crate::resolve_token`].
    pub fn from_env() -> Result<Self> {
        Ok(Self::new(crate::resolve_token()?))
    }

    /// The authenticated user.
    pub fn viewer(&self) -> Result<User> {
        self.get("/user", &[])
    }

    /// Open PRs where the viewer's review is requested.
    pub fn review_requested(&self) -> Result<Vec<PullRequest>> {
        self.search_pull_requests("is:pr is:open archived:false review-requested:@me")
    }

    /// Run a PR search query (first page, up to 100 results, newest update first).
    pub fn search_pull_requests(&self, query: &str) -> Result<Vec<PullRequest>> {
        let resp: SearchResponse<SearchIssue> = self.get(
            "/search/issues",
            &[
                ("q", query),
                ("sort", "updated"),
                ("order", "desc"),
                ("per_page", "100"),
            ],
        )?;
        Ok(resp
            .items
            .into_iter()
            .filter_map(SearchIssue::into_pull_request)
            .collect())
    }

    /// Changed files (with patches) of a pull request (first 100 files).
    pub fn pull_request_files(&self, repo: &RepoRef, number: u64) -> Result<Vec<PullRequestFile>> {
        let path = format!("/repos/{}/{}/pulls/{number}/files", repo.owner, repo.name);
        self.get(&path, &[("per_page", "100")])
    }

    fn get<T: DeserializeOwned>(&self, path: &str, query: &[(&str, &str)]) -> Result<T> {
        let mut req = self
            .agent
            .get(format!("{}{path}", self.base))
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", USER_AGENT)
            .header("X-GitHub-Api-Version", "2022-11-28");
        for (k, v) in query {
            req = req.query(*k, *v);
        }

        let mut resp = req.call()?;
        let status = resp.status().as_u16();
        let body = resp.body_mut().read_to_string()?;
        if !(200..300).contains(&status) {
            return Err(Error::Api { status, body });
        }
        Ok(serde_json::from_str(&body)?)
    }
}
