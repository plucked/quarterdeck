/// What a text area tells its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaEvent {
    /// The text changed (typing, paste, cut, deletion).
    Changed,
    /// The caret or current typing style changed the toolbar highlight.
    FormattingChanged,
    /// The comment toolbar or Markdown preview changed the editor's height.
    LayoutChanged,
    /// cmd-enter, or Return in a single-line area: the owner takes the text.
    Submit,
    /// escape: the owner drops the edit.
    Cancel,
}
