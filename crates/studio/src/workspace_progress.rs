/// How far preparing a review workspace has come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceProgress {
    /// Downloading the head commit; git's percentage once it reports one.
    Fetching { percent: Option<u8> },
    /// Writing the two trees and the diff.
    Unpacking,
}
