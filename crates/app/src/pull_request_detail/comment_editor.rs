use std::sync::Arc;

use gpui::{Entity, Subscription};
use qd_cache::NoteId;
use qd_github::LineSide;

use super::loaded_file::LoadedFile;
use crate::text_area::TextArea;

/// The comment being written or edited, with its text area.
pub struct CommentEditor {
    pub target: EditorTarget,
    pub text: Entity<TextArea>,
    /// Original replacement text at the current inline anchor, read once per range change.
    pub(super) suggestion_source: Option<Arc<str>>,
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

impl EditorTarget {
    /// Source for a GitHub suggestion, only for a new comment on head-side patch lines.
    pub(super) fn suggestion_source(&self, files: &[LoadedFile]) -> Option<Arc<str>> {
        let Self::NewComment {
            file,
            path,
            side: LineSide::New,
            line,
            start_line,
        } = self
        else {
            return None;
        };
        let loaded = files.get(*file)?;
        if loaded.filename() != path || loaded.file.status == "removed" {
            return None;
        }
        qd_diff::patch_head_lines(
            loaded.file.patch.as_deref()?,
            start_line.unwrap_or(*line)..=*line,
        )
        .map(Into::into)
    }
}
