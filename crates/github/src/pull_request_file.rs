use serde::Deserialize;

/// One file changed by a pull request, as `GET /repos/{o}/{r}/pulls/{n}/files` returns it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PullRequestFile {
    pub filename: String,
    /// `added`, `removed`, `modified`, `renamed`, `copied`, `changed` or `unchanged`.
    pub status: String,
    pub additions: u64,
    pub deletions: u64,
    /// Unified diff hunks; absent for binary or very large files.
    pub patch: Option<String>,
}
