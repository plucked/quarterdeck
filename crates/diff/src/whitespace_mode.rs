/// Whether lines that differ only in whitespace count as changed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum WhitespaceMode {
    /// Every change is shown, as in the patch.
    #[default]
    Show,
    /// A removed line replaced by one equal to it but for whitespace shows
    /// as unchanged, and whitespace-only parts of a line are not highlighted.
    Ignore,
}
