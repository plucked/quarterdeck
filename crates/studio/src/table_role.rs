/// What a table is for, which decides how its rows are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableRole {
    /// A mapping or comparison: rows as given.
    Mapping,
    /// A sequence of steps (a minimal failure case, before and after):
    /// rows are numbered.
    Trace,
}
