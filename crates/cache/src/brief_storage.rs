use std::time::Duration;

use anyhow::{Result, anyhow};
use qd_studio::{BriefRecord, CodeAccess, RejectedWidget, RequestLayer, Studio, ToolCall};
use rusqlite::{OptionalExtension, Transaction, params};

use crate::brief_sql_text::{CodeAccessKind, reader_from_sql_text, reader_sql_text};
use crate::brief_widget_writing::put_widget;
use crate::cached_statements::execute_cached;
use crate::scope_columns::ScopeColumns;
use crate::sql_integers::{read_u64, sql_integer};
use crate::{BriefIndex, ReviewScope, StoredBrief, UiStateStore};

/// The scope condition of a brief query, the scope columns as ?1 to ?5.
const SCOPE_MATCH: &str = "pr_id IS ?1 AND repo_owner IS ?2 AND repo_name IS ?3
    AND first_number IS ?4 AND last_number IS ?5";

impl UiStateStore {
    /// Stores the brief `studio` with its `record` as the brief of `scope`,
    /// replacing the one before, in one transaction: every page, widget and
    /// part of a widget in its own typed rows. Borrowed, so the caller keeps
    /// both to show (the record holds the whole prompt and transcript).
    pub fn put_brief(
        &self,
        scope: &ReviewScope,
        studio: &Studio,
        record: &BriefRecord,
    ) -> Result<()> {
        let transaction = self.connection().unchecked_transaction()?;

        // The brief before, with every part (the foreign keys cascade).
        let columns = ScopeColumns::of(scope)?;
        execute_cached(
            &transaction,
            &format!("DELETE FROM briefs WHERE {SCOPE_MATCH}"),
            columns.as_params(),
        )?;

        // The brief's own row, then its pages, then the record's lists.
        let brief_id = put_brief_row(&transaction, &columns, studio, record)?;
        put_pages(&transaction, brief_id, studio)?;
        put_record_lists(&transaction, brief_id, record)?;
        transaction.commit()?;
        Ok(())
    }

    /// The stored brief of `scope`, if there is one. A brief whose rows no
    /// longer read (a value outside its CHECK) is an error, which the app
    /// treats as no brief.
    pub fn brief(&self, scope: &ReviewScope) -> Result<Option<StoredBrief>> {
        // The brief's own row.
        let columns = ScopeColumns::of(scope)?;
        let row = self
            .connection()
            .query_row(
                &format!(
                    "SELECT id, repository, number, url, subject, head_sha, reader, code_access,
                        diff_only_reason, prompt, lead, cost_usd, session_id, model, duration_ms,
                        written_at
                     FROM briefs WHERE {SCOPE_MATCH}"
                ),
                columns.as_params(),
                |row| {
                    Ok(BriefRow {
                        id: row.get(0)?,
                        repository: row.get(1)?,
                        number: read_u64(row, 2)?,
                        url: row.get(3)?,
                        subject: row.get(4)?,
                        head_sha: row.get(5)?,
                        reader: row.get(6)?,
                        code_access: row.get(7)?,
                        diff_only_reason: row.get(8)?,
                        prompt: row.get(9)?,
                        lead: row.get(10)?,
                        cost_usd: row.get(11)?,
                        session_id: row.get(12)?,
                        model: row.get(13)?,
                        duration_ms: read_u64(row, 14)?,
                        written_at: read_u64(row, 15)?,
                    })
                },
            )
            .optional()?;
        let Some(row) = row else {
            return Ok(None);
        };

        // The pages, each with its widgets.
        let pages = self.brief_pages(row.id)?;
        let record = BriefRecord {
            repository: row.repository,
            number: row.number,
            url: row.url,
            subject: row.subject,
            layers: self.brief_layers(row.id)?,
            head_sha: row.head_sha,
            reader: reader_from_sql_text(&row.reader)
                .ok_or_else(|| anyhow!("unknown reader {}", row.reader))?,
            code: match (
                CodeAccessKind::from_sql_text(&row.code_access),
                row.diff_only_reason,
            ) {
                (Some(CodeAccessKind::Workspace), _) => CodeAccess::Workspace,
                (Some(CodeAccessKind::DiffOnly), Some(reason)) => CodeAccess::DiffOnly(reason),
                (Some(CodeAccessKind::DiffOnly), None) => {
                    return Err(anyhow!("a diff-only brief without its reason"));
                }
                (None, _) => return Err(anyhow!("unknown code access {}", row.code_access)),
            },
            prompt: row.prompt,
            answer: None,
            transcript: None,
            tool_calls: self.brief_tool_calls(row.id)?,
            rejected: self.brief_rejected(row.id)?,
            cost_usd: row.cost_usd,
            session_id: row.session_id,
            model: row.model,
            duration: Duration::from_millis(row.duration_ms),
            written_at: row.written_at,
        };
        Ok(Some(StoredBrief {
            studio: Studio {
                lead: row.lead,
                pages,
            },
            record,
        }))
    }

