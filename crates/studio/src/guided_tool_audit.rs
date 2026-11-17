/// One bounded server-side record of an authoring call and its validation result.
/// It contains no endpoint, bearer credential, source text, or CLI session identity.
#[derive(Debug, Clone)]
pub struct GuidedToolAudit {
    pub tool_name: String,
    pub status: String,
    pub revision: u64,
    pub issue_codes: Vec<String>,
}
