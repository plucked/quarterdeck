/// A failure to prepare, start or shut down an ephemeral MCP endpoint.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// The supplied tool catalog cannot be represented by the MCP contract.
    #[error("invalid MCP tool catalog: {0}")]
    InvalidCatalog(String),
    /// The operating system could not create listener, runtime or thread resources.
    #[error("MCP server I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// Secure per-run credential generation failed.
    #[error("MCP credential randomness failed: {0}")]
    Randomness(String),
    /// The owned runtime thread terminated unexpectedly.
    #[error("MCP runtime thread terminated unexpectedly")]
    RuntimeStopped,
}
