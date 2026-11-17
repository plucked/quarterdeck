use std::sync::atomic::Ordering;

use qd_cache::{AgentPublication, ReviewScope};
use qd_studio::{BriefRecord, Studio};

use super::review_job_error::ReviewJobError;
use super::review_publication_gate::ReviewPublicationGate;
use crate::brief_view::PreparedBrief;
use crate::ui_state::SharedUiStateStore;

/// A sealed brief and its exact run record, kept intact until storage succeeds.
#[derive(Debug)]
pub(super) struct PendingBrief {
    scope: ReviewScope,
    studio: Studio,
    record: BriefRecord,
    publication: AgentPublication,
}

impl PendingBrief {
    /// Retains the original studio because its prepared view cannot be saved again.
    pub fn new(
        scope: ReviewScope,
        studio: Studio,
        record: BriefRecord,
        publication: AgentPublication,
    ) -> Self {
        Self {
            scope,
            studio,
            record,
            publication,
        }
    }

    pub fn run_id(&self) -> &str {
        &self.publication.run_id
    }

    pub fn scope(&self) -> &ReviewScope {
        &self.scope
    }

    pub fn head_sha(&self) -> Option<&str> {
        self.record.head_sha.as_deref()
    }

    pub fn base_sha(&self) -> Option<&str> {
        self.record
            .mcp
            .as_ref()
            .and_then(|record| record.base_sha.as_deref())
    }

    /// Saves the exact sealed result without starting another agent or reservation.
    /// Storage failure returns ownership so the UI can offer an explicit retry.
    pub fn save(
        self: Box<Self>,
        store: &SharedUiStateStore,
        gate: &ReviewPublicationGate,
    ) -> Result<PreparedBrief, ReviewJobError<Self>> {
        // An accepted cancellation also prevents an exact saved retry from resurfacing.
        if gate.cancellation_flag().load(Ordering::SeqCst) {
            return Err(ReviewJobError::Failed("Cancelled.".into()));
        }

        // The database arbitrates cancellation only after obtaining publication authority.
        let outcome = store.lock().put_brief_for_run(
            &self.scope,
            &self.studio,
            &self.record,
            &self.publication,
            || gate.begin_publication(),
        );
        match outcome {
            Ok(true) => Ok(PreparedBrief::prepare(self.studio, self.record)),
            Ok(false) => Err(ReviewJobError::Failed(Self::rejection_message(gate))),
            Err(error) => {
                // Keep an invalidated snapshot terminal even if refresh raced the failed save.
                gate.failed_publication();
                if gate.cancellation_flag().load(Ordering::SeqCst) {
                    return Err(ReviewJobError::Failed(Self::rejection_message(gate)));
                }
                Err(ReviewJobError::SaveFailed {
                    draft: self,
                    message: format!(
                        "Could not save the brief. Your previous brief is unchanged: {error}"
                    ),
                })
            }
        }
    }

