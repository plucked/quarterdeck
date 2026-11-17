use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

use gpui::Task;
use qd_cache::ReviewScope;
use qd_studio::{AgentProgress, BriefRecord, CodeAccess, ReaderFamiliarity, Studio, StudioPage};

use super::brief_key::BriefKey;
use super::brief_pane::BriefPane;
use super::pending_brief::PendingBrief;
use super::review_agent_task::ReviewAgentTask;
use super::review_job_error::ReviewJobError;
use super::review_publication_gate::ReviewPublicationGate;
use super::review_target::ReviewTarget;
use crate::brief_view::BriefSaveStatus;
use crate::brief_view::{BriefHandlers, BriefState, PreparedBrief};

/// A window-free pane exercises publication behavior without any model invocation.
fn pane() -> BriefPane {
    BriefPane::new(BriefHandlers {
        show_anchor: Rc::new(|_, _, _| {}),
        write: Rc::new(|_, _| {}),
        retry_save: Rc::new(|_, _| {}),
        cancel: Rc::new(|_, _| {}),
        show_page: Rc::new(|_, _, _| {}),
        choose_reader: Rc::new(|_, _, _| {}),
        open_report: Rc::new(|_, _| {}),
        save_report: Rc::new(|_, _| {}),
        close_report: Rc::new(|_, _| {}),
        copy_report_path: Rc::new(|_, _| {}),
    })
}

fn key() -> BriefKey {
    BriefKey {
        pull_request_id: 42,
        target: ReviewTarget::Layer,
    }
}

/// A visible two-page artifact distinguished by its saved subject.
fn brief(subject: &str) -> PreparedBrief {
    PreparedBrief::prepare(
        Studio {
            lead: subject.into(),
            pages: (0..2)
                .map(|index| StudioPage {
                    title: format!("Page {index}"),
                    summary: "A saved page".into(),
                    widgets: vec![],
                })
                .collect(),
        },
        BriefRecord {
            mcp: None,
            repository: "test/repo".into(),
            number: 42,
            url: "https://example.invalid/42".into(),
            subject: subject.into(),
            layers: vec![],
            head_sha: Some("head".into()),
            reader: ReaderFamiliarity::default(),
            code: CodeAccess::Workspace,
            prompt: String::new(),
            answer: None,
            transcript: None,
            tool_calls: vec![],
            rejected: vec![],
            cost_usd: None,
            session_id: None,
            model: None,
            duration: Duration::ZERO,
            written_at: 1,
        },
    )
}

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

fn assert_saved(pane: &BriefPane, subject: &str) {
    let Some(BriefState::Written(brief)) = pane.state_of(&key()) else {
        panic!("saved brief missing")
    };
    assert_eq!(brief.record.subject, subject);
}

#[test]
fn keeps_saved_brief_and_page_during_repair_and_after_failure() {
    let mut pane = pane();
    pane.set_state(&key(), BriefState::Written(Box::new(brief("saved"))));
    pane.set_page(&key(), 1);
    pane.start_run(&key(), run("replacement"));
    pane.update_run_progress(
        &key(),
        "replacement",
        AgentProgress::ReviewingBrief {
            pages: 1,
            issues: 2,
        },
    );
    assert_saved(&pane, "saved");
    assert_eq!(pane.page_of(&key()), 1);
    assert!(pane.finish_run(
        &key(),
        "replacement",
        Err(ReviewJobError::Failed("Could not save".into()))
    ));
    assert_saved(&pane, "saved");
    assert_eq!(pane.page_of(&key()), 1);
    assert_eq!(pane.error_of(&key()), Some("Could not save"));
    assert!(pane.run_of(&key()).is_none());
}

#[test]
fn cancellation_and_revision_changes_reject_late_results() {
    let mut pane = pane();
    pane.set_state(&key(), BriefState::Written(Box::new(brief("saved"))));
    let cancelled = run("cancelled");
    let gate = cancelled.gate.clone();
    pane.start_run(&key(), cancelled);
    pane.cancel_run(&key());
    assert!(gate.cancellation_flag().load(Ordering::SeqCst));
    pane.start_run(&key(), run("current"));
    assert!(!pane.finish_run(&key(), "cancelled", Ok(brief("late"))));
    pane.invalidate_runs(&ReviewScope::PullRequest(99), Some("new"), Some("base"));
    assert!(pane.run_of(&key()).is_some());
    pane.invalidate_runs(&ReviewScope::PullRequest(42), Some("new"), Some("base"));
    assert!(pane.run_of(&key()).is_none());
    assert!(!pane.finish_run(&key(), "current", Ok(brief("stale"))));
    assert_saved(&pane, "saved");
}

#[test]
fn publication_that_already_started_completes_before_another_run() {
    let mut pane = pane();
    pane.set_state(&key(), BriefState::Written(Box::new(brief("saved"))));
    let publishing = run("publishing");
    assert!(publishing.gate.begin_publication());
    pane.start_run(&key(), publishing);
    pane.cancel_run(&key());
    assert!(pane.run_of(&key()).is_some());
    assert!(pane.finish_run(&key(), "publishing", Ok(brief("new"))));
    assert_saved(&pane, "new");
    assert!(pane.run_of(&key()).is_none());
}

