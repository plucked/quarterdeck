/// Where a diff line comes from: the patch, or the file's text shown
/// around the patch's hunks.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LineOrigin {
    /// A line of GitHub's patch: changed, or context the patch carries.
    #[default]
    Patch,
    /// An unchanged line revealed from the file's text, outside the patch:
    /// GitHub takes no review comment on it.
    FileText,
}
