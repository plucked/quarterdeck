use crate::{NoteAnchor, NoteDestination, NoteId, ReviewScope};

/// One review comment the user wrote: a local note or a GitHub draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewNote {
    pub id: NoteId,
    pub destination: NoteDestination,
    pub scope: ReviewScope,
    /// The file it is about.
    pub path: String,
    pub head_sha: Option<String>,
    pub base_sha: Option<String>,
    pub excerpt: Option<String>,
    pub anchor: NoteAnchor,
    /// Markdown.
    pub body: String,
    /// Seconds since the Unix epoch.
    pub created_at: i64,
    pub updated_at: i64,
}

impl ReviewNote {
    /// Line numbers identify code only within the diff where the author selected them.
    pub fn matches_revision(&self, head: Option<&str>, base: Option<&str>) -> bool {
        matches!(self.anchor, NoteAnchor::Reply { .. })
            || (head.is_some()
                && base.is_some()
                && self.head_sha.as_deref() == head
                && self.base_sha.as_deref() == base)
    }
}
