/// How much of a response body has arrived.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadProgress {
    pub received_bytes: u64,
    /// From `Content-Length`; `None` when GitHub streams without one.
    pub total_bytes: Option<u64>,
}
