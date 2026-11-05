use qc_cache::DiffViewSettings;

use super::changed_files::ChangedFiles;
use super::file_review_state::FileReviewState;

/// What the detail pane shows, as the row list needs it.
pub struct DetailContent<'state> {
    pub has_pull_request: bool,
    pub has_description: bool,
    pub changed_files: Option<&'state ChangedFiles>,
    pub file_review: &'state FileReviewState,
    pub settings: DiffViewSettings,
}
