use std::io::Read;
use std::sync::{Arc, OnceLock};

use serde::de::DeserializeOwned;
use ureq::Agent;
use ureq::http::{HeaderMap, Response, StatusCode};
use ureq::{Body, RequestBuilder};

use crate::graphql_response::GraphQlResponse;
use crate::listed_details_query::{LISTED_DETAILS_QUERY, ListedDetailsData};
use crate::next_page_url::next_page_url;
use crate::other_pull_requests::others_search_queries;
use crate::pull_request_revision::PullRequestRefs;
use crate::pull_request_stack::StackResponse;
use crate::search_issue::SearchIssue;
use crate::search_response::SearchResponse;
use crate::unified_diff::files_of_unified_diff;
use crate::{
    ApiUsage, ConditionalResponse, DownloadProgress, ETag, GitHubError, ListedPullRequestDetails,
    PullRequest, PullRequestFile, PullRequestStack, RateLimit, RateLimitResource, RepoRef,
    RevisionCheck,
};

pub(crate) const API_BASE: &str = "https://api.github.com";
pub(crate) const API_VERSION: &str = "2022-11-28";
const USER_AGENT: &str = concat!("quarterdeck/", env!("CARGO_PKG_VERSION"));
/// The JSON answers of the REST API.
pub(crate) const JSON_MEDIA_TYPE: &str = "application/vnd.github+json";
/// A comparison as one `git diff` text: every file, unlike the JSON
/// comparison, which stops at 300.
const DIFF_MEDIA_TYPE: &str = "application/vnd.github.diff";
/// A blob's content as it is, not base64 in JSON.
pub(crate) const RAW_MEDIA_TYPE: &str = "application/vnd.github.raw+json";
/// GitHub's maximum page size.
pub(crate) const PAGE_SIZE: &str = "100";
/// GitHub lists at most 3000 changed files per pull request: 30 pages of 100.
const MAX_FILE_PAGES: usize = 30;
/// The most changed files `pull_request_files` can return; a larger pull
/// request's files come whole from `compare_files` instead.
pub const MAX_LISTED_FILES: u64 = 3000;
const REVIEW_REQUESTED_QUERY: &str = "is:pr is:open archived:false review-requested:@me";
const AUTHORED_QUERY: &str = "is:pr is:open archived:false author:@me";
/// Open pull requests by others that the viewer submitted a review on.
const REVIEWED_QUERY: &str = "is:pr is:open archived:false reviewed-by:@me -author:@me";
/// How much of a body is read between progress reports.
const READ_CHUNK_BYTES: usize = 64 * 1024;

