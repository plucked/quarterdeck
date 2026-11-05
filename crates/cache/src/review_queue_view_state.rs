use crate::DetailScrollPosition;

/// What the review queue restores after a relaunch.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ReviewQueueViewState {
    /// GitHub id of the selected pull request; it may no longer be in the queue.
    pub selected_pull_request_id: Option<u64>,
    pub detail_scroll: DetailScrollPosition,
}
