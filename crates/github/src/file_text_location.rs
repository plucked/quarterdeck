/// Where a file's text is read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileTextLocation {
    /// A git blob by its full SHA (40 hex digits).
    Blob(String),
    /// The file at `path` in commit `commit`: for files whose blob SHA is
    /// unknown or abbreviated.
    Path { commit: String, path: String },
}
