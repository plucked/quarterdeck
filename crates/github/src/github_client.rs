use serde::de::DeserializeOwned;
use ureq::Agent;
use ureq::http::{HeaderMap, StatusCode};

use crate::next_page_url::next_page_url;
use crate::pull_request_revision::PullRequestRefs;
use crate::search_issue::SearchIssue;
use crate::search_response::SearchResponse;
use crate::{
    ApiUsage, ConditionalResponse, ETag, GitHubError, PullRequest, PullRequestFile,
    PullRequestRevision, RateLimit, RateLimitResource, RepoRef,
};

const API_BASE: &str = "https://api.github.com";
const API_VERSION: &str = "2022-11-28";
const USER_AGENT: &str = concat!("quick-check/", env!("CARGO_PKG_VERSION"));
/// GitHub's maximum page size.
const PAGE_SIZE: &str = "100";
/// GitHub lists at most 3000 changed files per pull request: 30 pages of 100.
const MAX_FILE_PAGES: usize = 30;
const REVIEW_REQUESTED_QUERY: &str = "is:pr is:open archived:false review-requested:@me";

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
        let response: SearchResponse<SearchIssue> = self.get_json(
            &format!("{API_BASE}/search/issues"),
            &[
                ("q", REVIEW_REQUESTED_QUERY),
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

    /// The head and base commits of a pull request, unless they are unchanged
    /// since the response tagged `etag` (then it is free).
    pub fn pull_request_revision(
        &self,
        repo: &RepoRef,
        number: u64,
        etag: Option<&ETag>,
    ) -> Result<ConditionalResponse<PullRequestRevision>, GitHubError> {
        let url = format!(
            "{API_BASE}/repos/{}/{}/pulls/{number}",
            repo.owner, repo.name
        );
        let response = self.send(&url, &[], etag)?;
        if response.status == StatusCode::NOT_MODIFIED {
            return Ok(ConditionalResponse::NotModified);
        }
        let refs: PullRequestRefs = serde_json::from_str(&response.body)?;
        Ok(ConditionalResponse::Modified {
            value: refs.into_revision(),
            etag: response.etag,
        })
    }

    /// Every file a pull request changes, with its patch, following the
    /// pages (100 files each, at most 3000 files in all).
    pub fn pull_request_files(
        &self,
        repo: &RepoRef,
        number: u64,
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
            let response = self.send(&url, &query, None)?;
            files.extend(serde_json::from_str::<Vec<PullRequestFile>>(
                &response.body,
            )?);
            // The next page's URL already carries its query.
            next = response.next_page.map(|url| (url, Vec::new()));
        }
        Ok(files)
    }

    /// Sends an authenticated GET and decodes the JSON body.
    fn get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        query: &[(&str, &str)],
    ) -> Result<T, GitHubError> {
        let response = self.send(url, query, None)?;
        Ok(serde_json::from_str(&response.body)?)
    }

    /// Sends an authenticated GET, conditional on `etag` when given, and
    /// reads the answer whole. Counts the request, keeps the budget GitHub
    /// reports, and turns an exhausted budget or an error status into an
    /// error. `304 Not Modified` is a success with an empty body.
    fn send(
        &self,
        url: &str,
        query: &[(&str, &str)],
        etag: Option<&ETag>,
    ) -> Result<HttpResponse, GitHubError> {
        // Build the request with the headers GitHub requires for REST v3.
        let mut request = self
            .agent
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {}", self.token))
            .header("User-Agent", USER_AGENT)
            .header("X-GitHub-Api-Version", API_VERSION);
        if let Some(etag) = etag {
            request = request.header("If-None-Match", etag.as_str());
        }
        for (key, value) in query {
            request = request.query(*key, *value);
        }

        // Count it and keep the budget before judging the answer.
        let mut response = request.call()?;
        let status = response.status();
        let headers = response.headers().clone();
        let budget = rate_limit_of(&headers);
        self.usage
            .record(status == StatusCode::NOT_MODIFIED, budget);

        // Read the whole body so an error status can carry GitHub's message.
        let body = response.body_mut().read_to_string()?;
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