    /// Distinguishes accepted cancellation from another generation taking ownership.
    fn rejection_message(gate: &ReviewPublicationGate) -> String {
        if gate.cancellation_flag().load(Ordering::SeqCst) {
            "Cancelled.".into()
        } else {
            "A newer brief replaced this run. Your saved brief is unchanged.".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use qd_cache::{AgentPublication, ReviewScope, UiStateStore};
    use qd_studio::{
        BriefMcpRecord, BriefRecord, CodeAccess, GuidedToolAudit, ReaderFamiliarity, Studio,
        StudioPage, StudioWidget,
    };

    use super::PendingBrief;
    use crate::pull_request_detail::review_job_error::ReviewJobError;
    use crate::pull_request_detail::review_publication_gate::ReviewPublicationGate;
    use crate::ui_state::SharedUiStateStore;

    /// A complete tiny artifact includes session-only material that a retry must retain.
    fn pending(run_id: &str) -> Box<PendingBrief> {
        let publication = AgentPublication {
            run_id: run_id.into(),
            snapshot_id: "head:base".into(),
            validation_receipt: format!("receipt-{run_id}"),
            cost_usd: Some(0.5),
            session_id: Some("synthetic-session".into()),
            model: Some("synthetic-model".into()),
            duration_ms: 1000,
            audit: vec![GuidedToolAudit {
                tool_name: "brief_finish".into(),
                status: "ready".into(),
                revision: 3,
                issue_codes: vec![],
            }],
        };
        let record = BriefRecord {
            mcp: Some(BriefMcpRecord {
                base_sha: Some("base".into()),
                run_id: publication.run_id.clone(),
                snapshot_id: publication.snapshot_id.clone(),
                validation_receipt: publication.validation_receipt.clone(),
                audit: publication.audit.clone(),
            }),
            repository: "synthetic/project".into(),
            number: 42,
            url: "https://example.invalid/42".into(),
            subject: run_id.into(),
            layers: vec![],
            head_sha: Some("head".into()),
            reader: ReaderFamiliarity::default(),
            code: CodeAccess::DiffOnly("test".into()),
            prompt: "Review the synthetic change.".into(),
            answer: Some(serde_json::json!({"lead": run_id})),
            transcript: Some(vec!["synthetic event".into()]),
            tool_calls: vec![],
            rejected: vec![],
            cost_usd: publication.cost_usd,
            session_id: publication.session_id.clone(),
            model: publication.model.clone(),
            duration: Duration::from_millis(publication.duration_ms),
            written_at: 1,
        };
        Box::new(PendingBrief::new(
            ReviewScope::PullRequest(42),
            Studio {
                lead: run_id.into(),
                pages: vec![StudioPage {
                    title: "Behavior".into(),
                    summary: "Review the change.".into(),
                    widgets: vec![StudioWidget::Section {
                        heading: "Contract".into(),
                        body: "Keep ownership explicit.".into(),
                    }],
                }],
            },
            record,
            publication,
        ))
    }

    /// Synthetic generation ownership uses the same public reservation API as the job.
    fn reserve(store: &SharedUiStateStore, draft: &PendingBrief) {
        store
            .lock()
            .begin_brief_run(
                draft.scope(),
                draft.run_id(),
                &draft.publication.snapshot_id,
                draft.head_sha(),
                draft.base_sha(),
            )
            .unwrap();
    }

    /// Repeated SQL failures preserve the predecessor and the exact completed brief for retry.
    #[test]
    fn storage_failure_retains_completed_brief_for_save_only_retry() {
        // Publish a predecessor, then fail after the replacement's pages have been written.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let store = SharedUiStateStore::new(UiStateStore::open(&path).unwrap());
        let old = pending("old");
        reserve(&store, &old);
        old.save(&store, &ReviewPublicationGate::default()).unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TRIGGER reject_brief_receipt BEFORE INSERT ON brief_publications BEGIN SELECT RAISE(FAIL, 'synthetic save failure'); END;").unwrap();
        let mut draft = pending("new");
        reserve(&store, &draft);
        let gate = ReviewPublicationGate::default();

        // Failed attempts keep all completed content while SQLite retains the old receipt.
        for _ in 0..2 {
            draft = match draft.save(&store, &gate) {
                Err(ReviewJobError::SaveFailed { draft, .. }) => draft,
                _ => panic!("failed storage must retain the completed brief"),
            };
            assert_eq!(draft.studio, pending("new").studio);
            assert_eq!(draft.record.answer, pending("new").record.answer);
            assert_eq!(draft.record.transcript, pending("new").record.transcript);
            assert_eq!(draft.publication.validation_receipt, "receipt-new");
            assert_eq!(draft.publication.duration_ms, 1000);
            let stored = store.lock().brief(draft.scope()).unwrap().unwrap();
            assert_eq!(stored.studio, pending("old").studio);
            assert_eq!(stored.record.mcp.unwrap().validation_receipt, "receipt-old");
        }

        // The retained artifact saves without a new reservation and keeps its original record.
        connection
            .execute_batch("DROP TRIGGER reject_brief_receipt")
            .unwrap();
        let brief = draft.save(&store, &gate).unwrap();
        assert_eq!(brief.record.subject, "new");
        assert_eq!(brief.record.answer, pending("new").record.answer);
        assert_eq!(brief.record.transcript, pending("new").record.transcript);
        assert_eq!(brief.record.written_at, 1);
        let publication = store
            .lock()
            .brief_publication(&ReviewScope::PullRequest(42))
            .unwrap()
            .unwrap();
        assert_eq!(publication.run_id, "new");
        assert_eq!(publication.validation_receipt, "receipt-new");
        assert_eq!(publication.cost_usd, Some(0.5));
        assert_eq!(publication.duration_ms, 1000);
        assert_eq!(publication.session_id.as_deref(), Some("synthetic-session"));
        assert_eq!(publication.model.as_deref(), Some("synthetic-model"));
        assert_eq!(publication.audit[0].tool_name, "brief_finish");
        assert_eq!(publication.audit[0].revision, 3);
        // Exact retry verifies both persisted artifact and metadata against the same result.
        pending("new")
            .save(&store, &ReviewPublicationGate::default())
            .unwrap();
    }

    /// Cancellation and a newer reservation discard completed briefs without retry state.
    #[test]
    fn cancellation_and_new_generation_are_terminal() {
        let directory = tempfile::tempdir().unwrap();
        let store = SharedUiStateStore::new(
            UiStateStore::open(&directory.path().join("state.sqlite")).unwrap(),
        );
        let cancelled = pending("cancelled");
        reserve(&store, &cancelled);
        let gate = ReviewPublicationGate::default();
        assert!(gate.cancel());
        assert!(matches!(
            cancelled.save(&store, &gate),
            Err(ReviewJobError::Failed(_))
        ));

        let superseded = pending("older");
        reserve(&store, &superseded);
        reserve(&store, &pending("newer"));
        assert!(matches!(
            superseded.save(&store, &ReviewPublicationGate::default()),
            Err(ReviewJobError::Failed(_))
        ));
        assert!(
            store
                .lock()
                .brief(&ReviewScope::PullRequest(42))
                .unwrap()
                .is_none()
        );
    }
}
