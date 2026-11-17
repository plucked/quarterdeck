/// The bucket and item positions of one stop in the prepared walkthrough.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct WalkthroughItemId {
    pub bucket: usize,
    pub item: usize,
}
