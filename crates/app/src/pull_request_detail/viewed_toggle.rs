use qd_cache::ViewedFile;

/// What toggling a file's viewed checkbox did, for the caller to save.
#[derive(Debug, PartialEq, Eq)]
pub enum ViewedToggle {
    /// The file is now viewed at this fingerprint.
    Marked(ViewedFile),
    /// The file named here is no longer viewed.
    Unmarked(String),
}
