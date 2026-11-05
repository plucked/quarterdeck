/// Where the selected pull request came from, which decides what a refresh
/// that no longer lists it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionOrigin {
    /// Picked from a list: when it leaves every list (merged, closed,
    /// review done), the selection clears.
    Listed,
    /// Opened from a stack's map although in no list: it stays selected.
    StackLayer,
    /// Merged or closed from the app: it stays selected though it leaves
    /// the lists, so the outcome stays in view.
    Finished,
}
