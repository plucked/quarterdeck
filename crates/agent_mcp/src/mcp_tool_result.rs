/// A tool result with both readable diagnostics and structured application data.
#[derive(Clone, Debug)]
pub struct McpToolResult {
    pub(crate) text: String,
    pub(crate) structured: serde_json::Value,
    pub(crate) is_error: bool,
}

impl McpToolResult {
    /// Returns a successful operation with machine-readable state or a receipt.
    pub fn success(text: impl Into<String>, structured: serde_json::Value) -> Self {
        Self {
            text: text.into(),
            structured,
            is_error: false,
        }
    }

    /// Returns a repairable tool error; text must contain the actionable detail.
    pub fn failure(text: impl Into<String>, structured: serde_json::Value) -> Self {
        Self {
            text: text.into(),
            structured,
            is_error: true,
        }
    }
}
