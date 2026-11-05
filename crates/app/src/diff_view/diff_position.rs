/// A place in the diff text: a row of the detail list and a byte offset
/// into that row's code. Ordered by row, then byte, which is reading order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiffPosition {
    pub row: usize,
    pub byte: usize,
}
