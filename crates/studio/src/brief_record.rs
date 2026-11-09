use std::time::Duration;

use super::code_access::CodeAccess;
use super::reader_familiarity::ReaderFamiliarity;
use super::rejected_widget::RejectedWidget;
use super::request_layer::RequestLayer;
use super::tool_call::ToolCall;

/// Everything about how one brief was made, kept with it for the session
/// so it can be reported: what it was about, what the agent was given, what
/// it answered and did, and what the app's checks left out.
#[derive(Debug, Clone)]
pub struct BriefRecord {
    /// `owner/name`.
    pub repository: String,
    pub number: u64,
    /// The pull request's web address.
    pub url: String,
    /// `Pull request #12: Title`, or the combined range of a stack.
    pub subject: String,
    /// The stack layers a combined brief spans, bottom first.
    pub layers: Vec<RequestLayer>,
    /// The head commit the brief was written for (the shown pull request's,
    /// or the combined range's top layer's); `None` when it was not known.
    pub head_sha: Option<String>,
    pub reader: ReaderFamiliarity,
    pub code: CodeAccess,
    /// The user message, as sent on stdin.
    pub prompt: String,
    /// The structured answer, before the app's checks; `None` for a brief
    /// loaded from storage (only the checked studio is kept).
    pub answer: Option<serde_json::Value>,
    /// The run's events without the partial-message stream; `None` for a
    /// brief loaded from storage.
    pub transcript: Option<Vec<String>>,
    /// What the agent read, in order.
    pub tool_calls: Vec<ToolCall>,
    pub rejected: Vec<RejectedWidget>,
    pub cost_usd: Option<f64>,
    pub session_id: Option<String>,
    pub model: Option<String>,
    /// From pressing Write brief to the answer, checkout included.
    pub duration: Duration,
    /// When it was written, in seconds since the Unix epoch.
    pub written_at: u64,
}
