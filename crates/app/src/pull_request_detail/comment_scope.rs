use qd_cache::NoteDestination;

/// Which comments the diff shows: GitHub's conversations and drafts with
/// the local notes on a single pull request, the local notes alone on a
/// combined review (GitHub has no place for comments across layers).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentScope {
    GitHubAndLocal,
    LocalOnly,
}

impl CommentScope {
    /// Whether notes going to `destination` show.
    pub fn includes(self, destination: NoteDestination) -> bool {
        match self {
            Self::GitHubAndLocal => true,
            Self::LocalOnly => destination == NoteDestination::Local,
        }
    }
}
