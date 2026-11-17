/// Validation is advisory; only `Ready` denotes an immutable finished draft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftStatus {
    Accepted,
    Rejected,
    Valid,
    Ready,
}
