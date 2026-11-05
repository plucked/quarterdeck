use std::ops::Range;

use crate::InlineStyle;

/// A byte range of an [`crate::InlineText`]'s text that has a non-plain style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledRun {
    pub range: Range<usize>,
    pub style: InlineStyle,
}
