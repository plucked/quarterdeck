use qc_cache::DiffViewSettings;

use super::changed_files::ChangedFiles;
use super::comment_placement::CommentPlacement;
use super::file_review_state::FileReviewState;

/// What the detail pane shows, as the row list needs it.
pub struct DetailContent<'state> {
    pub has_pull_request: bool,
    pub changed_files: Option<&'state ChangedFiles>,
    /// Whether each loaded file passes the file filter, by file index.
    pub shown_files: &'state [bool],
    pub file_review: &'state FileReviewState,
    pub settings: DiffViewSettings,
    /// Where comment cards and the composer go.
    pub comments: &'state CommentPlacement,
}
