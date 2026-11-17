use crate::mcp_model::McpModel;

/// A terminal answer identifies only the artifact already sealed by the server.
pub const MCP_COMPLETION_SCHEMA: &str = r#"{
 "type":"object","additionalProperties":false,"required":["validation_receipt"],
 "properties":{"validation_receipt":{"type":"string","minLength":1}}
}"#;

/// Artifact-specific authoring instructions and exact MCP permissions.
#[derive(Clone, Copy)]
pub(crate) struct McpRunFormat<'a> {
    pub guide: &'a str,
    pub tool_names: &'a [&'a str],
    pub model: McpModel,
}
