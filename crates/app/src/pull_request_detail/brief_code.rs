use std::path::Path;
use std::sync::atomic::AtomicBool;

use qd_github::PullRequest;
use qd_studio::{
    AgentProgress, CodeAccess, ReviewWorkspace, WorkspaceError, WorkspaceProgress, WorkspaceSource,
    prepare_workspace, workspace_root,
};

use crate::app_services::AppServices;

/// The code a brief's agent can read: a workspace with the head commit, or
/// only the diffs and why. The workspace is removed when this is dropped,
/// so it lives as long as the run that holds it.
pub enum BriefCode {
    Workspace(ReviewWorkspace),
    /// Why there is no checkout (shown to the reader with the brief).
    DiffOnly(String),
}

/// Preparing the code was cancelled with the run.
pub struct CodeCancelled;

impl BriefCode {
    /// Checks out `head_sha` of `pull_request`'s repository into a fresh
    /// workspace with `change_diff` (the loaded patches) beside it,
    /// reporting progress. Any failure gives a brief from the diffs alone,
    /// saying why; only a cancelled run is an error. Blocks; call it on the
    /// background executor.
    pub fn prepare(
        services: &AppServices,
        pull_request: &PullRequest,
        head_sha: Option<&str>,
        change_diff: &str,
        cancel: &AtomicBool,
        report: &mut dyn FnMut(AgentProgress),
    ) -> Result<Self, CodeCancelled> {
        // The client, the folder and the commit, else the diffs alone.
        let diff_only = |reason: &str| Ok(BriefCode::DiffOnly(reason.to_owned()));
        let Ok(client) = &services.github else {
            return diff_only("no GitHub token");
        };
        let Some(root) = workspace_root() else {
            return diff_only("no cache folder");
        };
        let Some(head_sha) = head_sha else {
            return diff_only("its head commit is not known yet");
        };
        report(AgentProgress::FetchingCode { percent: None });

        // The head, shallow, into a workspace of its own.
        let source = WorkspaceSource::github(
            &pull_request.repo.full_name(),
            head_sha,
            Some(client.token().to_owned()),
        );
        let mut on_step = |step| {
            report(match step {
                WorkspaceProgress::Fetching { percent } => AgentProgress::FetchingCode { percent },
                WorkspaceProgress::Unpacking => AgentProgress::UnpackingCode,
            });
        };
        match prepare_workspace(&root, &source, change_diff, cancel, &mut on_step) {
            Ok(workspace) => Ok(BriefCode::Workspace(workspace)),
            Err(WorkspaceError::Cancelled) => Err(CodeCancelled),
            Err(error) => diff_only(&format!("the checkout failed: {error}")),
        }
    }

    /// What the agent is told it can read.
    pub fn access(&self) -> CodeAccess {
        match self {
            Self::Workspace(_) => CodeAccess::Workspace,
            Self::DiffOnly(reason) => CodeAccess::DiffOnly(reason.clone()),
        }
    }

    /// The workspace folder the agent runs in, with a checkout.
    pub fn workspace(&self) -> Option<&Path> {
        match self {
            Self::Workspace(workspace) => Some(workspace.path()),
            Self::DiffOnly(_) => None,
        }
    }
}
