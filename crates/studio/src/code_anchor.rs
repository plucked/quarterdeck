use std::fmt;
use std::ops::RangeInclusive;

use super::code_side::CodeSide;

/// A range of lines in one changed file, on one side of the change, that a
/// widget points at. The agent writes it; [`AnchorTargets`](crate::AnchorTargets)
/// checks that the lines exist in the diff before a widget is accepted, so
/// the app can always cut the quoted code from the diff itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeAnchor {
    /// The file's path in the pull request (its new path for a rename).
    pub path: String,
    pub side: CodeSide,
    /// First and last line, 1-based and inclusive, numbered on `side`.
    pub first_line: u32,
    pub last_line: u32,
}

impl CodeAnchor {
    /// The anchored line numbers.
    pub fn lines(&self) -> RangeInclusive<u32> {
        self.first_line..=self.last_line
    }
}

impl fmt::Display for CodeAnchor {
    /// `src/lib.rs:12–20 (head)`, or `src/lib.rs:12 (head)` for one line.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.first_line.cmp(&self.last_line) {
            std::cmp::Ordering::Equal => write!(
                formatter,
                "{}:{} ({})",
                self.path,
                self.first_line,
                self.side.as_str()
            ),
            std::cmp::Ordering::Less | std::cmp::Ordering::Greater => write!(
                formatter,
                "{}:{}–{} ({})",
                self.path,
                self.first_line,
                self.last_line,
                self.side.as_str()
            ),
        }
    }
}
