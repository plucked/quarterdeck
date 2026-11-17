use std::sync::Arc;

use gpui::Context;

use super::detail_event::DetailEvent;
use super::pull_request_detail::PullRequestDetail;
use super::review_agent_task::ReviewAgentTask;
use super::review_publication_gate::ReviewPublicationGate;
use crate::ui_state::UiStateRecorder;

impl PullRequestDetail {
    /// Retries only the completed brief's local transaction, retaining its original receipt.
    pub(super) fn retry_brief_save(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.shown_brief_key() else {
            return;
        };
        let Some(draft) = self.brief().pending_save_of(&key) else {
            return;
        };
        if self.brief().run_of(&key).is_some()
            || draft.head_sha() != self.shown_head_sha()
            || draft.base_sha() != self.shown_base_sha.as_deref()
        {
            return;
        }
        let Some(draft) = self.brief_mut().take_pending_save(&key) else {
            return;
        };
        // A fresh attempt gate preserves cancellation without reserving a new generation.
        let run_id = draft.run_id().to_owned();
        let scope = draft.scope().clone();
        let head_sha = draft.head_sha().map(str::to_owned);
        let base_sha = draft.base_sha().map(str::to_owned);
        let gate = Arc::new(ReviewPublicationGate::default());
        let save_gate = gate.clone();
        let store = cx.global::<UiStateRecorder>().shared_store();
        let task_key = key.clone();
        let task_run_id = run_id.clone();
        let task = cx.spawn(async move |detail, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move { draft.save(&store, &save_gate) })
                .await;
            detail
                .update(cx, |detail, cx| {
                    let published = outcome.is_ok();
                    if detail
                        .brief_mut()
                        .finish_run(&task_key, &task_run_id, outcome)
                    {
                        if published {
                            cx.emit(DetailEvent::BriefStored);
                        }
                        detail.refresh_brief_rows();
                        cx.notify();
                    }
                })
                .ok();
        });
        self.brief_mut().start_save(
            &key,
            ReviewAgentTask {
                run_id,
                scope,
                head_sha,
                base_sha,
                gate,
                _task: task,
            },
        );
        self.refresh_brief_rows();
        cx.notify();
    }

    /// Saves the retained guide without checking out code, reserving a run or calling Claude.
    pub(super) fn retry_guided_review_save(&mut self, cx: &mut Context<Self>) {
        let Some(scope) = self.review_scope().cloned() else {
            return;
        };
        let Some(state) = self.smart_filters.get(&scope) else {
            return;
        };
        let Some(draft) = state.pending_save.as_ref() else {
            return;
        };
        if state.run.is_some()
            || Some(draft.head_sha()) != self.shown_head_sha()
            || draft.base_sha() != self.shown_base_sha.as_deref()
        {
            return;
        }
        let Some(draft) = self
            .smart_filters
            .get_mut(&scope)
            .and_then(|state| state.pending_save.take())
        else {
            return;
        };
        // Retry the original generation; a different publisher is still rejected by SQLite.
        let run_id = draft.run_id().to_owned();
        let scope = draft.scope().clone();
        let head_sha = Some(draft.head_sha().to_owned());
        let base_sha = draft.base_sha().map(str::to_owned);
        let gate = Arc::new(ReviewPublicationGate::default());
        let save_gate = gate.clone();
        let store = cx.global::<UiStateRecorder>().shared_store();
        let task_scope = scope.clone();
        let task_run_id = run_id.clone();
        let task = cx.spawn(async move |detail, cx| {
            let outcome = cx
                .background_executor()
                .spawn(async move { draft.save(&store, &save_gate) })
                .await;
            detail
                .update(cx, |detail, cx| {
                    detail.finish_smart_filter_run(&task_scope, &task_run_id, outcome, cx);
                })
                .ok();
        });
        let state = self.smart_filters.entry(scope.clone()).or_default();
        state.run = Some(ReviewAgentTask {
            run_id,
            scope,
            head_sha,
            base_sha,
            gate,
            _task: task,
        });
        state.error = None;
        state.progress = None;
        state.saving = true;
        cx.notify();
    }
}
