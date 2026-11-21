use std::sync::Arc;

use futures::{StreamExt as _, channel::mpsc};
use gpui::Context;
use qd_cache::ReviewScope;
use qd_studio::{ReaderFamiliarity, SmartFilter, find_claude};

use super::pending_guided_review::PendingGuidedReview;
use super::pull_request_detail::PullRequestDetail;
use super::review_agent_task::ReviewAgentTask;
use super::review_job_error::ReviewJobError;
use super::review_publication_gate::ReviewPublicationGate;
use super::review_run_id::new_review_run_id;
use super::smart_filter_job::SmartFilterJob;
use super::smart_filter_state::SmartFilterState;
use super::studio_request::studio_request;
use crate::ui_state::UiStateRecorder;

impl PullRequestDetail {
    pub(super) fn current_smart_filter(&self) -> Option<&SmartFilterState> {
        self.smart_filters.get(self.review_scope()?)
    }

    pub(super) fn smart_filter_is_stale(&self, filter: &SmartFilter) -> bool {
        self.shown_head_sha()
            .is_none_or(|head| filter.is_stale(head, self.shown_base_sha.as_deref()))
    }

    /// Installs a saved plan after loading a scope, without overwriting a run
    /// or a more recent result already held in this session.
    pub(super) fn restore_smart_filter(&mut self, saved: Option<SmartFilter>) {
        if let Some(scope) = self.review_scope().cloned() {
            let state = self.smart_filters.entry(scope).or_default();
            if state.filter.is_none() {
                state.filter = saved;
            }
        }
        self.reset_guided_walkthrough();
    }

    /// Clears step-specific navigation when a saved guide is loaded or replaced.
    fn reset_guided_walkthrough(&mut self) {
        self.clear_diff_selection();
        self.row_tops_width = None;
        self.walkthrough.selected_group = None;
        self.walkthrough.current_item = None;
        self.walkthrough.expanded_items.clear();
    }

    /// Starts only on an explicit click, and never duplicates an active run
    /// for the same scope, even if the reader navigates away and comes back.
    pub(super) fn generate_smart_filter(&mut self, cx: &mut Context<Self>) {
        let (Some(pull_request), Some(scope), Some(head_sha)) = (
            self.pull_request().cloned(),
            self.review_scope().cloned(),
            self.shown_head_sha().map(str::to_owned),
        ) else {
            return;
        };
        if self.loaded_files().is_empty()
            || self
                .current_smart_filter()
                .is_some_and(|state| state.run.is_some())
        {
            return;
        }
        let Some(program) = find_claude() else {
            self.smart_filters.entry(scope).or_default().error = Some(
                "Claude Code was not found. Install it from claude.ai/code and log in once in a terminal, then try again.".into());
            cx.notify();
            return;
        };
        let run_id = new_review_run_id();
        let gate = Arc::new(ReviewPublicationGate::default());
        let job = SmartFilterJob {
            run_id: run_id.clone(),
            gate: gate.clone(),
            services: self.services().clone(),
            request: studio_request(
                &pull_request,
                self.target(),
                self.review_layers(),
                self.loaded_files(),
                ReaderFamiliarity::default(),
            ),
            pull_request,
            scope: scope.clone(),
            head_sha: head_sha.clone(),
            base_sha: self.shown_base_sha.clone(),
            program,
            files: self.loaded_files().to_vec(),
            store: cx.global::<UiStateRecorder>().shared_store(),
        };
        let task_run_id = run_id.clone();
        let base_sha = self.shown_base_sha.clone();
        let run_scope = scope.clone();
        let task = cx.spawn(async move |detail, cx| {
            let (sender, mut progress) = mpsc::unbounded();
            let run = cx.background_executor().spawn(async move {
                job.run(&mut |step| {
                    sender.unbounded_send(step).ok();
                })
            });
            while let Some(step) = progress.next().await {
                if detail
                    .update(cx, |detail, cx| {
                        if let Some(state) = detail.smart_filters.get_mut(&scope)
                            && state
                                .run
                                .as_ref()
                                .is_some_and(|run| run.run_id == task_run_id)
                        {
                            state.progress = Some(step);
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    return;
                }
            }
            let outcome = run.await;
            detail
                .update(cx, |detail, cx| {
                    detail.finish_smart_filter_run(&scope, &task_run_id, outcome, cx);
                })
                .ok();
        });
        let state = self.smart_filters.entry(run_scope.clone()).or_default();
        state.error = None;
        state.progress = None;
        state.pending_save = None;
        state.saving = false;
        state.run = Some(ReviewAgentTask {
            run_id,
            scope: run_scope,
            head_sha: Some(head_sha),
            base_sha,
            gate,
            _task: task,
        });
        cx.notify();
    }

    /// Applies only the generation still owned by this scope; failures retain its saved guide.
    pub(super) fn finish_smart_filter_run(
        &mut self,
        scope: &ReviewScope,
        run_id: &str,
        outcome: Result<SmartFilter, ReviewJobError<PendingGuidedReview>>,
        cx: &mut Context<Self>,
    ) {
        let state = self.smart_filters.entry(scope.clone()).or_default();
        let published = outcome.is_ok();
        if !state.finish_run(run_id, outcome) {
            return;
        }
        if published && self.review_scope() == Some(scope) {
            self.reset_guided_walkthrough();
            self.rebuild_rows(self.anchor_at_top());
            self.resume_guided_review(cx);
        }
        cx.notify();
    }

    pub(super) fn cancel_smart_filter(&mut self, cx: &mut Context<Self>) {
        if let Some(scope) = self.review_scope().cloned()
            && let Some(state) = self.smart_filters.get_mut(&scope)
        {
            state.cancel();
        }
        cx.notify();
    }

    /// A refreshed snapshot invalidates only its own scope's in-flight guide.
    pub(super) fn invalidate_guided_review(
        &mut self,
        scope: &qd_cache::ReviewScope,
        head_sha: Option<&str>,
        base_sha: Option<&str>,
    ) {
        if let Some(state) = self.smart_filters.get_mut(scope) {
            state.invalidate(head_sha, base_sha);
        }
    }
}
