use std::ops::Range;

use crate::DiffLineKind;

/// One line of a hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    /// The line's number in the old file; `None` for an added line.
    pub old_number: Option<u32>,
    /// The line's number in the new file; `None` for a removed line.
    pub new_number: Option<u32>,
    /// The line as displayed: without its `+`/`-`/` ` marker and with tabs
    /// expanded to spaces.
    pub text: String,
    /// Byte ranges of `text` that differ from the line it was paired with
    /// (a removed line with the added line replacing it). Empty for context
    /// lines, unpaired lines, lines rewritten too much to compare, and when
    /// highlighting whole lines only.
    pub changed_ranges: Vec<Range<usize>>,
}