/// One answer from GitHub, read whole.
pub(crate) struct HttpResponse {
    status: StatusCode,
    pub body: String,
    etag: Option<ETag>,
    pub next_page: Option<String>,
}

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

    /// Open pull requests that request the viewer's review, most recently
    /// updated first. One search request (search has no conditional requests).
    pub fn review_requested(&self) -> Result<Vec<PullRequest>, GitHubError> {
        self.search_pull_requests(REVIEW_REQUESTED_QUERY)
    }

    /// Others' open pull requests the viewer has reviewed (GitHub takes the
    /// viewer off the requested reviewers once a review is submitted), most
    /// recently updated first; may overlap [`Self::review_requested`]. One
    /// search request.
    pub fn reviewed_by_viewer(&self) -> Result<Vec<PullRequest>, GitHubError> {
        self.search_pull_requests(REVIEWED_QUERY)
    }

    /// The viewer's own open pull requests, most recently updated first. One
    /// search request.
    pub fn authored(&self) -> Result<Vec<PullRequest>, GitHubError> {
        self.search_pull_requests(AUTHORED_QUERY)
    }

    /// The branches of the pull requests in the lists, the viewer's latest
    /// review and where they sit in their stacks: the viewer's, those
    /// requesting the viewer's review, those the viewer reviewed, and others' in `other_repositories` (the 100 most recently updated of
    /// the first ten repositories). One GraphQL request (the search API
    /// lacks both).
    pub fn listed_details(
        &self,
        other_repositories: &[RepoRef],
    ) -> Result<Vec<ListedPullRequestDetails>, GitHubError> {
        let others = others_search_queries(other_repositories).into_iter().next();
        let data: ListedDetailsData = self.graphql(
            LISTED_DETAILS_QUERY,
            &serde_json::json!({
                "authored": AUTHORED_QUERY,
                "reviewRequested": REVIEW_REQUESTED_QUERY,
                "reviewed": REVIEWED_QUERY,
                "includeOthers": others.is_some(),
                "others": others.unwrap_or_default(),
            }),
        )?;
        Ok(data.into_details())
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
        let response = self.get(&url, &[], etag, JSON_MEDIA_TYPE, &mut |_| {})?;
        if response.status == StatusCode::NOT_MODIFIED {
            return Ok(ConditionalResponse::NotModified);
        }
        let refs: PullRequestRefs = serde_json::from_str(&response.body)?;
        Ok(ConditionalResponse::Modified {
            value: refs.into_check(),
            etag: response.etag,
        })
    }

    /// Every file a pull request changes, with its patch, following the
    /// pages (100 files each, at most 3000 files in all). Reports the count
    /// fetched so far after each page.
    pub fn pull_request_files(
        &self,
        repo: &RepoRef,
        number: u64,
        on_page: &mut dyn FnMut(usize),
    ) -> Result<Vec<PullRequestFile>, GitHubError> {
        let first_page = format!("{}/pulls/{number}/files", repo.api_url());
        let mut files = Vec::new();
        let mut next = Some((first_page, vec![("per_page", PAGE_SIZE)]));
        for _ in 0..MAX_FILE_PAGES {
            let Some((url, query)) = next.take() else {
                break;
            };
            let response = self.get(&url, &query, None, JSON_MEDIA_TYPE, &mut |_| {})?;
            files.extend(serde_json::from_str::<Vec<PullRequestFile>>(
                &response.body,
            )?);
            on_page(files.len());
            // The next page's URL already carries its query.
            next = response.next_page.map(|url| (url, Vec::new()));
        }
        Ok(files)
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
        let response = self.get(&url, &[], etag, JSON_MEDIA_TYPE, &mut |_| {})?;
        if response.status == StatusCode::NOT_MODIFIED {
            return Ok(ConditionalResponse::NotModified);
        }
        let stack: StackResponse = serde_json::from_str(&response.body)?;
        Ok(ConditionalResponse::Modified {
            value: stack.into_stack(repo),
            etag: response.etag,
        })
    }

    /// Every file changed from `base_sha` to `head_sha` (from their merge
    /// base, as a pull request compares), with patches: one request for the
    /// whole `git diff`, reporting the bytes received as they arrive.
    pub fn compare_files(
        &self,
        repo: &RepoRef,
        base_sha: &str,
        head_sha: &str,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<Vec<PullRequestFile>, GitHubError> {
        let url = format!("{}/compare/{base_sha}...{head_sha}", repo.api_url());
        let response = self.get(&url, &[], None, DIFF_MEDIA_TYPE, on_progress)?;
        Ok(files_of_unified_diff(&response.body))
    }

    /// The pull requests a search query finds, most recently updated first.
    fn search_pull_requests(&self, query: &str) -> Result<Vec<PullRequest>, GitHubError> {
        let response: SearchResponse<SearchIssue> = self.get_json(
            &format!("{API_BASE}/search/issues"),
            &[
                ("q", query),
                ("sort", "updated"),
                ("order", "desc"),
                ("per_page", PAGE_SIZE),
            ],
        )?;
        Ok(response
            .items
            .into_iter()
            .filter_map(SearchIssue::into_pull_request)
            .collect())
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

    /// Reads an answer whole, reporting the bytes as they arrive. Counts the
    /// request, keeps the budget GitHub reports, and turns an exhausted
    /// budget or an error status into an error. `304 Not Modified` is a
    /// success with an empty body.
    fn read_response(
        &self,
        mut response: Response<Body>,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<HttpResponse, GitHubError> {
        // Count it and keep the budget before judging the answer.
        let status = response.status();
        let headers = response.headers().clone();
        let budget = rate_limit_of(&headers);
        self.usage
            .record(status == StatusCode::NOT_MODIFIED, budget);

        // Read the whole body (so an error status can carry GitHub's message), in chunks to report progress.
        let total_bytes = response.body().content_length();
        let mut reader = response.body_mut().as_reader();
        let mut bytes = Vec::new();
        let mut chunk = vec![0; READ_CHUNK_BYTES];
        loop {
            let read = reader.read(&mut chunk).map_err(ureq::Error::from)?;
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..read]);
            on_progress(DownloadProgress {
                received_bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
                total_bytes,
            });
        }
        let body = String::from_utf8_lossy(&bytes).into_owned();

        // A used-up budget first: it says when it refills, even with a `retry-after`.
        if is_rate_limited(status, &headers) {
            let (resource, resets_at) = budget.map_or(("API", 0), |(resource, limit)| {
                (resource.name(), limit.resets_at)
            });
            return Err(GitHubError::RateLimited {
                resource,
                resets_at,
            });
        }
        if is_secondary_rate_limited(status, &headers, &body) {
            return Err(GitHubError::SlowDown {
                retry_after_seconds: header_text(&headers, "retry-after")
                    .and_then(|seconds| seconds.parse().ok()),
            });
        }
        if !status.is_success() && status != StatusCode::NOT_MODIFIED {
            return Err(GitHubError::Api {
                status: status.as_u16(),
                body,
            });
        }
        Ok(HttpResponse {
            status,
            body,
            etag: header_text(&headers, "etag").map(ETag::new),
            next_page: header_text(&headers, "link").and_then(|link| next_page_url(&link)),
        })
    }
}
/// A GraphQL failure (reported with status 200) as an API error.
pub(crate) fn graphql_error(message: String) -> GitHubError {
    GitHubError::Api {
        status: StatusCode::OK.as_u16(),
        body: message,
    }
}

