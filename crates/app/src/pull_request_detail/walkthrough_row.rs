use super::walkthrough_item_id::WalkthroughItemId;

/// A small row of the virtualized plan; content lives in prepared buckets and files.
#[derive(Clone, Copy)]
pub(super) enum WalkthroughRow {
    Bucket(usize),
    Actions(usize),
    Guidance(usize),
    Item(WalkthroughItemId),
    GroupMembers(WalkthroughItemId),
    File { id: WalkthroughItemId, file: usize },
}
