use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::brief_mcp_guide::{BRIEF_TOOL_NAMES, brief_mcp_system_prompt};
use crate::brief_mcp_handler::BriefMcpHandler;
use crate::mcp_model::McpModel;
use crate::mcp_run_format::McpRunFormat;
use crate::run_mcp_review::{completion_receipt, run_mcp_review};
use crate::{AgentProgress, AgentRunError, BriefRun, BriefRunAnswer};

/// Authors complete brief pages with repair feedback and returns the sealed result.
/// The shared blocking runner preserves existing Claude default model selection;
/// the application remains responsible for publishing the artifact and metadata.
pub fn run_brief_review(
    program: &Path,
    prompt: &str,
    workspace: Option<&Path>,
    run: BriefRun<'_>,
    cancel: &AtomicBool,
    on_progress: &mut dyn FnMut(AgentProgress),
) -> Result<BriefRunAnswer, AgentRunError> {
    // The endpoint and caller independently own this frozen brief draft.
    let handler = Arc::new(BriefMcpHandler::new(run));
    let guide = brief_mcp_system_prompt();
    let answer = run_mcp_review(
        program,
        prompt,
        workspace,
        McpRunFormat {
            guide: &guide,
            tool_names: BRIEF_TOOL_NAMES,
            model: McpModel::ClaudeDefault,
        },
        &handler,
        cancel,
        on_progress,
    )?;

    // Preserve both typed rendering data and the exact accepted artifact for reports.
    let receipt = completion_receipt(&answer)?.to_owned();
    let finished = handler
        .finished_studio(&receipt)
        .map_err(AgentRunError::Failed)?;
    Ok(BriefRunAnswer {
        studio: finished.studio,
        artifact: finished.artifact,
        answer,
        validation_receipt: receipt,
        audit: handler.audit(),
    })
}
