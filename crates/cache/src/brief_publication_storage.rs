use anyhow::{Result, ensure};
use qd_studio::{BriefRecord, Studio};
use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::agent_publication_storage::{read_agent_publication, write_agent_publication};
use crate::brief_storage::write_brief;
use crate::publication_owner::PublicationOwner;
use crate::scope_columns::ScopeColumns;
use crate::{AgentPublication, ReviewScope, StoredBrief, UiStateStore};

const SCOPE_MATCH: &str = "pr_id IS ?1 AND repo_owner IS ?2 AND repo_name IS ?3
    AND first_number IS ?4 AND last_number IS ?5";

impl UiStateStore {
    /// Reserves one brief generation with its captured head/base snapshot identity.
    /// Existing saved briefs and their receipts remain untouched.
    pub fn begin_brief_run(
        &self,
        scope: &ReviewScope,
        run_id: &str,
        snapshot_id: &str,
        head_sha: Option<&str>,
        base_sha: Option<&str>,
    ) -> Result<()> {
        // Serialize competing generations before replacing their ownership row.
        let transaction =
            Transaction::new_unchecked(self.connection(), TransactionBehavior::Immediate)?;
        let columns = ScopeColumns::of(scope)?;
        transaction.execute(
            &format!("DELETE FROM brief_runs WHERE {SCOPE_MATCH}"),
            columns.as_params(),
        )?;
        transaction.execute(
            "INSERT INTO brief_runs (pr_id,repo_owner,repo_name,first_number,last_number,
                run_id,snapshot_id,head_sha,base_sha) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            columns.as_params_followed_by(&[&run_id, &snapshot_id, &head_sha, &base_sha]),
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Atomically publishes a completed brief, run record, receipt and typed audit.
    ///
    /// The captured snapshot and record must match both the head and base revisions.
    /// The callback claims publication only after the writer lock and reservation
    /// checks; it must not access this store. Exact saved retries return true without
    /// invoking it. False means supersession or accepted cancellation.
    pub fn put_brief_for_run(
        &self,
        scope: &ReviewScope,
        studio: &Studio,
        record: &BriefRecord,
        publication: &AgentPublication,
        can_publish: impl FnOnce() -> bool,
    ) -> Result<bool> {
        // Both durable representations must describe the same completed agent run.
        publication.validate()?;
        ensure!(
            publication.cost_usd.map(f64::to_bits) == record.cost_usd.map(f64::to_bits)
                && publication.session_id == record.session_id
                && publication.model == record.model
                && u128::from(publication.duration_ms) == record.duration.as_millis(),
            "brief record and publication metadata disagree"
        );
        let transaction =
            Transaction::new_unchecked(self.connection(), TransactionBehavior::Immediate)?;
        if let Some(stored) = read_publication(&transaction, scope)?
            && stored.run_id == publication.run_id
        {
            ensure!(
                stored.matches_publication(publication)
                    && self
                        .brief(scope)?
                        .as_ref()
                        .is_some_and(|stored| matches_stored_brief(stored, studio, record)),
                "publication retry conflicts with the saved brief or metadata"
            );
            return Ok(true);
        }

        // A queued cancellation or newer run wins before any artifact replacement.
        let columns = ScopeColumns::of(scope)?;
        let base_sha = record.mcp.as_ref().and_then(|mcp| mcp.base_sha.as_deref());
        let owns_publication = transaction
            .query_row(
                &format!(
                    "SELECT 1 FROM brief_runs WHERE {SCOPE_MATCH}
                AND run_id=?6 AND snapshot_id=?7 AND head_sha IS ?8 AND base_sha IS ?9"
                ),
                columns.as_params_followed_by(&[
                    &publication.run_id,
                    &publication.snapshot_id,
                    &record.head_sha,
                    &base_sha,
                ]),
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !owns_publication || !can_publish() {
            return Ok(false);
        }

        // Every page, widget, record list and publication row share one commit.
        let brief_id = write_brief(&transaction, scope, studio, record)?;
        write_agent_publication(&transaction, PublicationOwner::Brief, brief_id, publication)?;
        transaction.execute(
            "UPDATE brief_publications SET base_sha=?2 WHERE brief_id=?1",
            rusqlite::params![brief_id, base_sha],
        )?;
        transaction.execute(
            &format!("DELETE FROM brief_runs WHERE {SCOPE_MATCH}"),
            columns.as_params(),
        )?;
        transaction.commit()?;
        Ok(true)
    }

    /// Reads the current brief's durable receipt and bounded typed diagnostics.
    pub fn brief_publication(&self, scope: &ReviewScope) -> Result<Option<AgentPublication>> {
        let transaction = self.connection().unchecked_transaction()?;
        let publication = read_publication(&transaction, scope)?;
        transaction.commit()?;
        Ok(publication)
    }
}

/// Resolves one saved brief before reading its metadata in the same transaction.
fn read_publication(
    connection: &Connection,
    scope: &ReviewScope,
) -> Result<Option<AgentPublication>> {
    let columns = ScopeColumns::of(scope)?;
    let brief_id: Option<i64> = connection
        .query_row(
            &format!("SELECT id FROM briefs WHERE {SCOPE_MATCH}"),
            columns.as_params(),
            |row| row.get(0),
        )
        .optional()?;
    match brief_id {
        Some(brief_id) => read_agent_publication(connection, PublicationOwner::Brief, brief_id),
        None => Ok(None),
    }
}

/// Compares persisted artifact fields, excluding session-only raw output and transcript.
fn matches_stored_brief(stored: &StoredBrief, studio: &Studio, record: &BriefRecord) -> bool {
    let saved = &stored.record;
    stored.studio == *studio
        && saved.repository == record.repository
        && saved.number == record.number
        && saved.url == record.url
        && saved.subject == record.subject
        && saved.layers.len() == record.layers.len()
        && saved
            .layers
            .iter()
            .zip(&record.layers)
            .all(|(left, right)| left.number == right.number && left.title == right.title)
        && saved.head_sha == record.head_sha
        && saved.mcp.as_ref().and_then(|mcp| mcp.base_sha.as_deref())
            == record.mcp.as_ref().and_then(|mcp| mcp.base_sha.as_deref())
        && saved.reader == record.reader
        && saved.code == record.code
        && saved.prompt == record.prompt
        && saved.tool_calls == record.tool_calls
        && saved.rejected == record.rejected
        && saved.written_at == record.written_at
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

    use qd_studio::{
        BriefMcpRecord, BriefRecord, CodeAccess, GuidedToolAudit, ReaderFamiliarity, Studio,
        StudioPage, StudioWidget,
    };
    use rusqlite::{Transaction, TransactionBehavior};

    use crate::{AgentPublication, ReviewScope, UiStateStore};

    /// A complete tiny brief independent of any external repository or model.
    fn studio(lead: &str) -> Studio {
        Studio {
            lead: lead.into(),
            pages: vec![StudioPage {
                title: "Behavior".into(),
                summary: "Check the change.".into(),
                widgets: vec![StudioWidget::Section {
                    heading: "Contract".into(),
                    body: "Keep ownership explicit.".into(),
                }],
            }],
        }
    }

    /// Metadata mirrors the real record while leaving raw messages session-only.
    fn record(run_id: &str) -> BriefRecord {
        let publication = publication(run_id);
        BriefRecord {
            repository: "synthetic/project".into(),
            number: 42,
            url: "https://example.invalid/42".into(),
            subject: "Synthetic change".into(),
            layers: vec![],
            head_sha: Some("head".into()),
            reader: ReaderFamiliarity::default(),
            code: CodeAccess::DiffOnly("test".into()),
            prompt: "Review the synthetic change.".into(),
            answer: None,
            transcript: Some(vec!["synthetic event".into()]),
            mcp: Some(BriefMcpRecord {
                base_sha: Some("base".into()),
                run_id: publication.run_id,
                snapshot_id: publication.snapshot_id,
                validation_receipt: publication.validation_receipt,
                audit: publication.audit,
            }),
            tool_calls: vec![],
            rejected: vec![],
            cost_usd: Some(0.5),
            session_id: Some("synthetic-session".into()),
            model: Some("synthetic-model".into()),
            duration: Duration::from_secs(1),
            written_at: 1,
        }
    }

    /// A receipt and a repair outcome that survive with the saved brief.
    fn publication(run_id: &str) -> AgentPublication {
        AgentPublication {
            run_id: run_id.into(),
            snapshot_id: "head:base".into(),
            validation_receipt: format!("receipt-{run_id}"),
            cost_usd: Some(0.5),
            session_id: Some("synthetic-session".into()),
            model: Some("synthetic-model".into()),
            duration_ms: 1000,
            audit: vec![GuidedToolAudit {
                tool_name: "brief_replace_page".into(),
                status: "rejected".into(),
                revision: 1,
                issue_codes: vec!["invalid_anchor".into()],
            }],
        }
    }

    /// Replacing a brief keeps its receipt atomic, recoverable and independent of raw output.
    #[test]
    fn receipt_and_brief_commit_and_rollback_together() {
        // Keep a pre-MCP brief intact while preparing its first publication.
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let store = UiStateStore::open(&path).unwrap();
        let scope = ReviewScope::PullRequest(42);
        store
            .put_brief(&scope, &studio("legacy"), &record("legacy"))
            .unwrap();
        assert!(store.brief(&scope).unwrap().unwrap().record.mcp.is_none());
        store
            .begin_brief_run(&scope, "first", "head:base", Some("head"), Some("base"))
            .unwrap();
        assert!(
            store
                .put_brief_for_run(
                    &scope,
                    &studio("first"),
                    &record("first"),
                    &publication("first"),
                    || true
                )
                .unwrap()
        );

        // Fail after replacing pages and writing metadata; the entire replacement rolls back.
        store
            .begin_brief_run(&scope, "second", "head:base", Some("head"), Some("base"))
            .unwrap();
        store.connection().execute_batch("CREATE TRIGGER reject_brief_audit BEFORE INSERT ON brief_tool_issues BEGIN SELECT RAISE(FAIL, 'synthetic issue failure'); END;").unwrap();
        assert!(
            store
                .put_brief_for_run(
                    &scope,
                    &studio("second"),
                    &record("second"),
                    &publication("second"),
                    || true
                )
                .is_err()
        );
        let saved = store.brief(&scope).unwrap().unwrap();
        assert_eq!(saved.studio.lead, "first");
        assert_eq!(
            saved.record.mcp.unwrap().validation_receipt,
            "receipt-first"
        );
        assert!(saved.record.answer.is_none());
        assert!(saved.record.transcript.is_none());
        store
            .connection()
            .execute_batch("DROP TRIGGER reject_brief_audit")
            .unwrap();
        assert!(
            store
                .put_brief_for_run(
                    &scope,
                    &studio("second"),
                    &record("second"),
                    &publication("second"),
                    || true
                )
                .unwrap()
        );
        drop(store);
        assert_reopened_publication(&path, &scope);
    }

    /// Reopening and exact retries recover the receipt without a second gate claim.
    fn assert_reopened_publication(path: &std::path::Path, scope: &ReviewScope) {
        let reopened = UiStateStore::open(path).unwrap();
        assert_eq!(
            reopened
                .brief(scope)
                .unwrap()
                .unwrap()
                .record
                .mcp
                .unwrap()
                .base_sha
                .as_deref(),
            Some("base")
        );
        assert!(
            reopened
                .brief_publication(scope)
                .unwrap()
                .unwrap()
                .matches_publication(&publication("second"))
        );
        assert!(
            reopened
                .put_brief_for_run(
                    scope,
                    &studio("second"),
                    &record("second"),
                    &publication("second"),
                    || panic!("retry gate called")
                )
                .unwrap()
        );
        assert!(
            reopened
                .put_brief_for_run(
                    scope,
                    &studio("different"),
                    &record("second"),
                    &publication("second"),
                    || true
                )
                .is_err()
        );
        let mut changed_base = record("second");
        changed_base.mcp.as_mut().unwrap().base_sha = Some("different-base".into());
        assert!(
            reopened
                .put_brief_for_run(
                    scope,
                    &studio("second"),
                    &changed_base,
                    &publication("second"),
                    || panic!("conflicting retry gate called")
                )
                .is_err()
        );
    }

    /// Superseded generations and changed snapshots never reach the publication gate.
    #[test]
    fn generations_and_snapshots_are_checked_before_the_gate() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let first = UiStateStore::open(&path).unwrap();
        let second = UiStateStore::open(&path).unwrap();
        let scope = ReviewScope::PullRequest(42);
        first
            .put_brief(&scope, &studio("legacy"), &record("legacy"))
            .unwrap();
        first
            .begin_brief_run(&scope, "first", "head:base", Some("head"), Some("base"))
            .unwrap();
        second
            .begin_brief_run(&scope, "second", "head:base", Some("head"), Some("base"))
            .unwrap();
        assert!(
            !first
                .put_brief_for_run(
                    &scope,
                    &studio("first"),
                    &record("first"),
                    &publication("first"),
                    || panic!("stale gate called")
                )
                .unwrap()
        );
        let mut changed = publication("second");
        changed.snapshot_id = "head:different-base".into();
        assert!(
            !second
                .put_brief_for_run(
                    &scope,
                    &studio("second"),
                    &record("second"),
                    &changed,
                    || panic!("snapshot gate called")
                )
                .unwrap()
        );
        let mut changed_base = record("second");
        changed_base.mcp.as_mut().unwrap().base_sha = Some("different-base".into());
        assert!(
            !second
                .put_brief_for_run(
                    &scope,
                    &studio("second"),
                    &changed_base,
                    &publication("second"),
                    || panic!("base-only mismatch gate called")
                )
                .unwrap()
        );
        assert!(
            !second
                .put_brief_for_run(
                    &scope,
                    &studio("second"),
                    &record("second"),
                    &publication("second"),
                    || false
                )
                .unwrap()
        );
        assert_eq!(first.brief(&scope).unwrap().unwrap().studio.lead, "legacy");
    }

    /// Cancellation remains available while a brief waits for SQLite's writer lock.
    #[test]
    fn queued_brief_publication_can_still_be_cancelled() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.sqlite");
        let publisher = UiStateStore::open(&path).unwrap();
        let blocker = UiStateStore::open(&path).unwrap();
        publisher
            .connection()
            .busy_timeout(Duration::from_secs(2))
            .unwrap();
        let scope = ReviewScope::PullRequest(42);
        publisher
            .put_brief(&scope, &studio("legacy"), &record("legacy"))
            .unwrap();
        publisher
            .begin_brief_run(&scope, "new", "head:base", Some("head"), Some("base"))
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
            let result = publisher.put_brief_for_run(
                &publishing_scope,
                &studio("new"),
                &record("new"),
                &publication("new"),
                || !publishing_cancelled.load(Ordering::SeqCst),
            );
            result_sender.send(result).unwrap();
        });
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
            blocker.brief(&scope).unwrap().unwrap().studio.lead,
            "legacy"
        );
    }
}
