/// Every way a GitHub call can fail.
#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    /// Neither `GITHUB_TOKEN` nor `gh auth token` produced a token.
    #[error("no GitHub token found (set GITHUB_TOKEN or run `gh auth login`)")]
    NoToken,
    /// The budget for this kind of request is used up until `resets_at`
    /// (seconds since the Unix epoch).
    #[error("GitHub {resource} rate limit reached; it refills in {} min", minutes_until(*resets_at))]
    RateLimited {
        resource: &'static str,
        resets_at: u64,
    },
    /// GitHub answered with a non-success HTTP status.
    #[error("GitHub API returned {status}: {body}")]
    Api { status: u16, body: String },
    /// The request did not complete (DNS, TLS, connection, body read).
    #[error("http error: {0}")]
    Http(#[from] ureq::Error),
    /// A file's text cannot be shown, and why (binary, too large, missing).
    #[error("the file cannot be shown: {reason}")]
    NoFileText { reason: &'static str },
    /// The response body did not match the expected JSON shape.
    #[error("decode error: {0}")]
    Decode(#[from] serde_json::Error),
}

/// Whole minutes from now until `unix_seconds`, rounded up; 0 when past.
fn minutes_until(unix_seconds: u64) -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    unix_seconds.saturating_sub(now).div_ceil(60)
}
