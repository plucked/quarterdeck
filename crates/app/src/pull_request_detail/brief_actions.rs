use std::sync::Arc;

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui::Context;
use qd_studio::{ReaderFamiliarity, find_claude};
use qd_update::AppVersion;

use super::brief_job::BriefJob;
use super::brief_key::BriefKey;
use super::brief_pane::ShownBrief;
use super::brief_row::BriefRow;
use super::detail_event::DetailEvent;
use super::pull_request_detail::PullRequestDetail;
use super::review_agent_task::ReviewAgentTask;
use super::review_publication_gate::ReviewPublicationGate;
use super::review_run_id::new_review_run_id;
use super::studio_request::studio_request;
use crate::brief_view::{BriefPageRow, BriefState, PreparedBrief, ReportHost};
use crate::ui_state::UiStateRecorder;

/// Shown when no `claude` executable is found.
const CLAUDE_MISSING: &str = "Claude Code was not found. Install it (claude.ai/code) so `claude` is on PATH, in ~/.local/bin or in Homebrew's folder, and log in once in a terminal.";

impl PullRequestDetail {
    /// Which brief the shown pull request and target have; `None` without
    /// a pull request.
    pub(super) fn shown_brief_key(&self) -> Option<BriefKey> {
        Some(BriefKey {
            pull_request_id: self.pull_request()?.id,
            target: self.target().clone(),
        })
    }

    /// The shown pull request's brief at the shown target, if one was started.
    pub(super) fn current_brief(&self) -> Option<&BriefState> {
        self.brief().state_of(&self.shown_brief_key()?)
    }

    /// The page shown of the shown brief.
    pub(super) fn current_brief_page(&self) -> usize {
        self.shown_brief_key()
            .map_or(0, |key| self.brief().page_of(&key))
    }

    /// Shows `brief`, read from storage, as the shown pull request's brief
    /// at the shown target, unless this session already has one there (a
    /// run in flight, or one written since).
    pub(super) fn show_stored_brief(&mut self, brief: PreparedBrief) {
        let Some(key) = self.shown_brief_key() else {
            return;
        };
        if self.brief().state_of(&key).is_none() {
            self.brief_mut()
                .set_state(&key, BriefState::Written(Box::new(brief)));
        }
    }

    /// Lays the shown brief's diagrams out for a column `width` wide,
    /// measured by the brief area's probe when the reader resizes the pane.
    pub(super) fn set_brief_column_width(&mut self, width: f32, cx: &mut Context<Self>) {
        self.brief_mut().set_column_width(width);
        cx.notify();
    }

    /// Lays out the brief's rows for the shown pull request and target:
    /// the status row until a brief is written, then the shown page.
    pub(super) fn refresh_brief_rows(&mut self) {
        let shown = self.shown_brief_key().map(|key| ShownBrief {
            page: self.brief().page_of(&key),
            key,
        });
        let rows = match self.current_brief() {
            Some(BriefState::Written(brief)) => {
                BriefPageRow::rows_of(brief, self.current_brief_page())
                    .into_iter()
                    .map(BriefRow::Page)
                    .collect()
            }
            Some(BriefState::Writing(_) | BriefState::Failed(_)) | None => vec![BriefRow::Status],
        };
        self.brief_mut().set_rows(shown, rows);
    }

    /// How well the reader knows the shown pull request's repository, as
    /// last picked this session.
    pub(super) fn current_familiarity(&self) -> ReaderFamiliarity {
        self.pull_request()
            .map_or_else(ReaderFamiliarity::default, |pull_request| {
                self.brief().familiarity_for(&pull_request.repo.full_name())
            })
    }

    /// Remembers how well the reader knows the shown pull request's
    /// repository; the status card or footer showing the picker is laid out
    /// again, since the line explaining the choice changes.
    pub(super) fn choose_familiarity(
        &mut self,
        familiarity: ReaderFamiliarity,
        cx: &mut Context<Self>,
    ) {
        let Some(repository) = self
            .pull_request()
            .map(|pull_request| pull_request.repo.full_name())
        else {
            return;
        };
        self.brief_mut().set_familiarity(repository, familiarity);
        self.brief_mut().remeasure(BriefRow::Status);
        self.brief_mut()
            .remeasure(BriefRow::Page(BriefPageRow::Footer));
        cx.notify();
    }

