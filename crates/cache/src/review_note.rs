use crate::{NoteAnchor, NoteDestination, NoteId, ReviewScope};

/// One review comment the user wrote: a local note or a GitHub draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewNote {
    pub id: NoteId,
    pub destination: NoteDestination,
    pub scope: ReviewScope,
    /// The file it is about.
    pub path: String,
    pub anchor: NoteAnchor,
    /// Markdown.
    pub body: String,
    /// Seconds since the Unix epoch.
    pub created_at: i64,
    pub updated_at: i64,
}
