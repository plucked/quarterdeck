/// What to do when the window becomes active, as decided by
/// [`super::refresh_schedule::RefreshSchedule::decide_on_activation`].
#[derive(Debug, PartialEq, Eq)]
pub enum ActivationRefresh {
    /// The queue is stale enough to refresh now.
    Refresh,
    /// The last refresh is recent; the timed refresh will catch up.
    NotDue,
    /// A refresh is already in flight; starting another would cancel it.
    AlreadyRefreshing,
}