    /// Writes a report on the shown brief with the words in the report
    /// panel and the page shown ([`ReportHost::save_report`]).
    pub(super) fn save_shown_brief_report(&mut self, cx: &mut Context<Self>) {
        let page = self.current_brief_page();
        if let Some(BriefState::Written(brief)) = self.current_brief() {
            let record = brief.record.clone();
            let shown_page = brief.shown_page(page);
            self.save_report(record, shown_page, app_version(), cx);
        }
    }

    /// Shows page `page` of the shown brief, from its top.
    pub(super) fn show_brief_page(&mut self, page: usize, cx: &mut Context<Self>) {
        let Some(key) = self.shown_brief_key() else {
            return;
        };
        self.brief_mut().set_page(&key, page);
        self.refresh_brief_rows();
        cx.notify();
    }

    /// Starts a scoped MCP draft while retaining a published brief and its page.
    pub(super) fn write_brief(&mut self, cx: &mut Context<Self>) {
        let Some(pull_request) = self.pull_request().cloned() else {
            return;
        };
        if self.loaded_files().is_empty() {
            return;
        }
        let key = BriefKey {
            pull_request_id: pull_request.id,
            target: self.target().clone(),
        };
        if self.brief().run_of(&key).is_some() {
            return;
        }
        let Some(program) = find_claude() else {
            self.brief_mut().fail_to_start(&key, CLAUDE_MISSING.into());
            self.refresh_brief_rows();
            cx.notify();
            return;
        };
        let run_id = new_review_run_id();
        let gate = Arc::new(ReviewPublicationGate::default());
        let head_sha = self.shown_head_sha().map(str::to_owned);
        let base_sha = self.shown_base_sha.clone();
        let scope = key.target.viewed_scope(&pull_request);
        let job = BriefJob {
            run_id: run_id.clone(),
            gate: gate.clone(),
            base_sha: base_sha.clone(),
            services: self.services().clone(),
            request: studio_request(
                &pull_request,
                &key.target,
                self.review_layers(),
                self.loaded_files(),
                self.current_familiarity(),
            ),
            pull_request,
            target: key.target.clone(),
            head_sha: head_sha.clone(),
            program,
            files: self.loaded_files().to_vec(),
            store: cx.global::<UiStateRecorder>().shared_store(),
        };
        let run_key = key.clone();
        let task_run_id = run_id.clone();
        let task = cx.spawn(async move |detail, cx| {
            let (progress_sender, mut progress) = mpsc::unbounded();
            let run = cx.background_executor().spawn(async move {
                job.run(&mut |step| {
                    progress_sender.unbounded_send(step).ok();
                })
            });
            while let Some(step) = progress.next().await {
                if detail
                    .update(cx, |detail, cx| {
                        detail
                            .brief_mut()
                            .update_run_progress(&key, &task_run_id, step);
                        detail.refresh_brief_rows();
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
                    let published = outcome.is_ok();
                    if detail.brief_mut().finish_run(&key, &task_run_id, outcome) {
                        if published {
                            cx.emit(DetailEvent::BriefStored);
                        }
                        detail.refresh_brief_rows();
                        cx.notify();
                    }
                })
                .ok();
        });
        self.brief_mut().start_run(
            &run_key,
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

    /// Stops the shown brief's run (killing the agent); the brief it was
    /// replacing shows again, if there was one.
    pub(super) fn cancel_brief(&mut self, cx: &mut Context<Self>) {
        if let Some(key) = self.shown_brief_key() {
            self.brief_mut().cancel_run(&key);
        }
        self.refresh_brief_rows();
        cx.notify();
    }
}

/// This build's version for a report: the package version, marked as a
/// development build when it is one.
fn app_version() -> String {
    let version = AppVersion::current();
    if cfg!(debug_assertions) {
        format!("{version} (development build)")
    } else {
        version.to_string()
    }
}
