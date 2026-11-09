use qd_diff::FileDiff;

/// The diffs a brief quotes its code from, found by path: the pull
/// request's loaded files in the detail, the sample files in the gallery.
pub trait BriefDiffs {
    /// The diff of the file at `path`, or (for a rename) previously at it;
    /// `None` when no such file is loaded.
    fn diff_of(&self, path: &str) -> Option<&FileDiff>;
}
