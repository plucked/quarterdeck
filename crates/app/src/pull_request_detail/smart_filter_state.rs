use qd_studio::{AgentProgress, SmartFilter};

use super::pending_guided_review::PendingGuidedReview;
use super::review_agent_task::ReviewAgentTask;
use super::review_job_error::ReviewJobError;

/// One scope's saved plan and optional replacement in flight. Keeping the
/// old plan separate means cancellation or failure never loses its buckets.
#[derive(Default)]
pub(super) struct SmartFilterState {
    pub filter: Option<SmartFilter>,
    pub progress: Option<AgentProgress>,
    pub error: Option<String>,
    pub run: Option<ReviewAgentTask>,
    pub pending_save: Option<Box<PendingGuidedReview>>,
    pub saving: bool,
}

impl SmartFilterState {
    /// Applies only the current generation, retaining completed output after a save failure.
    pub fn finish_run(
        &mut self,
        run_id: &str,
        outcome: Result<SmartFilter, ReviewJobError<PendingGuidedReview>>,
    ) -> bool {
        if self.run.as_ref().is_none_or(|run| run.run_id != run_id) {
            return false;
        }
        // Drop only the attempt; a retained draft uses a fresh gate for each retry.
        self.run = None;
        self.progress = None;
        self.saving = false;
        self.pending_save = None;
        match outcome {
            Ok(filter) => {
                self.filter = Some(filter);
                self.error = None;
            }
            Err(ReviewJobError::Failed(message)) => self.error = Some(message),
            Err(ReviewJobError::SaveFailed { draft, message }) => {
                self.pending_save = Some(draft);
                self.error = Some(message);
            }
        }
        true
    }

    /// Cancels a running attempt or discards unsaved output, leaving the saved guide intact.
    pub fn cancel(&mut self) {
        if self
            .run
            .as_ref()
            .map_or(self.pending_save.is_some(), |run| run.gate.cancel())
        {
            self.run = None;
            self.progress = None;
            self.pending_save = None;
            self.saving = false;
            self.error = None;
        }
    }

