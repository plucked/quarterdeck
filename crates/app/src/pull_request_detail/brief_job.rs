use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use qd_cache::UiStateStore;
use qd_github::{PullRequest, RepoRef};
use qd_studio::{
    AgentProgress, BriefRecord, RequestLayer, StudioRequest, ToolCall, WorkspaceDiff, parse_studio,
    run_claude, studio_prompt,
};

use super::brief_actions::change_diff_of;
use super::brief_code::BriefCode;
use super::loaded_file::LoadedFile;
use super::review_target::ReviewTarget;
use crate::app_services::AppServices;
use crate::brief_view::PreparedBrief;
use crate::display_date::unix_now;
use crate::ui_state::write_brief;

/// One brief to write, with everything its background run needs.
pub struct BriefJob {
    pub services: AppServices,
    pub pull_request: PullRequest,
    /// What the brief is about; it is stored under this target's scope.
    pub target: ReviewTarget,
    /// The head commit the loaded files are of: checked out, and recorded
    /// with the brief (the version line compares it with the shown head).
    pub head_sha: Option<String>,
    /// The `claude` executable.
    pub program: PathBuf,
    /// The request without its code access, which the checkout decides.
    pub request: StudioRequest,
    /// The loaded files (shared, cheap to clone), whose patches become the
    /// workspace's `change.diff` on the background executor.
    pub files: Vec<LoadedFile>,
    /// Where the brief is stored.
    pub store: Arc<Mutex<UiStateStore>>,
}

impl BriefJob {
    /// Writes the brief: checks out the code, runs Claude Code in it, checks
    /// its answer, stores the brief for later sessions and prepares it for
    /// drawing; the workspace is removed on return. Blocks; run it on the
    /// background executor. The error says why there is no brief.
    pub fn run(
        self,
        cancel: &AtomicBool,
        report: &mut dyn FnMut(AgentProgress),
    ) -> Result<PreparedBrief, String> {
        // The layers' descriptions, then the code, then the agent in it.
        let started = Instant::now();
        let mut request = self.request;
        read_layer_descriptions(&self.services, &self.pull_request.repo, &mut request.layers);
        let code = BriefCode::prepare(
            &self.services,
            &self.pull_request,
            self.head_sha.as_deref(),
            &change_diff_of(&self.files),
            cancel,
            report,
        )
        .map_err(|_| "Cancelled.".to_owned())?;
        request.code = code.access();
        let prompt = studio_prompt(&request);
        write_workspace_diffs(&code, &prompt.workspace_diffs);
        let answer = run_claude(
            &self.program,
            &prompt.text,
            code.workspace(),
            cancel,
            report,
        )
        .map_err(|error| error.to_string())?;
        let raw_answer = answer.output.clone();
        let checked = parse_studio(answer.output, &prompt.targets).map_err(|error| {
            format!("The agent's answer did not have the studio's shape: {error}")
        })?;

        // Everything about the run, kept with the brief for a report.
        let record = BriefRecord {
            repository: self.pull_request.repo.full_name(),
            number: self.pull_request.number,
            url: self.pull_request.html_url.clone(),
            subject: request.subject.clone(),
            layers: request.layers.clone(),
            head_sha: self.head_sha,
            reader: request.reader,
            code: request.code,
            prompt: prompt.text,
            answer: Some(raw_answer),
            tool_calls: ToolCall::from_transcript(&answer.transcript),
            transcript: Some(answer.transcript),
            rejected: checked.rejected,
            cost_usd: answer.cost_usd,
            session_id: answer.session_id,
            model: answer.model,
            duration: started.elapsed(),
            written_at: u64::try_from(unix_now()).unwrap_or(0),
        };

        // Keep it for later sessions, then prepare it for drawing.
        let scope = self.target.viewed_scope(&self.pull_request);
        write_brief(&self.store, &scope, &checked.studio, &record);
        Ok(PreparedBrief::prepare(checked.studio, record))
    }
}

/// Fills in the descriptions of the stack layers that lack one: from the
/// cache, else one request per layer (cached for next time). A layer that
/// cannot be read keeps none; the prompt says so. Blocks: background
/// executor.
fn read_layer_descriptions(services: &AppServices, repo: &RepoRef, layers: &mut [RequestLayer]) {
    for layer in layers
        .iter_mut()
        .filter(|layer| layer.description.is_none())
    {
        let cached = services
            .cache
            .lock()
            .pull_request_by_number(repo, layer.number);
        let pull_request = match (cached, &services.github) {
            (Ok(Some(cached)), _) => Some(cached),
            (Ok(None) | Err(_), Ok(client)) => {
                match client.pull_request_by_number(repo, layer.number) {
                    Ok(fetched) => {
                        if let Err(error) = services.cache.lock().put_pull_request(&fetched) {
                            eprintln!("brief: cannot cache layer #{}: {error:#}", layer.number);
                        }
                        Some(fetched)
                    }
                    Err(error) => {
                        eprintln!("brief: cannot read layer #{}: {error:#}", layer.number);
                        None
                    }
                }
            }
            (Ok(None) | Err(_), Err(_)) => None,
        };
        layer.description = pull_request.and_then(|pull_request| pull_request.body);
    }
}

/// Writes every changed file's numbered diff into the workspace
/// (`diffs/<path>.diff`), so the agent can read and cite the diffs the
/// prompt had no room for. Without a workspace there is nowhere to write; a
/// failed write is reported on stderr (those diffs then cannot be found,
/// so they are not cited).
fn write_workspace_diffs(code: &BriefCode, diffs: &[WorkspaceDiff]) {
    let Some(folder) = code.workspace() else {
        return;
    };
    if let Err(error) = WorkspaceDiff::write_all(folder, diffs) {
        eprintln!("brief: cannot write the workspace's diffs: {error}");
    }
}
