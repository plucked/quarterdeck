use crate::LineSide;

/// A conversation on a pull request's diff, as GitHub keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewThread {
    /// GitHub's node id, for replies and resolving.
    pub id: String,
    pub path: String,
    pub side: LineSide,
    /// The line in the current diff; `None` when the code moved on (outdated).
    pub line: Option<u32>,
    /// The first line of a multi-line comment.
    pub start_line: Option<u32>,
    /// The line the thread was started on, in the diff of its time.
    pub original_line: Option<u32>,
    pub is_resolved: bool,
    pub is_outdated: bool,
    pub permissions: ThreadPermissions,
    /// Oldest first.
    pub comments: Vec<ThreadComment>,
}

/// What the viewer may do with a [`ReviewThread`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThreadPermissions {
    pub can_reply: bool,
    pub can_resolve: bool,
    pub can_unresolve: bool,
}

/// One comment of a [`ReviewThread`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadComment {
    /// GitHub's node id, for editing and deleting.
    pub id: String,
    /// Empty when the account is gone.
    pub author_login: String,
    /// Markdown.
    pub body: String,
    pub created_at: String,
    /// Part of the viewer's pending review on GitHub, not published yet.
    pub is_pending: bool,
    pub can_update: bool,
    pub can_delete: bool,
}
