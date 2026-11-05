/// How an asynchronous merge ended, or that it still runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeOutcome {
    /// Still running on GitHub; poll the request `uuid` again.
    Pending { uuid: String },
    /// Merged into the base branch.
    Merged,
    /// Added to the merge queue; it merges when the queue gets to it.
    Enqueued,
    /// GitHub could not merge it; why.
    Failed(String),
}
