/// What the detail pane's main column shows under the header.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DetailMode {
    /// The changed files' diffs.
    #[default]
    Diff,
    /// The AI-written brief of the change.
    Brief,
}
