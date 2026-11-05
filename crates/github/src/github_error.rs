/// Every way a GitHub call can fail.
#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    /// Neither `GITHUB_TOKEN` nor `gh auth token` produced a token.
    #[error("no GitHub token found (set GITHUB_TOKEN or run `gh auth login`)")]
    NoToken,
    /// GitHub answered with a non-success HTTP status.
    #[error("GitHub API returned {status}: {body}")]
    Api { status: u16, body: String },
    /// The request did not complete (DNS, TLS, connection, body read).
    #[error("http error: {0}")]
    Http(#[from] ureq::Error),
    /// The response body did not match the expected JSON shape.
    #[error("decode error: {0}")]
    Decode(#[from] serde_json::Error),
}
