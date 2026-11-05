/// Which code column of a diff row a text is in: the one column of a
/// unified diff, or the old or new half of a split row. A selection stays
/// in one column, so copying from a split view never mixes old and new code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    Unified,
    Old,
    New,
}