    /// Invalidates pending saves even if a snapshot changed while a transaction was in flight.
    pub fn invalidate(&mut self, head_sha: Option<&str>, base_sha: Option<&str>) {
        let invalidated_run = self.run.as_ref().is_some_and(|run| {
            (run.head_sha.as_deref() != head_sha || run.base_sha.as_deref() != base_sha)
                && run.gate.invalidate()
        });
        let invalidated_save = self.pending_save.as_ref().is_some_and(|draft| {
            Some(draft.head_sha()) != head_sha || draft.base_sha() != base_sha
        });
        if invalidated_run || invalidated_save {
            self.run = None;
            self.progress = None;
            self.pending_save = None;
            self.saving = false;
            self.error = Some("The diff changed while creating the guide. Create a new guided review for this revision.".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    use gpui::Task;
    use qd_cache::{GuidedReviewPublication, ReviewScope};
    use qd_studio::{GuidedToolAudit, SmartBucket};

    use super::*;
    use crate::pull_request_detail::review_publication_gate::ReviewPublicationGate;

    /// A saved artifact whose contents identify which generation is visible.
    fn filter(label: &str) -> SmartFilter {
        SmartFilter {
            head_sha: "head".into(),
            base_sha: Some("base".into()),
            generated_at: 42,
            buckets: vec![SmartBucket {
                order: 1,
                label: label.into(),
                description: "Read the changed behavior.".into(),
                suggest_viewed: false,
                items: vec![],
            }],
        }
    }

    /// A completed task exercises lifecycle transitions without a window or process.
    fn run(run_id: &str) -> ReviewAgentTask {
        ReviewAgentTask {
            run_id: run_id.into(),
            scope: ReviewScope::PullRequest(42),
            head_sha: Some("head".into()),
            base_sha: Some("base".into()),
            gate: Arc::new(ReviewPublicationGate::default()),
            _task: Task::ready(()),
        }
    }

    /// A sealed replacement carries original publication metadata through every retry.
    fn pending(run_id: &str) -> Box<PendingGuidedReview> {
        Box::new(PendingGuidedReview::new(
            ReviewScope::PullRequest(42),
            filter(run_id),
            GuidedReviewPublication {
                run_id: run_id.into(),
                snapshot_id: "head:base".into(),
                validation_receipt: format!("receipt-{run_id}"),
                cost_usd: Some(0.125),
                session_id: Some("completed-session".into()),
                model: Some("completed-model".into()),
                duration_ms: 2400,
                audit: vec![GuidedToolAudit {
                    tool_name: "guide_finish".into(),
                    status: "ready".into(),
                    revision: 2,
                    issue_codes: vec![],
                }],
            },
        ))
    }

    /// Creates the state after generation completed but durable publication failed.
    fn awaiting_save(run_id: &str) -> SmartFilterState {
        let mut state = SmartFilterState {
            filter: Some(filter("saved")),
            run: Some(run(run_id)),
            ..SmartFilterState::default()
        };
        assert!(state.finish_run(
            run_id,
            Err(ReviewJobError::SaveFailed {
                draft: pending(run_id),
                message: "Disk unavailable".into(),
            }),
        ));
        state
    }

    /// Repeated save failures retain the exact completed draft and old guide until publication succeeds.
    #[test]
    fn save_retries_keep_the_completed_draft_and_saved_guide() {
        // Retain the original allocation, including its artifact and publication metadata.
        let mut state = awaiting_save("completed");
        let original = std::ptr::from_ref(state.pending_save.as_deref().unwrap());
        assert_eq!(state.filter, Some(filter("saved")));
        assert_eq!(state.error.as_deref(), Some("Disk unavailable"));

        // Save-only attempts reuse the same identity and return ownership on each failure.
        for message in ["Still unavailable", "Storage remains unavailable"] {
            let draft = state.pending_save.take().unwrap();
            assert_eq!(std::ptr::from_ref(draft.as_ref()), original);
            assert_eq!(draft.run_id(), "completed");
            assert_eq!(draft.scope(), &ReviewScope::PullRequest(42));
            assert_eq!(draft.head_sha(), "head");
            assert_eq!(draft.base_sha(), Some("base"));
            state.run = Some(run(draft.run_id()));
            state.saving = true;
            state.error = None;
            assert!(state.finish_run(
                "completed",
                Err(ReviewJobError::SaveFailed {
                    draft,
                    message: message.into(),
                }),
            ));
            assert_eq!(state.filter, Some(filter("saved")));
            assert_eq!(state.error.as_deref(), Some(message));
            assert_eq!(
                std::ptr::from_ref(state.pending_save.as_deref().unwrap()),
                original
            );
            assert!(state.run.is_none());
            assert!(state.progress.is_none());
            assert!(!state.saving);
        }

        // Only successful durable publication replaces the saved artifact and removes retry state.
        let draft = state.pending_save.take().unwrap();
        state.run = Some(run(draft.run_id()));
        state.saving = true;
        assert!(state.finish_run("completed", Ok(filter("completed"))));
        assert_eq!(state.filter, Some(filter("completed")));
        assert!(state.pending_save.is_none());
        assert!(state.run.is_none());
        assert!(state.error.is_none());
        assert!(!state.saving);
    }

    /// A terminal generation or ownership failure preserves saved content without offering a save retry.
    #[test]
    fn terminal_failure_keeps_the_saved_guide_without_a_pending_save() {
        let mut state = SmartFilterState {
            filter: Some(filter("saved")),
            run: Some(run("failed")),
            progress: Some(AgentProgress::Thinking),
            saving: true,
            ..SmartFilterState::default()
        };
        assert!(state.finish_run("failed", Err(ReviewJobError::Failed("Superseded".into()))));
        assert_eq!(state.filter, Some(filter("saved")));
        assert_eq!(state.error.as_deref(), Some("Superseded"));
        assert!(state.run.is_none());
        assert!(state.progress.is_none());
        assert!(state.pending_save.is_none());
        assert!(!state.saving);
    }

    /// Discard clears a pending first guide or replacement and rejects any late failed-save callback.
    #[test]
    fn cancelling_pending_output_preserves_only_the_saved_guide() {
        for saved in [None, Some(filter("saved"))] {
            let mut state = awaiting_save("completed");
            state.filter = saved.clone();
            state.cancel();
            assert_eq!(state.filter, saved);
            assert!(state.pending_save.is_none());
            assert!(state.error.is_none());
            assert!(state.run.is_none());
            assert!(!state.saving);

            // A completion queued before discard cannot restore the discarded draft.
            assert!(!state.finish_run(
                "completed",
                Err(ReviewJobError::SaveFailed {
                    draft: pending("completed"),
                    message: "Late failure".into(),
                })
            ));
            assert!(state.pending_save.is_none());
            assert_eq!(state.filter, saved);
        }
    }

    /// Cancelling an active save retry revokes its gate and refuses its returned draft.
    #[test]
    fn cancelling_a_save_attempt_rejects_its_late_result() {
        let mut state = awaiting_save("completed");
        let draft = state.pending_save.take().unwrap();
        state.run = Some(run(draft.run_id()));
        state.saving = true;
        let gate = state.run.as_ref().unwrap().gate.clone();

        state.cancel();
        assert!(gate.cancellation_flag().load(Ordering::SeqCst));
        assert!(state.run.is_none());
        assert!(!state.saving);
        assert!(!state.finish_run(
            "completed",
            Err(ReviewJobError::SaveFailed {
                draft,
                message: "Save cancelled".into(),
            })
        ));
        assert!(state.pending_save.is_none());
        assert!(state.error.is_none());
        assert_eq!(state.filter, Some(filter("saved")));
    }

    /// Either commit changing removes a pending save; an unchanged snapshot preserves it.
    #[test]
    fn pending_save_is_bound_to_both_snapshot_commits() {
        for (head_sha, base_sha) in [("new-head", "base"), ("head", "new-base")] {
            let mut state = awaiting_save("completed");
            state.invalidate(Some("head"), Some("base"));
            assert_eq!(state.pending_save.as_ref().unwrap().run_id(), "completed");
            assert_eq!(state.error.as_deref(), Some("Disk unavailable"));

            state.invalidate(Some(head_sha), Some(base_sha));
            assert!(state.pending_save.is_none());
            assert!(state.run.is_none());
            assert!(!state.saving);
            assert!(state.error.as_deref().unwrap().contains("diff changed"));
            assert_eq!(state.filter, Some(filter("saved")));
            assert!(!state.finish_run("completed", Ok(filter("stale"))));
        }
    }

    /// Refresh during a save-only attempt cancels either changed commit and rejects its late draft.
    #[test]
    fn changed_snapshot_revokes_an_active_save_retry() {
        for (head_sha, base_sha) in [("new-head", "base"), ("head", "new-base")] {
            let mut state = awaiting_save("completed");
            let draft = state.pending_save.take().unwrap();
            state.run = Some(run(draft.run_id()));
            state.saving = true;
            let gate = state.run.as_ref().unwrap().gate.clone();

            state.invalidate(Some(head_sha), Some(base_sha));
            assert!(gate.cancellation_flag().load(Ordering::SeqCst));
            assert!(state.run.is_none());
            assert!(!state.saving);
            assert!(!state.finish_run(
                "completed",
                Err(ReviewJobError::SaveFailed {
                    draft,
                    message: "Late storage failure".into(),
                })
            ));
            assert!(state.pending_save.is_none());
            assert!(state.error.as_deref().unwrap().contains("diff changed"));
            assert_eq!(state.filter, Some(filter("saved")));
        }
    }

    /// Stale success and failure callbacks cannot replace a newer active run or retained draft.
    #[test]
    fn stale_callbacks_leave_newer_generation_state_untouched() {
        let mut state = SmartFilterState {
            filter: Some(filter("saved")),
            run: Some(run("newer")),
            progress: Some(AgentProgress::Thinking),
            ..SmartFilterState::default()
        };
        assert!(!state.finish_run("older", Ok(filter("stale"))));
        assert!(!state.finish_run(
            "older",
            Err(ReviewJobError::SaveFailed {
                draft: pending("older"),
                message: "Stale failure".into(),
            })
        ));
        assert_eq!(state.run.as_ref().unwrap().run_id, "newer");
        assert_eq!(state.progress, Some(AgentProgress::Thinking));
        assert!(state.pending_save.is_none());
        assert!(state.error.is_none());

        // Once the newer result is retained, stale callbacks cannot change its retry identity or error.
        assert!(state.finish_run(
            "newer",
            Err(ReviewJobError::SaveFailed {
                draft: pending("newer"),
                message: "New save failed".into(),
            })
        ));
        let retained = std::ptr::from_ref(state.pending_save.as_deref().unwrap());
        assert!(!state.finish_run(
            "older",
            Err(ReviewJobError::SaveFailed {
                draft: pending("older"),
                message: "Old failure".into(),
            })
        ));
        assert!(!state.finish_run("older", Ok(filter("stale"))));
        assert_eq!(
            std::ptr::from_ref(state.pending_save.as_deref().unwrap()),
            retained
        );
        assert_eq!(state.pending_save.as_ref().unwrap().run_id(), "newer");
        assert_eq!(state.error.as_deref(), Some("New save failed"));
        assert_eq!(state.filter, Some(filter("saved")));
    }
}
