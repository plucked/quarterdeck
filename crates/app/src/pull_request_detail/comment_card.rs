use qd_cache::NoteId;

/// One card of comments in the diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommentCard {
    /// A GitHub conversation (an index into the review state's threads),
    /// with the viewer's draft replies to it.
    Thread { thread: usize, replies: Vec<NoteId> },
    /// One of the user's notes: a local note, or a draft of a new GitHub
    /// conversation.
    Note(NoteId),
}
