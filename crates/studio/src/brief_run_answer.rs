use crate::{AgentAnswer, GuidedToolAudit, Studio};

/// A sealed, unpublished brief and its original accepted artifact for reporting.
#[derive(Debug, Clone)]
pub struct BriefRunAnswer {
    pub studio: Studio,
    pub artifact: serde_json::Value,
    pub answer: AgentAnswer,
    pub validation_receipt: String,
    pub audit: Vec<GuidedToolAudit>,
}
