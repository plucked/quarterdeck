/// A picker offers one explicit submission; refreshing clears its selection.
pub(super) enum SessionPickerPhase {
    Loading,
    Ready,
    Failed(String),
    Sending,
    Submitted(String),
}
