use qd_github::LineSide;

/// The lines the comment being written or edited is about: lines
/// `first..=last` on `side` of the loaded file `file`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommentRange {
    pub file: usize,
    pub side: LineSide,
    pub first: u32,
    pub last: u32,
}

impl CommentRange {
    /// A comment under line `line` that starts at `start_line` (or at
    /// `line` itself without one).
    pub fn ending_at(file: usize, side: LineSide, line: u32, start_line: Option<u32>) -> Self {
        Self {
            file,
            side,
            first: start_line.unwrap_or(line),
            last: line,
        }
    }
}
