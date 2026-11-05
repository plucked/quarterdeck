/// How much the local cache holds, for the settings window: each kind of
/// cached GitHub data, and the database file on disk. UI state (viewed marks,
/// notes, layout) is not counted; clearing the cache keeps it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheUsage {
    /// Pull requests with their lists, and stacks with their layers.
    pub pull_requests: CachedData,
    /// The changed files of single pull requests.
    pub pull_request_files: CachedData,
    /// The changed files of combined stack ranges (two commits compared).
    pub comparisons: CachedData,
    /// The database file and its write-ahead log on disk.
    pub disk_bytes: u64,
    /// Of `disk_bytes`, space SQLite freed but the file still holds.
    pub free_bytes: u64,
}

/// One kind of cached data: how many entries (pull requests, file sets,
/// comparisons), how many files they hold, and the bytes of their text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CachedData {
    pub entries: u64,
    pub files: u64,
    pub bytes: u64,
}
