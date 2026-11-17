use qd_agent_mcp::McpHandler;

use crate::AgentProgress;

/// Application-owned authoring lifecycle, independent of the transport runtime.
pub(crate) trait RunMcpHandler: McpHandler {
    /// Revokes mutations under their serialization gate.
    fn close(&self);
    /// Drains a bounded queue of authoritative accepted/rejected draft progress.
    fn drain_progress(&self) -> Vec<AgentProgress>;
}
