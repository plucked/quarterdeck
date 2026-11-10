/// How far a job split into batches has come: `done` of `total` batches
/// (the syntax coloring of the loaded diffs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchProgress {
    pub done: usize,
    pub total: usize,
}
