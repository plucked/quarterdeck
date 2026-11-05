/// What the review queue restores after a relaunch.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ReviewQueueViewState {
    /// GitHub id of the selected pull request; it may no longer be in the queue.
    pub selected_pull_request_id: Option<u64>,
    /// Vertical scroll offset of the detail pane in pixels, as gpui reports
    /// it: 0 at the top, negative further down.
    pub detail_scroll_offset: f32,
}
