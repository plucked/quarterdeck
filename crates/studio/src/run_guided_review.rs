use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::guided_mcp_handler::{GUIDED_TOOL_NAMES, GuidedMcpHandler};
use crate::guided_review_guide::GUIDED_REVIEW_GUIDE;
use crate::mcp_model::McpModel;
use crate::mcp_run_format::McpRunFormat;
use crate::run_mcp_review::{completion_receipt, run_mcp_review};
use crate::{AgentProgress, AgentRunError, GuidedReviewAnswer, GuidedReviewRun};

/// Authors a repairable guided review and returns its sealed, unpublished buckets.
/// Blocks off the UI thread. A server receipt, successful terminal result and
/// successful process exit are required; the host owns atomic publication.
pub fn run_guided_review(
    program: &Path,
    prompt: &str,
    workspace: Option<&Path>,
    run: GuidedReviewRun<'_>,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(AgentProgress),
) -> Result<GuidedReviewAnswer, AgentRunError> {
    // The endpoint and caller independently own the synchronized draft during the run.
    let handler = Arc::new(GuidedMcpHandler::new(run));
    let answer = run_mcp_review(
        program,
        prompt,
        workspace,
        McpRunFormat {
            guide: GUIDED_REVIEW_GUIDE,
            tool_names: GUIDED_TOOL_NAMES,
            model: McpModel::OpusHigh,
        },
        &handler,
        cancel,
        on_progress,
    )?;

    // Recover exactly the server-sealed artifact rather than a second model copy.
    let receipt = completion_receipt(&answer)?.to_owned();
    let buckets = handler
        .finished_buckets(&receipt)
        .map_err(AgentRunError::Failed)?;
    Ok(GuidedReviewAnswer {
        buckets,
        answer,
        validation_receipt: receipt,
        audit: handler.audit(),
    })
}
