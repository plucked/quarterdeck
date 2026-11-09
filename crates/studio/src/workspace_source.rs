/// What a review workspace checks out: the head commit of one repository,
/// read with a token when given (private repositories).
#[derive(Debug, Clone)]
pub struct WorkspaceSource {
    /// Where the commit is fetched from: a GitHub repository's HTTPS URL
    /// (`WorkspaceSource::github`), or a local path in tests.
    pub remote_url: String,
    /// The commit after the change: what `head/` holds.
    pub head_sha: String,
    /// Sent only to `github.com` as a request header, never written into
    /// the workspace, so the agent cannot read it.
    pub token: Option<String>,
}

impl WorkspaceSource {
    /// Commit `head_sha` of the GitHub repository `owner/name`.
    pub fn github(repository: &str, head_sha: impl Into<String>, token: Option<String>) -> Self {
        Self {
            remote_url: format!("https://github.com/{repository}.git"),
            head_sha: head_sha.into(),
            token,
        }
    }
}
