use qc_cache::DiffViewSettings;

use super::changed_files::ChangedFiles;
use super::file_review_state::FileReviewState;

/// What the detail pane shows, as the row list needs it.
pub struct DetailContent<'state> {
    pub has_pull_request: bool,
    pub changed_files: Option<&'state ChangedFiles>,
    /// Whether each loaded file passes the file filter, by file index.
    pub shown_files: &'state [bool],
    pub file_review: &'state FileReviewState,
    pub settings: DiffViewSettings,
}
