use std::sync::{Arc, OnceLock};

use serde::de::DeserializeOwned;
use ureq::Agent;
use ureq::RequestBuilder;
use ureq::http::StatusCode;

use crate::graphql_response::GraphQlResponse;
use crate::http_response::HttpResponse;
use crate::pull_request_revision::PullRequestRefs;
use crate::pull_request_stack::StackResponse;
use crate::search_issue::SearchIssue;
use crate::{
    ApiUsage, ConditionalResponse, DownloadProgress, ETag, GitHubError, PullRequest,
    PullRequestStack, RepoRef, RevisionCheck,
};

pub(crate) const API_BASE: &str = "https://api.github.com";
pub(crate) const API_VERSION: &str = "2022-11-28";
const USER_AGENT: &str = concat!("quarterdeck/", env!("CARGO_PKG_VERSION"));
/// The JSON answers of the REST API.
pub(crate) const JSON_MEDIA_TYPE: &str = "application/vnd.github+json";
/// A blob's content as it is, not base64 in JSON.
pub(crate) const RAW_MEDIA_TYPE: &str = "application/vnd.github.raw+json";
/// GitHub's maximum page size.
pub(crate) const PAGE_SIZE: &str = "100";

/// Blocking, authenticated GitHub REST client. Cheap to clone: clones share
/// the connection pool and the usage counts.
#[derive(Clone)]
pub struct GitHubClient {
    agent: Agent,
    token: String,
    usage: ApiUsage,
    /// The viewer's login once asked for; shared by clones, so it is asked once.
    viewer_login: Arc<OnceLock<String>>,
}

impl GitHubClient {
    /// A client that sends `token` as a bearer token.
    pub fn new(token: impl Into<String>) -> Self {
        // Non-success statuses come back as responses so their body can be reported.
        // A renamed repository redirects within the API host: the token goes along,
        // so a private one answers instead of a 404.
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .redirect_auth_headers(ureq::config::RedirectAuthHeaders::SameHost)
            .build()
            .into();
        Self {
            agent,
            token: token.into(),
            usage: ApiUsage::default(),
            viewer_login: Arc::default(),
        }
    }

    /// The token requests are sent with, for git to fetch commits of
    /// private repositories with.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// The login known for the token, once [`Self::viewer_login`] asked.
    pub(crate) fn known_viewer_login(&self) -> &OnceLock<String> {
        &self.viewer_login
    }

    /// The requests this client (and its clones) sent and the budgets left.
    pub fn usage(&self) -> &ApiUsage {
        &self.usage
    }

    /// The HTTP agent, for requests built in sibling files.
    pub(crate) fn agent(&self) -> &Agent {
        &self.agent
    }

    /// Sends `builder` (a PUT, PATCH or POST) with a JSON body under API
    /// version `api_version`, and returns the answer's body.
    pub(crate) fn send_json(
        &self,
        builder: RequestBuilder<ureq::typestate::WithBody>,
        body: &serde_json::Value,
        api_version: &str,
    ) -> Result<String, GitHubError> {
        let builder = self.with_versioned_headers(builder, JSON_MEDIA_TYPE, None, api_version);
        Ok(self
            .read_response(builder.send_json(body)?, &mut |_| {})?
            .body)
    }

    /// Sends a GET under API version `api_version` and returns the body.
    pub(crate) fn send_versioned_get(
        &self,
        url: &str,
        api_version: &str,
    ) -> Result<String, GitHubError> {
        let builder =
            self.with_versioned_headers(self.agent.get(url), JSON_MEDIA_TYPE, None, api_version);
        Ok(self.read_response(builder.call()?, &mut |_| {})?.body)
    }

    /// Sends a DELETE.
    pub(crate) fn send_delete(&self, url: &str) -> Result<(), GitHubError> {
        let builder = self.with_headers(self.agent.delete(url), JSON_MEDIA_TYPE, None);
        self.read_response(builder.call()?, &mut |_| {})?;
        Ok(())
    }

