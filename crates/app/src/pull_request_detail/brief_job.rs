use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use qd_cache::{AgentPublication, ReviewScope};
use qd_github::{PullRequest, RepoRef};
use qd_studio::{
    AgentProgress, BriefMcpRecord, BriefRecord, BriefRun, RequestLayer, StudioRequest, ToolCall,
    WorkspaceDiff, run_brief_review, studio_prompt,
};

use super::brief_code::BriefCode;
use super::change_diff_of::change_diff_of;
use super::loaded_file::LoadedFile;
use super::pending_brief::PendingBrief;
use super::review_job_error::ReviewJobError;
use super::review_publication_gate::ReviewPublicationGate;
use super::review_target::ReviewTarget;
use crate::app_services::AppServices;
use crate::brief_view::PreparedBrief;
use crate::display_text::unix_now;
use crate::ui_state::SharedUiStateStore;

/// One brief to write, with everything its background run needs.
pub(super) struct BriefJob {
    pub run_id: String,
    pub gate: Arc<ReviewPublicationGate>,
    pub base_sha: Option<String>,
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
    pub store: SharedUiStateStore,
}

impl BriefJob {
    /// Writes the brief: checks out the code, runs Claude Code in it, checks
    /// its answer, stores the brief for later sessions and prepares it for
    /// drawing; the workspace is removed on return. Blocks; run it on the
    /// background executor. The error says why there is no brief.
    pub fn run(
        self,
        report: &mut dyn FnMut(AgentProgress),
    ) -> Result<PreparedBrief, ReviewJobError<PendingBrief>> {
        // The layers' descriptions, then the code, then the agent in it.
        let started = Instant::now();
        let cancel = self.gate.cancellation_flag();
        let scope = self.target.viewed_scope(&self.pull_request);
        let snapshot_id = format!("head:{:?};base:{:?}", self.head_sha, self.base_sha);
        self.reserve_publication(&scope, &snapshot_id)?;
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
        write_workspace_diffs(&code, &prompt.workspace_diffs)?;
        let answer = run_brief_review(
            &self.program,
            &prompt.text,
            code.workspace(),
            BriefRun {
                run_id: &self.run_id,
                snapshot_id: &snapshot_id,
                targets: prompt.targets,
            },
            cancel,
            report,
        )
        .map_err(|error| error.to_string())?;
        let duration = started.elapsed();
        let publication = AgentPublication {
            run_id: self.run_id,
            snapshot_id,
            validation_receipt: answer.validation_receipt,
            cost_usd: answer.answer.cost_usd,
            session_id: answer.answer.session_id.clone(),
            model: answer.answer.model.clone(),
            duration_ms: u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
            audit: answer.audit,
        };

        // Everything about the run, kept with the brief for a report.
        let record = BriefRecord {
            mcp: Some(BriefMcpRecord {
                base_sha: self.base_sha,
                run_id: publication.run_id.clone(),
                snapshot_id: publication.snapshot_id.clone(),
                validation_receipt: publication.validation_receipt.clone(),
                audit: publication.audit.clone(),
            }),
            repository: self.pull_request.repo.full_name(),
            number: self.pull_request.number,
            url: self.pull_request.html_url.clone(),
            subject: request.subject.clone(),
            layers: request.layers.clone(),
            head_sha: self.head_sha,
            reader: request.reader,
            code: request.code,
            prompt: prompt.text,
            answer: Some(answer.artifact),
            tool_calls: ToolCall::from_transcript(&answer.answer.transcript),
            transcript: Some(answer.answer.transcript),
            rejected: Vec::new(),
            cost_usd: answer.answer.cost_usd,
            session_id: answer.answer.session_id,
            model: answer.answer.model,
            duration,
            written_at: u64::try_from(unix_now()).unwrap_or(0),
        };

        // A storage failure returns the exact completed artifact for a save-only retry.
        Box::new(PendingBrief::new(scope, answer.studio, record, publication))
            .save(&self.store, &self.gate)
    }
    /// Reserves the captured scope before spending a model run, after cancellation is checked.
    fn reserve_publication(&self, scope: &ReviewScope, snapshot_id: &str) -> Result<(), String> {
        let store = self.store.lock();
        if self.gate.cancellation_flag().load(Ordering::SeqCst) {
            return Err("Cancelled.".into());
        }
        store
            .begin_brief_run(
                scope,
                &self.run_id,
                snapshot_id,
                self.head_sha.as_deref(),
                self.base_sha.as_deref(),
            )
            .map_err(|error| format!("Could not start the brief: {error}"))
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
fn write_workspace_diffs(code: &BriefCode, diffs: &[WorkspaceDiff]) -> Result<(), String> {
    let Some(folder) = code.workspace() else {
        return Ok(());
    };
    WorkspaceDiff::write_all(folder, diffs)
        .map_err(|error| format!("Could not prepare the changed files: {error}"))
}
