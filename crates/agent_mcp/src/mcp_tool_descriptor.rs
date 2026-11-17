/// One application tool and the JSON schemas advertised to an agent.
#[derive(Clone, Debug)]
pub struct McpToolDescriptor {
    /// Stable MCP tool name, unique within this server.
    pub name: String,
    /// The operation and its constraints, visible to the model.
    pub description: String,
    /// JSON Schema object describing tool arguments.
    pub input_schema: serde_json::Value,
    /// Optional JSON Schema object describing successful structured results.
    pub output_schema: Option<serde_json::Value>,
    /// Whether this operation leaves application state unchanged.
    pub read_only: bool,
}
