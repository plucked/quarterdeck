use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use qd_cache::ReviewScope;
use qd_github::PullRequest;
use qd_studio::{
    AgentProgress, SmartFilter, StudioRequest, WorkspaceDiff, parse_smart_buckets,
    run_smart_filter, smart_filter_prompt,
};

use super::brief_code::BriefCode;
use super::change_diff_of::change_diff_of;
use super::loaded_file::LoadedFile;
use crate::app_services::AppServices;
use crate::display_text::unix_now;
use crate::ui_state::SharedUiStateStore;

/// Everything pinned at the click, independent of later navigation or refreshes.
pub(super) struct SmartFilterJob {
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
    pub fn run(
        self,
        cancel: &AtomicBool,
        report: &mut dyn FnMut(AgentProgress),
    ) -> Result<(SmartFilter, Option<String>), String> {
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
        let prompt = smart_filter_prompt(&request);
        if let Some(workspace) = code.workspace() {
            WorkspaceDiff::write_all(workspace, &prompt.workspace_diffs)
                .map_err(|error| format!("Could not prepare the changed files: {error}"))?;
        }
        let answer = run_smart_filter(
            &self.program,
            &prompt.text,
            code.workspace(),
            cancel,
            report,
        )
        .map_err(|error| error.to_string())?;
        let paths: Vec<_> = self.files.iter().map(LoadedFile::filename).collect();
        let filter = SmartFilter {
            head_sha: self.head_sha,
            base_sha: self.base_sha,
            generated_at: u64::try_from(unix_now()).unwrap_or(0),
            buckets: parse_smart_buckets(answer.output, &paths)?,
        };
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled.".into());
        }
        let warning = self
            .store
            .lock()
            .put_smart_filter(&self.scope, &filter)
            .err()
            .map(|error| {
                format!("Available this session, but could not save the smart filter: {error}")
            });
        Ok((filter, warning))
    }
}
