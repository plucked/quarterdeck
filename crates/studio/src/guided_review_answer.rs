use crate::{AgentAnswer, GuidedToolAudit, SmartBucket};

/// A sealed draft matched to a successful Claude terminal result.
/// Publication remains the application's atomic persistence operation.
#[derive(Debug, Clone)]
pub struct GuidedReviewAnswer {
    pub buckets: Vec<SmartBucket>,
    pub answer: AgentAnswer,
    pub validation_receipt: String,
    pub audit: Vec<GuidedToolAudit>,
}
