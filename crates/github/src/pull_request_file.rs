use serde::Deserialize;

/// One file changed by a pull request, as `GET /repos/{o}/{r}/pulls/{n}/files` returns it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PullRequestFile {
    pub filename: String,
    /// The path before a rename; set only when `status` is `renamed`.
    #[serde(default)]
    pub previous_filename: Option<String>,
    /// `added`, `removed`, `modified`, `renamed`, `copied`, `changed` or `unchanged`.
    pub status: String,
    pub additions: u64,
    pub deletions: u64,
    /// Unified diff hunks; absent for binary or very large files.
    pub patch: Option<String>,
    /// The git blob SHA of the file's content after the change: in full
    /// from the files API, abbreviated (7+ hex digits) when read from a
    /// whole diff's `index` line; `None` when unknown or deleted. Content
    /// addressed, so text cached under it never goes stale.
    #[serde(default)]
    pub sha: Option<String>,
}
