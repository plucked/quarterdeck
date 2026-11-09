/// Why an agent run produced no studio.
#[derive(Debug, thiserror::Error)]
pub enum AgentRunError {
    #[error("could not start the agent: {0}")]
    Start(#[source] std::io::Error),
    #[error("the agent stopped: {0}")]
    Failed(String),
    #[error("the agent finished without a structured answer")]
    NoAnswer,
    #[error("the run was cancelled")]
    Cancelled,
}
