use qd_github::LineSide;

/// One line of the diff a comment can sit under: line `line` on `side` of
/// the loaded file `file`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiffLineRef {
    pub file: usize,
    pub side: LineSide,
    pub line: u32,
}
