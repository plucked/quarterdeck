/// How a click on a line's `+` opens the comment composer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerOpening {
    /// A new comment on the clicked line, replacing any open editor.
    New,
    /// A shift-click: the comment open on the same file and side stretches
    /// over the lines in between; without one, a new comment.
    ExtendOpen,
}
