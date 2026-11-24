/// Whether lines that differ only in whitespace count as changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WhitespaceMode {
    /// Every change is shown, as in the patch.
    #[default]
    Show,
    /// Re-diffs each hunk ignoring whitespace so lines differing only in
    /// indentation or spacing show as unchanged, and whitespace-only parts of
    /// a line are not highlighted.
    Ignore,
}
