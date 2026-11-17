use std::sync::atomic::Ordering;

use qd_cache::{GuidedReviewPublication, ReviewScope};
use qd_studio::SmartFilter;

use super::review_job_error::ReviewJobError;
use super::review_publication_gate::ReviewPublicationGate;
use crate::ui_state::SharedUiStateStore;

/// A sealed guide and its exact run metadata, retained until its save succeeds.
#[derive(Debug)]
pub(super) struct PendingGuidedReview {
    scope: ReviewScope,
    filter: SmartFilter,
    publication: GuidedReviewPublication,
}

impl PendingGuidedReview {
    /// Keeps the completed artifact unchanged across storage attempts.
    pub fn new(
        scope: ReviewScope,
        filter: SmartFilter,
        publication: GuidedReviewPublication,
    ) -> Self {
        Self {
            scope,
            filter,
            publication,
        }
    }

    pub fn run_id(&self) -> &str {
        &self.publication.run_id
    }

    pub fn scope(&self) -> &ReviewScope {
        &self.scope
    }

    pub fn head_sha(&self) -> &str {
        &self.filter.head_sha
    }

    pub fn base_sha(&self) -> Option<&str> {
        self.filter.base_sha.as_deref()
    }

    /// Saves the sealed result without starting another generation or reservation.
    /// Storage failure returns ownership so the UI can offer an explicit retry.
    pub fn save(
        self: Box<Self>,
        store: &SharedUiStateStore,
        gate: &ReviewPublicationGate,
    ) -> Result<SmartFilter, ReviewJobError<Self>> {
        // An accepted cancellation also prevents an exact saved retry from resurfacing.
        if gate.cancellation_flag().load(Ordering::SeqCst) {
            return Err(ReviewJobError::Failed("Cancelled.".into()));
        }

        // The database claims the gate only after its writer lock and ownership checks.
        let outcome =
            store
                .lock()
                .put_guided_review(&self.scope, &self.filter, &self.publication, || {
                    gate.begin_publication()
                });
        match outcome {
            Ok(true) => Ok(self.filter),
            Ok(false) => Err(ReviewJobError::Failed(Self::rejection_message(gate))),
            Err(error) => {
                // A rolled-back transaction releases publication unless refresh revoked it.
                gate.failed_publication();
                if gate.cancellation_flag().load(Ordering::SeqCst) {
                    return Err(ReviewJobError::Failed(Self::rejection_message(gate)));
                }
                Err(ReviewJobError::SaveFailed {
                    draft: self,
                    message: format!(
                        "Could not save the guided review. Your previous guide is unchanged: {error}"
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
            "A newer guided review replaced this run. Your saved guide is unchanged.".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use qd_cache::{GuidedReviewPublication, ReviewScope, UiStateStore};
    use qd_studio::{GuidedToolAudit, SmartBucket, SmartFilter};

    use super::PendingGuidedReview;
    use crate::pull_request_detail::review_job_error::ReviewJobError;
    use crate::pull_request_detail::review_publication_gate::ReviewPublicationGate;
    use crate::ui_state::SharedUiStateStore;

    /// A tiny guide with distinct content for each synthetic completed run.
    fn pending(run_id: &str) -> Box<PendingGuidedReview> {
        Box::new(PendingGuidedReview::new(
            ReviewScope::PullRequest(42),
            SmartFilter {
                head_sha: "head".into(),
                base_sha: Some("base".into()),
                generated_at: 1,
                buckets: vec![SmartBucket {
                    order: 1,
                    label: run_id.into(),
                    description: "Review the change.".into(),
                    suggest_viewed: false,
                    files: vec!["source.rs".into()],
                    items: vec![],
                }],
            },
            GuidedReviewPublication {
                run_id: run_id.into(),
                snapshot_id: "head:base".into(),
                validation_receipt: format!("receipt-{run_id}"),
                cost_usd: Some(0.125),
                session_id: Some("synthetic-session".into()),
                model: Some("synthetic-model".into()),
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

    /// Synthetic generation ownership uses the same public reservation API as the job.
    fn reserve(store: &SharedUiStateStore, draft: &PendingGuidedReview) {
        store
            .lock()
            .begin_guided_review(
                draft.scope(),
                draft.run_id(),
                draft.head_sha(),
                draft.base_sha(),
            )
            .unwrap();
    }

    /// Repeated late SQL failures keep the old guide and the exact completed retry payload.
    #[test]
    fn storage_failure_retains_completed_guide_for_save_only_retry() {
        // Publish a predecessor, then reject the new receipt after its artifact was written.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let store = SharedUiStateStore::new(UiStateStore::open(&path).unwrap());
        let old = pending("old");
        reserve(&store, &old);
        old.save(&store, &ReviewPublicationGate::default()).unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection.execute_batch("CREATE TRIGGER reject_guide_receipt BEFORE INSERT ON guided_review_publications BEGIN SELECT RAISE(FAIL, 'synthetic save failure'); END;").unwrap();
        let mut draft = pending("new");
        reserve(&store, &draft);
        let gate = ReviewPublicationGate::default();

        // Both failed attempts return the same artifact, receipt and original timing.
        for _ in 0..2 {
            draft = match draft.save(&store, &gate) {
                Err(ReviewJobError::SaveFailed { draft, .. }) => draft,
                _ => panic!("failed storage must retain the completed guide"),
            };
            assert_eq!(draft.filter, pending("new").filter);
            assert_eq!(draft.publication.validation_receipt, "receipt-new");
            assert_eq!(draft.publication.duration_ms, 2400);
            assert_eq!(draft.publication.cost_usd, Some(0.125));
            let stored = store.lock().smart_filter(draft.scope()).unwrap().unwrap();
            assert_eq!(stored, pending("old").filter);
            let publication = store
                .lock()
                .guided_review_publication(draft.scope())
                .unwrap()
                .unwrap();
            assert_eq!(publication.validation_receipt, "receipt-old");
        }

        // Removing the fault permits the retained result and its full metadata to commit.
        connection
            .execute_batch("DROP TRIGGER reject_guide_receipt")
            .unwrap();
        let filter = draft.save(&store, &gate).unwrap();
        assert_eq!(filter, pending("new").filter);
        let publication = store
            .lock()
            .guided_review_publication(&ReviewScope::PullRequest(42))
            .unwrap()
            .unwrap();
        assert_eq!(publication.run_id, "new");
        assert_eq!(publication.validation_receipt, "receipt-new");
        assert_eq!(publication.cost_usd, Some(0.125));
        assert_eq!(publication.duration_ms, 2400);
        assert_eq!(publication.session_id.as_deref(), Some("synthetic-session"));
        assert_eq!(publication.model.as_deref(), Some("synthetic-model"));
        assert_eq!(publication.audit[0].tool_name, "guide_finish");
        assert_eq!(publication.audit[0].revision, 2);
        // The cache's exact retry verifies the original artifact and every metadata field.
        pending("new")
            .save(&store, &ReviewPublicationGate::default())
            .unwrap();
    }

    /// Cancelled and superseded sealed guides are discarded rather than offered for retry.
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
                .smart_filter(&ReviewScope::PullRequest(42))
                .unwrap()
                .is_none()
        );
    }
}
