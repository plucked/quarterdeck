/// An opaque digest issued only for a frozen, valid run/snapshot/revision/content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessReceipt(String);

impl ReadinessReceipt {
    pub(crate) fn new(digest: &str) -> Self {
        Self(format!("guided-review-v1:{digest}"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
