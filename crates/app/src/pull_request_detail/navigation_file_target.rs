use super::walkthrough_item_id::WalkthroughItemId;

/// A file row browses the full diff, opens its owning step, or toggles an inline diff.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum NavigationFileTarget {
    FileDiff,
    GuidedItem(WalkthroughItemId),
    GroupDiff,
}
