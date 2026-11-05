/// One of the file filter's on/off switches: each hides one group of files
/// from the tree and the diff when on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterToggle {
    /// Files marked viewed.
    Viewed,
    /// Test files and folders.
    Tests,
    /// Generated files (lockfiles, minified, snapshots).
    Generated,
}
