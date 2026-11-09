use gpui::SharedString;
use qd_studio::AgentProgress;

use super::prepared_brief::PreparedBrief;

/// Where the brief of one pull request (or combined range) stands.
pub enum BriefState {
    /// The agent is at work; how far it came, once it reported.
    Writing(Option<AgentProgress>),
    /// Boxed: with its record (prompt, answer, transcript) it is far larger
    /// than the other states.
    Written(Box<PreparedBrief>),
    /// The run failed; the message is shown with a way to try again.
    Failed(SharedString),
}
