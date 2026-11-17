use std::sync::Arc;

/// One file's indexing outcome. A stopping error ends requests without hiding partial results.
pub(super) enum SearchContentResult {
    Text {
        path: String,
        text: Arc<str>,
    },
    Skipped,
    Failed(String),
    Stopped {
        message: String,
        retry_after: Option<std::time::Duration>,
    },
}
