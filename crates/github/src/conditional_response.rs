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

impl<T> ConditionalResponse<T> {
    /// The same answer with a new value made from the value, the tag kept.
    pub fn map<U>(self, convert: impl FnOnce(T) -> U) -> ConditionalResponse<U> {
        match self {
            Self::NotModified => ConditionalResponse::NotModified,
            Self::Modified { value, etag } => ConditionalResponse::Modified {
                value: convert(value),
                etag,
            },
        }
    }
}
