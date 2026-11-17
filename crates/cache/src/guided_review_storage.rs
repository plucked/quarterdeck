use anyhow::{Result, ensure};
use qd_studio::SmartFilter;
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::agent_publication_storage::{read_agent_publication, write_agent_publication};
use crate::publication_owner::PublicationOwner;
use crate::scope_columns::ScopeColumns;
use crate::smart_filter_storage::write_smart_filter;
use crate::{GuidedReviewPublication, ReviewScope, UiStateStore};

const SCOPE_MATCH: &str = "pr_id IS ?1 AND repo_owner IS ?2 AND repo_name IS ?3
    AND first_number IS ?4 AND last_number IS ?5";

impl UiStateStore {
    /// Reserves publication for a new run without changing the saved guide.
    /// A newer reservation supersedes an older process's in-flight result.
    pub fn begin_guided_review(
        &self,
        scope: &ReviewScope,
        run_id: &str,
        head_sha: &str,
        base_sha: Option<&str>,
    ) -> Result<()> {
        // Reserve ownership only after taking the SQLite writer lock.
        let transaction =
            Transaction::new_unchecked(self.connection(), TransactionBehavior::Immediate)?;
        let columns = ScopeColumns::of(scope)?;
        transaction.execute(
            &format!("DELETE FROM guided_review_runs WHERE {SCOPE_MATCH}"),
            columns.as_params(),
        )?;
        transaction.execute(
            "INSERT INTO guided_review_runs (pr_id, repo_owner, repo_name, first_number,
                last_number, run_id, head_sha, base_sha) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            columns.as_params_followed_by(&[&run_id, &head_sha, &base_sha]),
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Publishes the current run's exact revision and successful receipt atomically.
    ///
    /// The callback claims publication after taking SQLite's writer lock and
    /// checking ownership, so cancellation can win while either lock is queued.
    /// True means saved or an exact retry of the current durable publication;
    /// false means superseded or cancelled. The callback must not access this store.
    pub fn put_guided_review(
        &self,
        scope: &ReviewScope,
        filter: &SmartFilter,
        publication: &GuidedReviewPublication,
        can_publish: impl FnOnce() -> bool,
    ) -> Result<bool> {
        // Check metadata before claiming authority, then recover exact saved retries.
        publication.validate()?;
        let transaction =
            Transaction::new_unchecked(self.connection(), TransactionBehavior::Immediate)?;
        if let Some(stored) = read_publication(&transaction, scope)?
            && stored.run_id == publication.run_id
        {
            ensure!(
                stored.matches_publication(publication)
                    && self.smart_filter(scope)?.as_ref() == Some(filter),
                "publication retry conflicts with the saved guide or metadata"
            );
            return Ok(true);
        }

        // No host gate is claimed before the current generation and revision match.
        let columns = ScopeColumns::of(scope)?;
        let owns_publication = transaction
            .query_row(
                &format!(
                    "SELECT 1 FROM guided_review_runs WHERE {SCOPE_MATCH}
                AND run_id = ?6 AND head_sha = ?7 AND base_sha IS ?8"
                ),
                columns.as_params_followed_by(&[
                    &publication.run_id,
                    &filter.head_sha,
                    &filter.base_sha,
                ]),
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !owns_publication || !can_publish() {
            return Ok(false);
        }
        // The previous guide and ownership survive any failed child-row write.
        write_smart_filter(&transaction, scope, filter)?;
        let filter_id = transaction.query_row(
            &format!("SELECT id FROM smart_filters WHERE {SCOPE_MATCH}"),
            columns.as_params(),
            |row| row.get(0),
        )?;
        write_agent_publication(
            &transaction,
            PublicationOwner::GuidedReview,
            filter_id,
            publication,
        )?;
        transaction.execute(
            &format!("DELETE FROM guided_review_runs WHERE {SCOPE_MATCH}"),
            columns.as_params(),
        )?;
        transaction.commit()?;
        Ok(true)
    }

    /// Reads the current guide's durable receipt and typed diagnostic metadata.
    pub fn guided_review_publication(
        &self,
        scope: &ReviewScope,
    ) -> Result<Option<GuidedReviewPublication>> {
        // Read the metadata and child audit rows from one consistent snapshot.
        let transaction = self.connection().unchecked_transaction()?;
        let publication = read_publication(&transaction, scope)?;
        transaction.commit()?;
        Ok(publication)
    }
}

/// Resolves a scope's current guide and reads its receipt in the same transaction.
fn read_publication(
    connection: &Connection,
    scope: &ReviewScope,
) -> Result<Option<GuidedReviewPublication>> {
    let columns = ScopeColumns::of(scope)?;
    let filter_id: Option<i64> = connection
        .query_row(
            &format!("SELECT id FROM smart_filters WHERE {SCOPE_MATCH}"),
            columns.as_params(),
            |row| row.get(0),
        )
        .optional()?;
    match filter_id {
        Some(filter_id) => {
            read_agent_publication(connection, PublicationOwner::GuidedReview, filter_id)
        }
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        },
        time::Duration,
    };

    use qd_studio::{GuidedToolAudit, SmartBucket, SmartFilter, SmartReviewItem};
    use rusqlite::{Transaction, TransactionBehavior};

    use crate::{GuidedReviewPublication, ReviewScope, UiStateStore};

    /// A small saved artifact whose child rows can deliberately violate SQL constraints.
    fn guide(head: &str) -> SmartFilter {
        SmartFilter {
            head_sha: head.into(),
            base_sha: Some("base".into()),
            generated_at: 1,
            buckets: vec![SmartBucket {
                order: 1,
                label: "Source".into(),
                description: "Read the source".into(),
                suggest_viewed: false,
                items: vec![SmartReviewItem {
                    label: "Entry".into(),
                    description: "Check behavior".into(),
                    files: vec!["a.rs".into()],
                }],
            }],
        }
    }

    /// Synthetic metadata includes a repair outcome and the final readiness call.
    fn publication(run_id: &str) -> GuidedReviewPublication {
        GuidedReviewPublication {
            run_id: run_id.into(),
            snapshot_id: "head:base".into(),
            validation_receipt: format!("receipt-{run_id}"),
            cost_usd: Some(0.125),
            session_id: Some("synthetic-session".into()),
            model: Some("synthetic-model".into()),
            duration_ms: 2400,
            audit: vec![
                GuidedToolAudit {
                    tool_name: "guide_validate".into(),
                    status: "rejected".into(),
                    revision: 1,
                    issue_codes: vec!["unknown_path".into(), "missing_paths".into()],
                },
                GuidedToolAudit {
                    tool_name: "guide_finish".into(),
                    status: "ready".into(),
                    revision: 2,
                    issue_codes: vec![],
                },
            ],
        }
    }

    /// Another connection's newer reservation and exact snapshot guard publication.
    #[test]
    fn newer_generation_and_revision_guard_the_saved_guide() {
        // Set up two connections, as two app processes would have.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let older = UiStateStore::open(&path).unwrap();
        let newer = UiStateStore::open(&path).unwrap();
        let scope = ReviewScope::PullRequest(42);
        older.put_smart_filter(&scope, &guide("old")).unwrap();
        older
            .begin_guided_review(&scope, "first", "head", Some("base"))
            .unwrap();
        newer
            .begin_guided_review(&scope, "second", "head", Some("base"))
            .unwrap();

        // Old generations and mismatched snapshots never replace the old guide.
        assert!(
            !older
                .put_guided_review(&scope, &guide("head"), &publication("first"), || panic!(
                    "stale gate called"
                ))
                .unwrap()
        );
        assert!(
            !newer
                .put_guided_review(&scope, &guide("wrong"), &publication("second"), || panic!(
                    "wrong snapshot gate called"
                ))
                .unwrap()
        );
        assert_eq!(older.smart_filter(&scope).unwrap().unwrap().head_sha, "old");
        assert!(
            newer
                .put_guided_review(&scope, &guide("head"), &publication("second"), || true)
                .unwrap()
        );
        assert_eq!(
            older.smart_filter(&scope).unwrap().unwrap().head_sha,
            "head"
        );
        // An uncertain acknowledgement recovers the exact durable result without mutation.
        assert!(
            newer
                .put_guided_review(&scope, &guide("head"), &publication("second"), || panic!(
                    "retry gate called"
                ))
                .unwrap()
        );
        assert!(
            older
                .guided_review_publication(&scope)
                .unwrap()
                .unwrap()
                .matches_publication(&publication("second"))
        );
        let mut conflict = guide("head");
        conflict.buckets[0].label = "Different content".into();
        assert!(
            newer
                .put_guided_review(&scope, &conflict, &publication("second"), || true)
                .is_err()
        );
    }

    /// Failed artifact and metadata child writes retain the prior guide and receipt.
    #[test]
    fn failed_publication_keeps_previous_guide_and_can_retry() {
        let directory = tempfile::tempdir().unwrap();
        let store = UiStateStore::open(&directory.path().join("state.sqlite")).unwrap();
        let scope = ReviewScope::PullRequest(42);
        store
            .begin_guided_review(&scope, "old", "old", Some("base"))
            .unwrap();
        store
            .put_guided_review(&scope, &guide("old"), &publication("old"), || true)
            .unwrap();
        store
            .begin_guided_review(&scope, "new", "head", Some("base"))
            .unwrap();
        let mut invalid = guide("head");
        let duplicate = invalid.buckets[0].items[0].clone();
        invalid.buckets[0].items.push(duplicate);

        // A late child failure rolls back the guide replacement and its metadata.
        assert!(
            store
                .put_guided_review(&scope, &invalid, &publication("new"), || true)
                .is_err()
        );
        store.connection().execute_batch("CREATE TRIGGER reject_audit BEFORE INSERT ON guided_review_tool_issues BEGIN SELECT RAISE(FAIL, 'synthetic issue write failure'); END;").unwrap();
        assert!(
            store
                .put_guided_review(&scope, &guide("head"), &publication("new"), || true)
                .is_err()
        );
        assert_eq!(store.smart_filter(&scope).unwrap().unwrap().head_sha, "old");
        assert!(
            store
                .guided_review_publication(&scope)
                .unwrap()
                .unwrap()
                .matches_publication(&publication("old"))
        );

        // The untouched reservation permits retry once the storage failure is gone.
        store
            .connection()
            .execute_batch("DROP TRIGGER reject_audit")
            .unwrap();
        assert!(
            store
                .put_guided_review(&scope, &guide("head"), &publication("new"), || true)
                .unwrap()
        );
        assert!(
            store
                .guided_review_publication(&scope)
                .unwrap()
                .unwrap()
                .matches_publication(&publication("new"))
        );
        let count: i64 = store
            .connection()
            .query_row(
                "SELECT COUNT(*) FROM guided_review_publications",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    /// Cancellation may win while publication waits behind another SQLite writer.
    #[test]
    fn cancellation_wins_while_waiting_for_the_writer_lock() {
        // Hold an external writer lock before the publication thread can begin.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let publisher = UiStateStore::open(&path).unwrap();
        let blocker = UiStateStore::open(&path).unwrap();
        publisher
            .connection()
            .busy_timeout(Duration::from_secs(2))
            .unwrap();
        let scope = ReviewScope::PullRequest(42);
        publisher.put_smart_filter(&scope, &guide("old")).unwrap();
        publisher
            .begin_guided_review(&scope, "new", "head", Some("base"))
            .unwrap();
        let transaction =
            Transaction::new_unchecked(blocker.connection(), TransactionBehavior::Immediate)
                .unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let publishing_cancelled = cancelled.clone();
        let publishing_scope = scope.clone();
        let (started_sender, started_receiver) = mpsc::sync_channel(1);
        let (result_sender, result_receiver) = mpsc::sync_channel(1);
        let thread = std::thread::spawn(move || {
            started_sender.send(()).unwrap();
            let result = publisher.put_guided_review(
                &publishing_scope,
                &guide("head"),
                &publication("new"),
                || !publishing_cancelled.load(Ordering::SeqCst),
            );
            result_sender.send(result).unwrap();
        });

        // Cancellation precedes acquisition; the eventual callback must observe it.
        started_receiver
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        cancelled.store(true, Ordering::SeqCst);
        transaction.commit().unwrap();
        assert!(
            !result_receiver
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap()
        );
        thread.join().unwrap();
        assert_eq!(
            blocker.smart_filter(&scope).unwrap().unwrap().head_sha,
            "old"
        );
        assert!(blocker.guided_review_publication(&scope).unwrap().is_none());
    }

    /// Metadata survives reopening and oversized input fails before the host gate.
    #[test]
    fn durable_metadata_round_trip_and_bounds() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let scope = ReviewScope::PullRequest(42);
        let store = UiStateStore::open(&path).unwrap();
        store
            .begin_guided_review(&scope, "new", "head", Some("base"))
            .unwrap();
        let mut oversized = publication("new");
        oversized.audit = vec![oversized.audit[0].clone(); 513];
        assert!(
            store
                .put_guided_review(&scope, &guide("head"), &oversized, || panic!(
                    "invalid metadata gate called"
                ))
                .is_err()
        );
        assert!(
            store
                .put_guided_review(&scope, &guide("head"), &publication("new"), || true)
                .unwrap()
        );
        drop(store);

        let reopened = UiStateStore::open(&path).unwrap();
        assert!(
            reopened
                .guided_review_publication(&scope)
                .unwrap()
                .unwrap()
                .matches_publication(&publication("new"))
        );
        assert_eq!(reopened.smart_filter(&scope).unwrap(), Some(guide("head")));
    }
}
