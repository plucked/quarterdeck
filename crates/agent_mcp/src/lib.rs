//! Blocking ownership of authenticated, per-run MCP tool servers.
//!
//! `mcp_server` owns server lifetime; `mcp_handler`, `mcp_tool_descriptor` and
//! `mcp_tool_result` define the application boundary; `mcp_error` reports failures.
//! `mcp_service` adapts the official SDK; `http_listener` and `http_policy` bound
//! HTTP access; `run_credential` creates an ephemeral bearer secret.

mod http_listener;
mod http_policy;
mod mcp_error;
mod mcp_handler;
mod mcp_server;
mod mcp_service;
mod mcp_tool_descriptor;
mod mcp_tool_result;
mod run_credential;

pub use mcp_error::McpError;
pub use mcp_handler::McpHandler;
pub use mcp_server::McpServer;
pub use mcp_tool_descriptor::McpToolDescriptor;
pub use mcp_tool_result::McpToolResult;
