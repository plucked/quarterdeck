/// A file's whole content at one version, as text, for showing the
/// unchanged lines around a diff's hunks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileText {
    /// The full git blob SHA of the content: the same content has the same
    /// SHA in every commit and repository, so it is a cache key that never
    /// goes stale.
    pub blob_sha: String,
    pub text: String,
}
