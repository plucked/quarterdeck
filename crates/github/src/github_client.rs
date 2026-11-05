use std::io::Read;

use serde::de::DeserializeOwned;
use ureq::Agent;
use ureq::http::{HeaderMap, Response, StatusCode};
use ureq::{Body, RequestBuilder};

use crate::next_page_url::next_page_url;
use crate::pull_request_revision::PullRequestRefs;
use crate::pull_request_stack::StackResponse;
use crate::search_issue::SearchIssue;
use crate::search_response::SearchResponse;
use crate::stack_membership_query::{STACK_MEMBERSHIP_QUERY, StackMembershipResponse};
use crate::unified_diff::files_of_unified_diff;
use crate::{
    ApiUsage, ConditionalResponse, DownloadProgress, ETag, GitHubError, ListedStackMembership,
    PullRequest, PullRequestFile, PullRequestStack, RateLimit, RateLimitResource, RepoRef,
    RevisionCheck,
};

const API_BASE: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";
const USER_AGENT: &str = concat!("quick-check/", env!("CARGO_PKG_VERSION"));
/// The JSON answers of the REST API.
const JSON_MEDIA_TYPE: &str = "application/vnd.github+json";
/// A comparison as one `git diff` text: every file, unlike the JSON
/// comparison, which stops at 300.
const DIFF_MEDIA_TYPE: &str = "application/vnd.github.diff";
/// GitHub's maximum page size.
const PAGE_SIZE: &str = "100";
/// GitHub lists at most 3000 changed files per pull request: 30 pages of 100.
const MAX_FILE_PAGES: usize = 30;
const REVIEW_REQUESTED_QUERY: &str = "is:pr is:open archived:false review-requested:@me";
const AUTHORED_QUERY: &str = "is:pr is:open archived:false author:@me";
/// How much of a body is read between progress reports.
const READ_CHUNK_BYTES: usize = 64 * 1024;

/// One answer from GitHub, read whole.
struct HttpResponse {
    status: StatusCode,
    body: String,
    etag: Option<ETag>,
    next_page: Option<String>,
}

/// Blocking, authenticated GitHub REST client. Cheap to clone: clones share
/// the connection pool and the usage counts.
#[derive(Clone)]
pub struct GitHubClient {
    agent: Agent,
    token: String,
    usage: ApiUsage,
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
            usage: ApiUsage::default(),
        }
    }

    /// A client using the token [`crate::resolve_token`] finds.
    pub fn from_env() -> Result<Self, GitHubError> {
        Ok(Self::new(crate::resolve_token()?))
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

    /// The viewer's own open pull requests, most recently updated first. One
    /// search request.
    pub fn authored(&self) -> Result<Vec<PullRequest>, GitHubError> {
        self.search_pull_requests(AUTHORED_QUERY)
    }

    /// Where the pull requests of both lists sit in their stacks; standalone
    /// ones are left out. One GraphQL request (the search API lacks stacks).
    pub fn stack_memberships(&self) -> Result<Vec<ListedStackMembership>, GitHubError> {
        let request = serde_json::json!({
            "query": STACK_MEMBERSHIP_QUERY,
            "variables": {
                "authored": AUTHORED_QUERY,
                "reviewRequested": REVIEW_REQUESTED_QUERY,
            },
        });
        let builder = self.with_headers(
            self.agent.post(format!("{API_BASE}/graphql")),
            JSON_MEDIA_TYPE,
            None,
        );
        let response = self.read_response(builder.send_json(&request)?, &mut |_| {})?;
        let answer: StackMembershipResponse = serde_json::from_str(&response.body)?;
        answer
            .into_memberships()
            .map_err(|message| GitHubError::Api {
                status: response.status.as_u16(),
                body: message,
            })
    }

    /// A pull request by number, shaped like the search results (same id),
    /// for a stack layer that is in neither list. One request.
    pub fn pull_request_by_number(
        &self,
        repo: &RepoRef,
        number: u64,
    ) -> Result<PullRequest, GitHubError> {
        let issue: SearchIssue = self.get_json(
            &format!(
                "{API_BASE}/repos/{}/{}/issues/{number}",
                repo.owner, repo.name
            ),
            &[],
        )?;
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
        let url = format!(
            "{API_BASE}/repos/{}/{}/pulls/{number}",
            repo.owner, repo.name
        );
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
        let first_page = format!(
            "{API_BASE}/repos/{}/{}/pulls/{number}/files",
            repo.owner, repo.name
        );
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
        let url = format!(
            "{API_BASE}/repos/{}/{}/stacks/{number}",
            repo.owner, repo.name
        );
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
        let url = format!(
            "{API_BASE}/repos/{}/{}/compare/{base_sha}...{head_sha}",
            repo.owner, repo.name
        );
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
    fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        query: &[(&str, &str)],
    ) -> Result<T, GitHubError> {
        let response = self.get(url, query, None, JSON_MEDIA_TYPE, &mut |_| {})?;
        Ok(serde_json::from_str(&response.body)?)
    }

    /// Sends an authenticated GET for `accept`, conditional on `etag` when
    /// given, and reads the answer whole.
    fn get(
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
        let request = request
            .header("Accept", accept)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", USER_AGENT)
            .header("X-GitHub-Api-Version", API_VERSION);
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

        if is_rate_limited(status, &headers) {
            let (resource, resets_at) = budget.map_or(("API", 0), |(resource, limit)| {
                (resource.name(), limit.resets_at)
            });
            return Err(GitHubError::RateLimited {
                resource,
                resets_at,
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
