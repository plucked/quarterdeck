/// What a finished agent run returned.
#[derive(Debug, Clone)]
pub struct AgentAnswer {
    /// The structured output, validated by the CLI against the schema it was given.
    pub output: serde_json::Value,
    /// What the run cost, as the CLI reports it.
    pub cost_usd: Option<f64>,
    /// The CLI's session, which a follow-up question could resume.
    pub session_id: Option<String>,
    /// The model the session ran, from its init event.
    pub model: Option<String>,
    /// Every event line but the partial-message stream: the init, whole
    /// assistant messages (tool calls), tool results and the result, for a
    /// brief report to show what the run read and did.
    pub transcript: Vec<String>,
}
