use crate::{ChangeHighlight, WhitespaceMode};

/// How a patch is turned into a [`crate::FileDiff`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiffOptions {
    pub whitespace: WhitespaceMode,
    pub highlight: ChangeHighlight,
}
