/// Which queue a saved selection is being matched against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestorePhase {
    /// The queue read from the cache at launch, possibly stale.
    Cached,
    /// Some lists have refreshed; others may still contain the cached launch data.
    Refreshing,
    /// The first queue fetched from GitHub; the last chance to find it.
    Refreshed,
    /// GitHub is unavailable for this launch; resolve from the cached queue.
    Unavailable,
}

impl RestorePhase {
    /// Cached and partially refreshed queues can still lose a selection later in startup.
    pub fn is_provisional(self) -> bool {
        matches!(self, Self::Cached | Self::Refreshing)
    }
}
