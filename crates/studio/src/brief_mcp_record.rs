use crate::GuidedToolAudit;

/// Durable evidence tying a brief to its sealed MCP draft. This is separate
/// from the full accepted artifact in `BriefRecord::answer` and contains no
/// endpoint, authentication capability or source text.
#[derive(Debug, Clone)]
pub struct BriefMcpRecord {
    /// The captured comparison base, used to detect base-only stack changes.
    pub base_sha: Option<String>,
    pub run_id: String,
    pub snapshot_id: String,
    pub validation_receipt: String,
    pub audit: Vec<GuidedToolAudit>,
}
