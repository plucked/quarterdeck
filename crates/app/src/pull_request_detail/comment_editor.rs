use gpui::{Entity, Subscription};
use qc_cache::NoteId;
use qc_github::LineSide;

use crate::text_area::TextArea;

/// The comment being written or edited, with its text area.
pub struct CommentEditor {
    pub target: EditorTarget,
    pub text: Entity<TextArea>,
    /// Keeps the text area's events (submit, cancel, changes) observed.
    pub _subscription: Subscription,
}

/// What the editor writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorTarget {
    /// A new comment under line `line` of `side` (lines `start_line..=line`)
    /// of file `file`.
    NewComment {
        file: usize,
        path: String,
        side: LineSide,
        line: u32,
        start_line: Option<u32>,
    },
    /// A draft reply to the GitHub conversation `thread_id`, in file `path`.
    Reply { thread_id: String, path: String },
    /// A new text for one of the user's notes.
    EditNote(NoteId),
    /// A new text for a GitHub comment the viewer may edit, sent at once.
    EditGitHubComment { comment_id: String },
}
