use super::file_listing::FileListing;

/// One changed file of a [`StudioRequest`](crate::StudioRequest).
#[derive(Clone)]
pub struct StudioRequestFile {
    pub path: String,
    /// The path before a rename.
    pub previous_path: Option<String>,
    /// GitHub's file status: `added`, `modified`, `removed`, `renamed`, …
    pub status: String,
    pub additions: u64,
    pub deletions: u64,
    pub listing: FileListing,
}
