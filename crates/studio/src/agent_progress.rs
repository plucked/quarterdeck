/// How far a brief's run has come: preparing the code, then the agent's
/// events as they arrive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentProgress {
    /// Downloading the two commits; git's percentage once it reports one.
    FetchingCode { percent: Option<u8> },
    /// Writing the commits' files and the diff into the workspace.
    UnpackingCode,
    /// The agent process started and read the prompt.
    Started,
    /// The model is reasoning.
    Thinking,
    /// The model is reading the checked-out code; this many tool calls so far.
    Exploring { tool_calls: usize },
    /// The model is writing its structured answer; this many characters so far.
    Writing { characters: usize },
}
