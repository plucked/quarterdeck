/// Every way a GitHub call can fail.
#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    /// No token in `GITHUB_TOKEN`, the Keychain or the `gh` CLI.
    #[error("not signed in to GitHub (sign in from Settings → GitHub, or run `gh auth login`)")]
    NoToken,
    /// The login Keychain could not be read or written.
    #[error("Keychain error: {message}")]
    Keychain { message: String },
    /// The budget for this kind of request is used up until `resets_at`
    /// (seconds since the Unix epoch).
    #[error("GitHub {resource} rate limit reached; it refills in {} min", minutes_until(*resets_at))]
    RateLimited {
        resource: &'static str,
        resets_at: u64,
    },
    /// GitHub refused a burst of requests (its secondary rate limit) and
    /// asks to wait, `retry_after_seconds` when it says how long.
    #[error("GitHub asked Quarterdeck to slow down; it tries again {}", retry_wording(*retry_after_seconds))]
    SlowDown { retry_after_seconds: Option<u64> },
    /// GitHub answered with a non-success HTTP status; `body` as sent, shown
    /// by its `message` when it is GitHub's JSON error.
    #[error("GitHub API returned {status}: {}", api_message(body))]
    Api { status: u16, body: String },
    /// The request did not complete (DNS, TLS, connection, body read).
    #[error("http error: {0}")]
    Http(#[from] ureq::Error),
    /// A file's text cannot be shown, and why: the file is binary, too
    /// large, missing or not a file (`reason` completes "the file …").
    #[error("the file {reason}")]
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

/// When the next attempt comes, after a secondary rate limit.
fn retry_wording(retry_after_seconds: Option<u64>) -> String {
    match retry_after_seconds {
        Some(seconds) => format!("after {} min", seconds.div_ceil(60).max(1)),
        None => "with the next refresh".to_owned(),
    }
}

/// The `message` of GitHub's JSON error body, else the body as sent.
fn api_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|json| json.get("message")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| body.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An API error shows GitHub's message, not its JSON.
    #[test]
    fn shows_the_message_of_a_json_error() {
        let error = GitHubError::Api {
            status: 404,
            body: r#"{"message": "Not Found", "documentation_url": "https://docs"}"#.to_owned(),
        };
        assert_eq!(error.to_string(), "GitHub API returned 404: Not Found");
        let error = GitHubError::Api {
            status: 500,
            body: "oops".to_owned(),
        };
        assert_eq!(error.to_string(), "GitHub API returned 500: oops");
    }
}
