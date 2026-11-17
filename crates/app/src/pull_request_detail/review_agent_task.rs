use std::sync::Arc;

use gpui::Task;
use qd_cache::ReviewScope;

use super::review_publication_gate::ReviewPublicationGate;

/// One scope's active generation and the immutable revision it can publish.
pub(super) struct ReviewAgentTask {
    pub run_id: String,
    pub scope: ReviewScope,
    pub head_sha: Option<String>,
    pub base_sha: Option<String>,
    pub gate: Arc<ReviewPublicationGate>,
    /// Retained until completion or accepted cancellation.
    pub _task: Task<()>,
}

impl Drop for ReviewAgentTask {
    fn drop(&mut self) {
        self.gate.cancel();
    }
}
