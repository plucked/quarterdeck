use gpui::SharedString;

/// The state of the review queue's refresh from GitHub.
pub enum RefreshStatus {
    /// Showing the last fetched or cached queue; nothing in flight.
    Idle,
    /// A refresh is in flight.
    Refreshing,
    /// The last refresh failed; the message is shown to the user.
    Failed(SharedString),
}
