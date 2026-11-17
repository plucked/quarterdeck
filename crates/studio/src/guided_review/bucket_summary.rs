/// Small accepted-bucket metadata; selectors are read separately for a repair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BucketSummary {
    pub bucket_id: String,
    pub label: String,
    pub item_count: usize,
    pub file_count: usize,
}
