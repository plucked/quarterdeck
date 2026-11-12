use futures::{StreamExt as _, channel::mpsc};
use gpui::Context;
use qd_studio::{ReaderFamiliarity, SmartFilter, find_claude};

use super::pull_request_detail::PullRequestDetail;
use super::run_cancel::RunCancel;
use super::smart_filter_job::SmartFilterJob;
use super::smart_filter_state::SmartFilterState;
use super::studio_request::studio_request;
use crate::facet_chips::ChipClick;
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
        self.sync_smart_buckets();
    }

    fn sync_smart_buckets(&mut self) {
        let buckets = self
            .current_smart_filter()
            .and_then(|state| state.filter.as_ref())
            .map(|filter| filter.buckets.clone())
            .unwrap_or_default();
        self.file_tree.set_smart_buckets(buckets);
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
        let job = SmartFilterJob {
            services: self.services().clone(),
            request: studio_request(
                &pull_request,
                self.target(),
                self.stack(),
                self.loaded_files(),
                ReaderFamiliarity::default(),
            ),
            pull_request,
            scope: scope.clone(),
            head_sha,
            base_sha: self.shown_base_sha.clone(),
            program,
            files: self.loaded_files().to_vec(),
            store: cx.global::<UiStateRecorder>().shared_store(),
        };
        let (cancel, flag) = RunCancel::new();
        let run_scope = scope.clone();
        let task = cx.spawn(async move |detail, cx| {
            let (sender, mut progress) = mpsc::unbounded();
            let run = cx.background_executor().spawn(async move {
                job.run(&flag, &mut |step| {
                    sender.unbounded_send(step).ok();
                })
            });
            while let Some(step) = progress.next().await {
                if detail
                    .update(cx, |detail, cx| {
                        detail
                            .smart_filters
                            .entry(scope.clone())
                            .or_default()
                            .progress = Some(step);
                        cx.notify();
                    })
                    .is_err()
                {
                    return;
                }
            }
            let outcome = run.await;
            detail
                .update(cx, |detail, cx| {
                    let state = detail.smart_filters.entry(scope.clone()).or_default();
                    state.run = None;
                    state.progress = None;
                    match outcome {
                        Ok((filter, warning)) => {
                            state.filter = Some(filter);
                            state.error = warning;
                        }
                        Err(error) => state.error = Some(error),
                    }
                    if detail.review_scope() == Some(&scope) {
                        detail.sync_smart_buckets();
                        detail.rebuild_rows(detail.anchor_at_top());
                    }
                    cx.notify();
                })
                .ok();
        });
        let state = self.smart_filters.entry(run_scope).or_default();
        state.error = None;
        state.progress = None;
        state.run = Some((task, cancel));
        cx.notify();
    }

    pub(super) fn cancel_smart_filter(&mut self, cx: &mut Context<Self>) {
        if let Some(scope) = self.review_scope().cloned()
            && let Some(state) = self.smart_filters.get_mut(&scope)
        {
            state.run = None;
            state.progress = None;
        }
        cx.notify();
    }

    pub(super) fn click_smart_bucket(
        &mut self,
        order: u32,
        click: ChipClick,
        cx: &mut Context<Self>,
    ) {
        self.file_tree.click_smart_bucket(order, click);
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }

    /// Bulk marks are always a user action, and require the original commits.
    pub(super) fn toggle_smart_bucket_viewed(&mut self, order: u32, cx: &mut Context<Self>) {
        let Some(scope) = self.review_scope().cloned() else {
            return;
        };
        let Some(filter) = self
            .current_smart_filter()
            .and_then(|state| state.filter.as_ref())
        else {
            return;
        };
        if self.smart_filter_is_stale(filter) {
            return;
        }
        let Some(bucket) = filter
            .buckets
            .iter()
            .find(|bucket| bucket.order == order && bucket.suggest_viewed)
        else {
            return;
        };
        let members: std::collections::HashSet<_> =
            bucket.files.iter().map(String::as_str).collect();
        let files: Vec<_> = self
            .loaded_files()
            .iter()
            .filter(|file| members.contains(file.filename()))
            .cloned()
            .collect();
        let viewed = !files
            .iter()
            .all(|file| self.file_review.is_viewed(file.filename()));
        for file in files {
            if let Some(toggle) = self.file_review.set_viewed(&file, viewed) {
                toggle.record(scope.clone(), cx);
            }
        }
        self.rebuild_rows(self.anchor_at_top());
        cx.notify();
    }
}