    /// Sends a GraphQL query or mutation and decodes its data; GraphQL
    /// errors (reported with status 200) become [`GitHubError::Api`].
    pub(crate) fn graphql<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: &serde_json::Value,
    ) -> Result<T, GitHubError> {
        self.graphql_answer(query, variables)?
            .into_data()
            .map_err(graphql_error)
    }

    /// [`Self::graphql`], keeping data GitHub sent without the parts the
    /// token may not read (`FORBIDDEN` errors) instead of failing.
    pub(crate) fn graphql_allowing_forbidden<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: &serde_json::Value,
    ) -> Result<T, GitHubError> {
        self.graphql_answer(query, variables)?
            .into_data_or_forbidden()
            .map_err(graphql_error)
    }

    /// Sends a GraphQL query or mutation and decodes the whole answer.
    fn graphql_answer<T: DeserializeOwned>(
        &self,
        query: &str,
        variables: &serde_json::Value,
    ) -> Result<GraphQlResponse<T>, GitHubError> {
        let request = serde_json::json!({ "query": query, "variables": variables });
        let builder = self.with_headers(
            self.agent.post(format!("{API_BASE}/graphql")),
            JSON_MEDIA_TYPE,
            None,
        );
        let response = self.read_response(builder.send_json(&request)?, &mut |_| {})?;
        Ok(serde_json::from_str(&response.body)?)
    }

    /// A pull request by number, shaped like the search results (same id),
    /// for a stack layer that is in neither list. One request.
    pub fn pull_request_by_number(
        &self,
        repo: &RepoRef,
        number: u64,
    ) -> Result<PullRequest, GitHubError> {
        let issue: SearchIssue =
            self.get_json(&format!("{}/issues/{number}", repo.api_url()), &[])?;
        issue.into_pull_request().ok_or_else(|| GitHubError::Api {
            status: StatusCode::OK.as_u16(),
            body: format!("issue {number} has no repository URL"),
        })
    }

    /// The head and base commits of a pull request and its file count, unless
    /// unchanged since the response tagged `etag` (then it is free).
    pub fn pull_request_revision(
        &self,
        repo: &RepoRef,
        number: u64,
        etag: Option<&ETag>,
    ) -> Result<ConditionalResponse<RevisionCheck>, GitHubError> {
        let url = format!("{}/pulls/{number}", repo.api_url());
        let refs = self.get_conditional::<PullRequestRefs>(&url, etag)?;
        Ok(refs.map(PullRequestRefs::into_check))
    }

    /// A stack with all its layers, unless unchanged since the response
    /// tagged `etag` (then it is free).
    pub fn pull_request_stack(
        &self,
        repo: &RepoRef,
        number: u64,
        etag: Option<&ETag>,
    ) -> Result<ConditionalResponse<PullRequestStack>, GitHubError> {
        let url = format!("{}/stacks/{number}", repo.api_url());
        let stack = self.get_conditional::<StackResponse>(&url, etag)?;
        Ok(stack.map(|stack| stack.into_stack(repo)))
    }

    /// Sends a GET conditional on `etag` (when given) and decodes the JSON
    /// body; `304 Not Modified` is [`ConditionalResponse::NotModified`],
    /// which GitHub does not count against the rate limit.
    fn get_conditional<T: DeserializeOwned>(
        &self,
        url: &str,
        etag: Option<&ETag>,
    ) -> Result<ConditionalResponse<T>, GitHubError> {
        let response = self.get(url, &[], etag, JSON_MEDIA_TYPE, &mut |_| {})?;
        if response.status == StatusCode::NOT_MODIFIED {
            return Ok(ConditionalResponse::NotModified);
        }
        Ok(ConditionalResponse::Modified {
            value: serde_json::from_str(&response.body)?,
            etag: response.etag,
        })
    }

    /// Sends an authenticated GET and decodes the JSON body.
    pub(crate) fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        query: &[(&str, &str)],
    ) -> Result<T, GitHubError> {
        let response = self.get(url, query, None, JSON_MEDIA_TYPE, &mut |_| {})?;
        Ok(serde_json::from_str(&response.body)?)
    }

    /// Sends an authenticated GET for `accept`, conditional on `etag` when
    /// given, and reads the answer whole.
    pub(crate) fn get(
        &self,
        url: &str,
        query: &[(&str, &str)],
        etag: Option<&ETag>,
        accept: &str,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<HttpResponse, GitHubError> {
        let mut request = self.with_headers(self.agent.get(url), accept, etag);
        for (key, value) in query {
            request = request.query(*key, *value);
        }
        self.read_response(request.call()?, on_progress)
    }

    /// Adds the headers GitHub requires for REST v3, the media type wanted
    /// and, for a conditional request, the tag of the last answer.
    fn with_headers<B>(
        &self,
        request: RequestBuilder<B>,
        accept: &str,
        etag: Option<&ETag>,
    ) -> RequestBuilder<B> {
        self.with_versioned_headers(request, accept, etag, API_VERSION)
    }

    /// [`Self::with_headers`] under API version `api_version`.
    fn with_versioned_headers<B>(
        &self,
        request: RequestBuilder<B>,
        accept: &str,
        etag: Option<&ETag>,
        api_version: &str,
    ) -> RequestBuilder<B> {
        let request = request
            .header("Accept", accept)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", USER_AGENT)
            .header("X-GitHub-Api-Version", api_version);
        match etag {
            Some(etag) => request.header("If-None-Match", etag.as_str()),
            None => request,
        }
    }
}

/// A GraphQL failure (reported with status 200) as an API error.
pub(crate) fn graphql_error(message: String) -> GitHubError {
    GitHubError::Api {
        status: StatusCode::OK.as_u16(),
        body: message,
    }
}
