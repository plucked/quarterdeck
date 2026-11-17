/// One stop in a bucket's reading order. Related assets can share a stop,
/// but every member stays an exact path and is marked only by the reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartReviewItem {
    pub label: String,
    pub description: String,
    /// Exact members in the agent's suggested reading order.
    pub files: Vec<String>,
}