    /// Which pull requests have a stored brief, by repository and number.
    pub fn brief_index(&self) -> Result<BriefIndex> {
        let mut statement = self
            .connection()
            .prepare("SELECT repository, number FROM briefs")?;
        let rows =
            statement.query_map([], |row| Ok((row.get::<_, String>(0)?, read_u64(row, 1)?)))?;
        let mut index = BriefIndex::default();
        for row in rows {
            let (repository, number) = row?;
            index.insert(&repository, number);
        }
        Ok(index)
    }

    /// The layers a combined brief spans, bottom first, without their
    /// descriptions (the stored prompt holds them).
    fn brief_layers(&self, brief_id: i64) -> Result<Vec<RequestLayer>> {
        let mut statement = self.connection().prepare_cached(
            "SELECT number, title FROM brief_layers WHERE brief_id = ?1 ORDER BY position",
        )?;
        let rows = statement.query_map([brief_id], |row| {
            // Descriptions are not stored: the stored prompt holds them.
            Ok(RequestLayer {
                number: read_u64(row, 0)?,
                title: row.get(1)?,
                description: None,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The widgets the checks left out, in order.
    fn brief_rejected(&self, brief_id: i64) -> Result<Vec<RejectedWidget>> {
        let mut statement = self.connection().prepare_cached(
            "SELECT widget, reason FROM brief_rejected WHERE brief_id = ?1 ORDER BY position",
        )?;
        let rows = statement.query_map([brief_id], |row| {
            Ok(RejectedWidget {
                widget: row.get(0)?,
                reason: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The tools the run called, in order.
    fn brief_tool_calls(&self, brief_id: i64) -> Result<Vec<ToolCall>> {
        let mut statement = self.connection().prepare_cached(
            "SELECT tool, input FROM brief_tool_calls WHERE brief_id = ?1 ORDER BY position",
        )?;
        let rows = statement.query_map([brief_id], |row| {
            Ok(ToolCall {
                tool: row.get(0)?,
                input: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}

/// The `briefs` row as read.
struct BriefRow {
    id: i64,
    repository: String,
    number: u64,
    url: String,
    subject: String,
    head_sha: Option<String>,
    reader: String,
    code_access: String,
    diff_only_reason: Option<String>,
    prompt: String,
    lead: String,
    cost_usd: Option<f64>,
    session_id: Option<String>,
    model: Option<String>,
    duration_ms: u64,
    written_at: u64,
}

/// Writes the brief's own row, its scope and the record of the run, and
/// returns its id.
fn put_brief_row(
    transaction: &Transaction<'_>,
    columns: &ScopeColumns,
    studio: &Studio,
    record: &BriefRecord,
) -> Result<i64> {
    let code_access = CodeAccessKind::of(&record.code).as_sql_text();
    let diff_only_reason = match &record.code {
        CodeAccess::Workspace => None,
        CodeAccess::DiffOnly(reason) => Some(reason.as_str()),
    };
    execute_cached(
        transaction,
        "INSERT INTO briefs (pr_id, repo_owner, repo_name, first_number, last_number,
            repository, number, url, subject, head_sha, reader, code_access,
            diff_only_reason, prompt, lead, cost_usd, session_id, model, duration_ms, written_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
            ?18, ?19, ?20)",
        columns.as_params_followed_by(params![
            record.repository,
            sql_integer(record.number)?,
            record.url,
            record.subject,
            record.head_sha,
            reader_sql_text(record.reader),
            code_access,
            diff_only_reason,
            record.prompt,
            studio.lead,
            record.cost_usd,
            record.session_id,
            record.model,
            i64::try_from(record.duration.as_millis()).unwrap_or(i64::MAX),
            sql_integer(record.written_at)?,
        ]),
    )?;
    Ok(transaction.last_insert_rowid())
}

/// Writes the pages and their widgets, the widgets numbered across the brief.
fn put_pages(transaction: &Transaction<'_>, brief_id: i64, studio: &Studio) -> Result<()> {
    let mut widget_position = 0_i64;
    for (page_position, page) in (0_i64..).zip(&studio.pages) {
        execute_cached(
            transaction,
            "INSERT INTO brief_pages (brief_id, position, title, summary) VALUES (?1, ?2, ?3, ?4)",
            params![brief_id, page_position, page.title, page.summary],
        )?;
        for widget in &page.widgets {
            put_widget(
                transaction,
                brief_id,
                widget_position,
                page_position,
                widget,
            )?;
            widget_position += 1;
        }
    }
    Ok(())
}

/// Writes the record's lists: the layers, what the checks left out and what
/// the run read.
fn put_record_lists(
    transaction: &Transaction<'_>,
    brief_id: i64,
    record: &BriefRecord,
) -> Result<()> {
    for (position, layer) in (0_i64..).zip(&record.layers) {
        execute_cached(
            transaction,
            "INSERT INTO brief_layers (brief_id, position, number, title) VALUES (?1, ?2, ?3, ?4)",
            params![brief_id, position, sql_integer(layer.number)?, layer.title],
        )?;
    }
    for (position, rejected) in (0_i64..).zip(&record.rejected) {
        execute_cached(
            transaction,
            "INSERT INTO brief_rejected (brief_id, position, widget, reason) VALUES (?1, ?2, ?3, ?4)",
            params![brief_id, position, rejected.widget, rejected.reason],
        )?;
    }
    for (position, call) in (0_i64..).zip(&record.tool_calls) {
        execute_cached(
            transaction,
            "INSERT INTO brief_tool_calls (brief_id, position, tool, input) VALUES (?1, ?2, ?3, ?4)",
            params![brief_id, position, call.tool, call.input],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use qd_github::RepoRef;
    use qd_studio::{
        CalloutTone, ChangeScope, CodeAnchor, CodeSide, DiagramChange, EdgeStyle, GraphEdge,
        GraphNode, MessageStyle, NodeRole, ReaderFamiliarity, SequenceLane, SequenceMessage,
        StepWeight, StudioGraph, StudioPage, StudioSequence, StudioTable, StudioTour, StudioWidget,
        TableRole, TourStep,
    };

    use super::*;

    fn anchor(first_line: u32) -> CodeAnchor {
        CodeAnchor {
            path: "src/lib.rs".into(),
            side: CodeSide::Head,
            first_line,
            last_line: first_line + 2,
        }
    }

    /// The first page's widgets: every kind but the diagrams.
    fn text_widgets() -> Vec<StudioWidget> {
        vec![
            StudioWidget::Section {
                heading: "Why".into(),
                body: "Because.".into(),
            },
            StudioWidget::Callout {
                tone: CalloutTone::Constraint,
                text: "Limit.".into(),
            },
            StudioWidget::CodeExcerpt {
                caption: "Here".into(),
                anchor: anchor(10),
            },
            StudioWidget::CodeCompare {
                caption: "Fix".into(),
                before: CodeAnchor {
                    side: CodeSide::Base,
                    ..anchor(4)
                },
                after: anchor(5),
            },
            StudioWidget::Table(StudioTable {
                title: "Trace".into(),
                columns: vec!["Action".into(), "After".into()],
                rows: vec![vec!["a".into(), "b".into()], vec!["c".into(), "d".into()]],
                role: TableRole::Trace,
            }),
            StudioWidget::Scope(ChangeScope {
                changes: vec!["One".into()],
                unchanged: Vec::new(),
                mechanical: vec!["Rename".into()],
            }),
        ]
    }

    /// The second page's widgets: the reading order and the diagrams.
    fn diagram_widgets() -> Vec<StudioWidget> {
        vec![
            StudioWidget::Tour(StudioTour {
                title: "Reading order".into(),
                steps: vec![TourStep {
                    title: "Core".into(),
                    explanation: "Does it.".into(),
                    weight: StepWeight::Core,
                    anchor: Some(anchor(20)),
                }],
            }),
            StudioWidget::Graph(StudioGraph {
                title: "Owners".into(),
                groups: vec!["Main thread".into()],
                nodes: vec![
                    GraphNode {
                        label: "Queue".into(),
                        detail: String::new(),
                        role: NodeRole::Owner,
                        group: Some(0),
                        anchor: None,
                        change: DiagramChange::Same,
                    },
                    GraphNode {
                        label: "GitHub".into(),
                        detail: "API".into(),
                        role: NodeRole::External,
                        group: None,
                        anchor: Some(anchor(30)),
                        change: DiagramChange::Added,
                    },
                ],
                edges: vec![GraphEdge {
                    from: 0,
                    to: 1,
                    label: "request".into(),
                    style: EdgeStyle::Async,
                    change: DiagramChange::Added,
                }],
            }),
            StudioWidget::Sequence(StudioSequence {
                title: "A refresh".into(),
                lanes: vec![
                    SequenceLane {
                        label: "Queue".into(),
                        role: NodeRole::Actor,
                    },
                    SequenceLane {
                        label: "GitHub".into(),
                        role: NodeRole::External,
                    },
                ],
                messages: vec![SequenceMessage {
                    from: 0,
                    to: 1,
                    label: "GET".into(),
                    style: MessageStyle::Call,
                    note: Some("held".into()),
                    anchor: None,
                }],
            }),
        ]
    }

    /// A record of a run with a checkout, for a newcomer.
    fn record() -> BriefRecord {
        BriefRecord {
            repository: "plucked/quarterdeck".into(),
            number: 42,
            url: "https://github.com/plucked/quarterdeck/pull/42".into(),
            subject: "Pull request #42: Hold".into(),
            layers: vec![RequestLayer {
                number: 41,
                title: "Base".into(),
                description: None,
            }],
            head_sha: Some("bbb".into()),
            reader: ReaderFamiliarity::Newcomer,
            code: CodeAccess::Workspace,
            prompt: "# Pull request".into(),
            answer: None,
            transcript: None,
            tool_calls: vec![ToolCall {
                tool: "Read".into(),
                input: "file_path: head/x".into(),
            }],
            rejected: vec![RejectedWidget {
                widget: "w".into(),
                reason: "r".into(),
            }],
            cost_usd: Some(0.5),
            session_id: Some("s".into()),
            model: Some("m".into()),
            duration: Duration::from_secs(65),
            written_at: 1_794_148_320,
        }
    }

    /// A brief with every kind of widget, on two pages.
    fn brief() -> StoredBrief {
        StoredBrief {
            studio: Studio {
                lead: "The lead.".into(),
                pages: vec![
                    StudioPage {
                        title: "Overview".into(),
                        summary: "S1".into(),
                        widgets: text_widgets(),
                    },
                    StudioPage {
                        title: "Diagrams".into(),
                        summary: "S2".into(),
                        widgets: diagram_widgets(),
                    },
                ],
            },
            record: record(),
        }
    }

    /// A brief reads back exactly as stored; a second brief for the same
    /// scope replaces it; the index finds briefs by repository and number.
    #[test]
    fn stores_and_reads_a_brief() {
        let store = UiStateStore::open_in_memory().expect("store");
        let scope = ReviewScope::PullRequest(7);
        let stored = brief();

        // Store, then read back every part.
        store
            .put_brief(&scope, &stored.studio, &stored.record)
            .expect("stored");
        let read = store.brief(&scope).expect("read").expect("a brief");
        assert_eq!(read.studio, stored.studio);
        assert_eq!(read.record.layers, stored.record.layers);
        assert_eq!(read.record.head_sha, stored.record.head_sha);
        assert_eq!(read.record.tool_calls, stored.record.tool_calls);
        assert_eq!(read.record.rejected, stored.record.rejected);
        assert_eq!(read.record.duration, stored.record.duration);
        assert_eq!(read.record.reader, ReaderFamiliarity::Newcomer);

        // Writing again replaces it; another scope stays apart.
        let mut again = brief();
        again.studio.lead = "Again.".into();
        store
            .put_brief(&scope, &again.studio, &again.record)
            .expect("replaced");
        assert_eq!(
            store
                .brief(&scope)
                .expect("read")
                .expect("a brief")
                .studio
                .lead,
            "Again."
        );
        let range = ReviewScope::CombinedLayers {
            repo: RepoRef {
                owner: "plucked".into(),
                name: "quarterdeck".into(),
            },
            first_number: 40,
            last_number: 42,
        };
        let ranged = brief();
        store
            .put_brief(&range, &ranged.studio, &ranged.record)
            .expect("stored");
        let index = store.brief_index().expect("index");
        let repo = RepoRef {
            owner: "plucked".into(),
            name: "quarterdeck".into(),
        };
        assert!(index.has_brief(&repo, 42));
        assert!(!index.has_brief(&repo, 41));
    }
}
