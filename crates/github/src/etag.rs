/// An HTTP entity tag GitHub sent with a response, as received (weak tags
/// keep their `W/` prefix). Sent back as `If-None-Match`; GitHub answers
/// `304 Not Modified` when the resource is unchanged, which costs no rate limit.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ETag(String);

impl ETag {
    pub fn new(tag: impl Into<String>) -> Self {
        Self(tag.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