/// An already completed model result, retained independently of any attempt's task/gate.
fn pending_brief(run_id: &str) -> Box<PendingBrief> {
    let publication = qd_cache::AgentPublication {
        run_id: run_id.into(),
        snapshot_id: "head:base".into(),
        validation_receipt: "sealed-receipt".into(),
        cost_usd: None,
        session_id: None,
        model: None,
        duration_ms: 0,
        audit: vec![],
    };
    let mut record = brief("pending").record;
    record.mcp = Some(qd_studio::BriefMcpRecord {
        base_sha: Some("base".into()),
        run_id: publication.run_id.clone(),
        snapshot_id: publication.snapshot_id.clone(),
        validation_receipt: publication.validation_receipt.clone(),
        audit: vec![],
    });
    Box::new(PendingBrief::new(
        ReviewScope::PullRequest(42),
        Studio {
            lead: "Completed output".into(),
            pages: vec![],
        },
        record,
        publication,
    ))
}

/// Repeated save failures retain the completed draft and saved page until a local retry succeeds.
#[test]
fn retains_completed_brief_across_save_retries() {
    // A completed generation fails to save without hiding the prior artifact.
    let mut pane = pane();
    pane.set_state(&key(), BriefState::Written(Box::new(brief("saved"))));
    pane.set_page(&key(), 1);
    pane.start_run(&key(), run("completed"));
    assert!(pane.finish_run(
        &key(),
        "completed",
        Err(ReviewJobError::SaveFailed {
            draft: pending_brief("completed"),
            message: "Disk unavailable".into(),
        })
    ));
    assert_saved(&pane, "saved");
    assert_eq!(pane.page_of(&key()), 1);
    assert_eq!(pane.save_status_of(&key()), BriefSaveStatus::Pending);

    // A second failed transaction returns the same draft, without another generation.
    let draft = pane.take_pending_save(&key()).unwrap();
    assert_eq!(draft.run_id(), "completed");
    pane.start_save(&key(), run("completed"));
    assert_eq!(pane.save_status_of(&key()), BriefSaveStatus::Saving);
    assert!(pane.finish_run(
        &key(),
        "completed",
        Err(ReviewJobError::SaveFailed {
            draft,
            message: "Still unavailable".into(),
        })
    ));
    assert_eq!(pane.pending_save_of(&key()).unwrap().run_id(), "completed");
    assert_eq!(pane.page_of(&key()), 1);

    // Successful publication replaces content and resets the page exactly once.
    let _draft = pane.take_pending_save(&key()).unwrap();
    pane.start_save(&key(), run("completed"));
    assert!(pane.finish_run(&key(), "completed", Ok(brief("new"))));
    assert_saved(&pane, "new");
    assert_eq!(pane.page_of(&key()), 0);
    assert_eq!(pane.save_status_of(&key()), BriefSaveStatus::None);
}

/// Pending saves are scoped, discarded on either commit change, and superseded by explicit writes.
#[test]
fn pending_brief_cannot_survive_cancel_revision_change_or_new_generation() {
    for (head, base) in [("new-head", "base"), ("head", "new-base")] {
        let mut pane = pane();
        pane.set_state(&key(), BriefState::Written(Box::new(brief("saved"))));
        pane.start_run(&key(), run("completed"));
        pane.finish_run(
            &key(),
            "completed",
            Err(ReviewJobError::SaveFailed {
                draft: pending_brief("completed"),
                message: "Save failed".into(),
            }),
        );
        // Another scope cannot invalidate this completed draft.
        pane.invalidate_runs(&ReviewScope::PullRequest(99), Some(head), Some(base));
        assert_eq!(pane.save_status_of(&key()), BriefSaveStatus::Pending);
        pane.invalidate_runs(&ReviewScope::PullRequest(42), Some(head), Some(base));
        assert_eq!(pane.save_status_of(&key()), BriefSaveStatus::None);
        assert_saved(&pane, "saved");
    }

    // Discard works for a first brief as well as a replacement.
    let mut pane = pane();
    pane.start_run(&key(), run("completed"));
    pane.finish_run(
        &key(),
        "completed",
        Err(ReviewJobError::SaveFailed {
            draft: pending_brief("completed"),
            message: "Save failed".into(),
        }),
    );
    pane.cancel_run(&key());
    assert!(pane.state_of(&key()).is_none());
    assert!(pane.pending_save_of(&key()).is_none());

    // A new generation cannot receive a late completion from the earlier draft.
    pane.start_run(&key(), run("old"));
    pane.finish_run(
        &key(),
        "old",
        Err(ReviewJobError::SaveFailed {
            draft: pending_brief("old"),
            message: "Save failed".into(),
        }),
    );
    pane.start_run(&key(), run("new"));
    assert!(pane.pending_save_of(&key()).is_none());
    assert!(!pane.finish_run(
        &key(),
        "old",
        Err(ReviewJobError::SaveFailed {
            draft: pending_brief("old"),
            message: "Late failure".into(),
        })
    ));
    assert!(pane.pending_save_of(&key()).is_none());
}
