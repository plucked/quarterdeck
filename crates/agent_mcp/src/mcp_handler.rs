use crate::{McpToolDescriptor, McpToolResult};

/// Application tools exposed for a single immutable run scope.
///
/// Calls can overlap. Implementations validate arguments and serialize their own
/// mutations. Handlers must return promptly: disconnects cannot undo synchronous
/// work, and server shutdown joins active handlers. Close application mutations
/// before shutting down a cancelled run. No SDK or runtime types cross this API.
pub trait McpHandler: Send + Sync + 'static {
    /// Returns the static catalog captured once before the server starts.
    fn tools(&self) -> Vec<McpToolDescriptor>;

    /// Executes a listed tool, returning actionable text for repairable failures.
    fn call_tool(&self, name: &str, arguments: serde_json::Value) -> McpToolResult;
}