/// The budget a response reports in its `x-ratelimit-*` headers.
fn rate_limit_of(headers: &HeaderMap) -> Option<(RateLimitResource, RateLimit)> {
    let number = |name: &str| header_text(headers, name)?.parse::<u64>().ok();
    let resource = RateLimitResource::from_header(&header_text(headers, "x-ratelimit-resource")?)?;
    Some((
        resource,
        RateLimit {
            limit: u32::try_from(number("x-ratelimit-limit")?).ok()?,
            remaining: u32::try_from(number("x-ratelimit-remaining")?).ok()?,
            resets_at: number("x-ratelimit-reset")?,
        },
    ))
}

/// Whether GitHub refused the request because the budget is used up:
/// `403` or `429` with no requests remaining.
fn is_rate_limited(status: StatusCode, headers: &HeaderMap) -> bool {
    let is_refused = status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS;
    is_refused && header_text(headers, "x-ratelimit-remaining").as_deref() == Some("0")
}

/// Whether GitHub refused a burst of requests (its secondary rate limit),
/// which it says with a 403 or 429 carrying `retry-after` or naming it.
fn is_secondary_rate_limited(status: StatusCode, headers: &HeaderMap, body: &str) -> bool {
    let is_refused = status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS;
    is_refused
        && (header_text(headers, "retry-after").is_some() || body.contains("secondary rate limit"))
}

/// A header's value as text, when present and valid.
fn header_text(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use ureq::http::HeaderValue;

    use super::*;

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.insert(*name, HeaderValue::from_static(value));
        }
        headers
    }

    /// The core budget is read from GitHub's rate-limit headers.
    #[test]
    fn reads_rate_limit_headers() {
        let headers = headers(&[
            ("x-ratelimit-resource", "core"),
            ("x-ratelimit-limit", "5000"),
            ("x-ratelimit-remaining", "4975"),
            ("x-ratelimit-reset", "1793000000"),
        ]);
        assert_eq!(
            rate_limit_of(&headers),
            Some((
                RateLimitResource::Core,
                RateLimit {
                    limit: 5000,
                    remaining: 4975,
                    resets_at: 1_793_000_000
                }
            ))
        );
    }

    /// Only a refusal with nothing remaining counts as rate limited.
    #[test]
    fn detects_exhausted_budget() {
        let exhausted = headers(&[("x-ratelimit-remaining", "0")]);
        assert!(is_rate_limited(StatusCode::FORBIDDEN, &exhausted));
        assert!(!is_rate_limited(StatusCode::OK, &exhausted));
        let other_refusal = headers(&[("x-ratelimit-remaining", "12")]);
        assert!(!is_rate_limited(StatusCode::FORBIDDEN, &other_refusal));
    }
}
