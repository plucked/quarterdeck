use crate::ETag;

/// The answer to a conditional request: unchanged since the tag sent, or a
/// new value with the tag to send next time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalResponse<T> {
    /// `304 Not Modified`: what was cached is still current. Free.
    NotModified,
    Modified {
        value: T,
        etag: Option<ETag>,
    },
}
