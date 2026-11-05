/// What a text area tells its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaEvent {
    /// The text changed (typing, paste, cut, deletion).
    Changed,
    /// cmd-enter: the owner takes the text.
    Submit,
    /// escape: the owner drops the edit.
    Cancel,
}
