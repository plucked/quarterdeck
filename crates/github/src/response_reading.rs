use std::io::Read;

use ureq::Body;
use ureq::http::{HeaderMap, Response, StatusCode};

use crate::http_response::HttpResponse;
use crate::next_page_url::next_page_url;
use crate::{DownloadProgress, ETag, GitHubClient, GitHubError, RateLimit, RateLimitResource};

/// How much of a body is read between progress reports.
const READ_CHUNK_BYTES: usize = 64 * 1024;

impl GitHubClient {
    /// Reads an answer whole, reporting the bytes as they arrive. Counts the
    /// request, keeps the budget GitHub reports, and turns an exhausted
    /// budget or an error status into an error. `304 Not Modified` is a
    /// success with an empty body.
    pub(crate) fn read_response(
        &self,
        mut response: Response<Body>,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<HttpResponse, GitHubError> {
        // Count it and keep the budget before judging the answer.
        let status = response.status();
        let headers = response.headers().clone();
        let budget = rate_limit_of(&headers);
        self.usage()
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
    is_refusal(status) && header_text(headers, "x-ratelimit-remaining").as_deref() == Some("0")
}

/// Whether GitHub refused a burst of requests (its secondary rate limit),
/// which it says with a 403 or 429 carrying `retry-after` or naming it.
fn is_secondary_rate_limited(status: StatusCode, headers: &HeaderMap, body: &str) -> bool {
    is_refusal(status)
        && (header_text(headers, "retry-after").is_some() || body.contains("secondary rate limit"))
}

/// Whether `status` is how GitHub refuses a request it limits: `403` or `429`.
fn is_refusal(status: StatusCode) -> bool {
    status == StatusCode::FORBIDDEN || status == StatusCode::TOO_MANY_REQUESTS
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
