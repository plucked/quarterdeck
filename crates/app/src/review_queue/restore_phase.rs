/// Which queue a saved selection is being matched against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestorePhase {
    /// The queue read from the cache at launch, possibly stale.
    CachedQueue,
    /// The first queue fetched from GitHub; the last chance to find it.
    RefreshedQueue,
}
