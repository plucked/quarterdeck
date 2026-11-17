use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use qd_cache::{GuidedReviewPublication, ReviewScope};
use qd_github::PullRequest;
use qd_studio::{
    AgentProgress, GuidedReviewRun, SmartFilter, StudioRequest, WorkspaceDiff,
    guided_review_prompt, run_guided_review,
};

use super::brief_code::BriefCode;
use super::change_diff_of::change_diff_of;
use super::loaded_file::LoadedFile;
use super::pending_guided_review::PendingGuidedReview;
use super::review_job_error::ReviewJobError;
use super::review_publication_gate::ReviewPublicationGate;
use crate::app_services::AppServices;
use crate::display_text::unix_now;
use crate::ui_state::SharedUiStateStore;

/// Everything pinned at the click, independent of later navigation or refreshes.
pub(super) struct SmartFilterJob {
    pub run_id: String,
    pub gate: Arc<ReviewPublicationGate>,
    pub services: AppServices,
    pub pull_request: PullRequest,
    pub scope: ReviewScope,
    pub head_sha: String,
    pub base_sha: Option<String>,
    pub program: PathBuf,
    pub request: StudioRequest,
    pub files: Vec<LoadedFile>,
    pub store: SharedUiStateStore,
}

impl SmartFilterJob {
    /// Prepares frozen code, repairs a private draft and publishes only its verified completion.
    pub fn run(
        self,
        report: &mut dyn FnMut(AgentProgress),
    ) -> Result<SmartFilter, ReviewJobError<PendingGuidedReview>> {
        let started = Instant::now();
        let cancel = self.gate.cancellation_flag();
        // Reserve ownership before spending an agent run; cancellation is checked
        // inside the lock so a delayed older job cannot supersede a newer one.
        {
            let store = self.store.lock();
            if cancel.load(Ordering::SeqCst) {
                return Err(ReviewJobError::Failed("Cancelled.".into()));
            }
            store
                .begin_guided_review(
                    &self.scope,
                    &self.run_id,
                    &self.head_sha,
                    self.base_sha.as_deref(),
                )
                .map_err(|error| format!("Could not start guided review: {error}"))?;
        }
        let code = BriefCode::prepare(
            &self.services,
            &self.pull_request,
            Some(&self.head_sha),
            &change_diff_of(&self.files),
            cancel,
            report,
        )
        .map_err(|_| "Cancelled.".to_owned())?;
        let mut request = self.request;
        request.code = code.access();
        let prompt = guided_review_prompt(&request);
        if let Some(workspace) = code.workspace() {
            WorkspaceDiff::write_all(workspace, &prompt.workspace_diffs)
                .map_err(|error| format!("Could not prepare the changed files: {error}"))?;
        }
        let paths: Vec<_> = self
            .files
            .iter()
            .map(|file| file.filename().to_owned())
            .collect();
        let snapshot_id = format!(
            "{}:{}",
            self.head_sha,
            self.base_sha.as_deref().unwrap_or("")
        );
        let run = GuidedReviewRun {
            run_id: &self.run_id,
            snapshot_id: &snapshot_id,
            changed_paths: &paths,
        };
        let answer = run_guided_review(
            &self.program,
            &prompt.text,
            code.workspace(),
            run,
            cancel,
            report,
        )
        .map_err(|error| error.to_string())?;
        let filter = SmartFilter {
            head_sha: self.head_sha,
            base_sha: self.base_sha,
            generated_at: u64::try_from(unix_now()).unwrap_or(0),
            buckets: answer.buckets,
        };
        let publication = GuidedReviewPublication {
            run_id: self.run_id,
            snapshot_id,
            validation_receipt: answer.validation_receipt,
            cost_usd: answer.answer.cost_usd,
            session_id: answer.answer.session_id,
            model: answer.answer.model,
            duration_ms: u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
            audit: answer.audit,
        };
        // A storage failure retains the sealed artifact and receipt for a save-only retry.
        Box::new(PendingGuidedReview::new(self.scope, filter, publication))
            .save(&self.store, &self.gate)
    }
}
