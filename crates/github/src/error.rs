#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("no GitHub token found (set GITHUB_TOKEN or run `gh auth login`)")]
    NoToken,
    #[error("GitHub API returned {status}: {body}")]
    Api { status: u16, body: String },
    #[error("http error: {0}")]
    Http(#[from] ureq::Error),
    #[error("decode error: {0}")]
    Decode(#[from] serde_json::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
