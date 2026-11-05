use std::ops::Range;

/// A byte range of an [`crate::InlineText`]'s text that links to `url`, as
/// written in the source (not checked or resolved).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRange {
    pub range: Range<usize>,
    pub url: String,
}
