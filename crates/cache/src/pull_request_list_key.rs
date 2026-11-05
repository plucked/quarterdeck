/// Name of an ordered list of pull requests in the cache, such as the
/// review-requested queue. Written by the app, stored in `pr_lists.list_key`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PullRequestListKey(&'static str);

impl PullRequestListKey {
    /// Open pull requests that request the viewer's review.
    pub const REVIEW_REQUESTED: Self = Self("review_requested");

    /// The key as stored in SQLite.
    pub fn as_str(self) -> &'static str {
        self.0
    }
}
