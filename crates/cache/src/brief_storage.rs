use std::time::Duration;

use anyhow::{Context as _, Result, anyhow};
use qd_studio::{
    BriefRecord, ChangeScope, CodeAccess, CodeAnchor, GraphEdge, GraphNode, RejectedWidget,
    RequestLayer, SequenceLane, SequenceMessage, Studio, StudioGraph, StudioPage, StudioSequence,
    StudioTable, StudioTour, StudioWidget, TableRole, ToolCall, TourStep,
};
use rusqlite::{OptionalExtension, Transaction, params};

use crate::brief_sql_text::{
    change_from_sql_text, change_sql_text, edge_style_from_sql_text, edge_style_sql_text,
    message_style_from_sql_text, message_style_sql_text, reader_from_sql_text, reader_sql_text,
    role_from_sql_text, role_sql_text, side_from_sql_text, side_sql_text, tone_from_sql_text,
    tone_sql_text, weight_from_sql_text, weight_sql_text,
};
use crate::cached_statements::execute_cached;
use crate::scope_columns::ScopeColumns;
use crate::sql_integers::{read_u64, sql_integer};
use crate::{BriefIndex, ReviewScope, StoredBrief, UiStateStore};

/// The scope condition of a brief query, the scope columns as ?1 to ?5.
const SCOPE_MATCH: &str = "pr_id IS ?1 AND repo_owner IS ?2 AND repo_name IS ?3
    AND first_number IS ?4 AND last_number IS ?5";

/// An anchor as its four nullable columns: path, side, first and last line.
type AnchorColumns<'a> = (
    Option<&'a str>,
    Option<&'static str>,
    Option<i64>,
    Option<i64>,
);

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
        let columns = ScopeColumns::of(scope)?;
        execute_cached(
            &transaction,
            &format!("DELETE FROM briefs WHERE {SCOPE_MATCH}"),
            columns.as_params(),
        )?;

        // The brief's own row: its scope and the record of the run.
        let (code_access, diff_only_reason) = match &record.code {
            CodeAccess::Workspace => ("workspace", None),
            CodeAccess::DiffOnly(reason) => ("diff_only", Some(reason.as_str())),
        };
        execute_cached(&transaction,
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
        let brief_id = transaction.last_insert_rowid();

        // The layers, the pages and their widgets, numbered across the brief.
        for (position, layer) in (0_i64..).zip(&record.layers) {
            execute_cached(
                &transaction,
                "INSERT INTO brief_layers (brief_id, position, number, title) VALUES (?1, ?2, ?3, ?4)",
                params![brief_id, position, sql_integer(layer.number)?, layer.title],
            )?;
        }
        let mut widget_position = 0_i64;
        for (page_position, page) in (0_i64..).zip(&studio.pages) {
            execute_cached(
                &transaction,
                "INSERT INTO brief_pages (brief_id, position, title, summary) VALUES (?1, ?2, ?3, ?4)",
                params![brief_id, page_position, page.title, page.summary],
            )?;
            for widget in &page.widgets {
                put_widget(
                    &transaction,
                    brief_id,
                    widget_position,
                    page_position,
                    widget,
                )?;
                widget_position += 1;
            }
        }

        // What the checks left out, and what the run read.
        for (position, rejected) in (0_i64..).zip(&record.rejected) {
            execute_cached(
                &transaction,
                "INSERT INTO brief_rejected (brief_id, position, widget, reason) VALUES (?1, ?2, ?3, ?4)",
                params![brief_id, position, rejected.widget, rejected.reason],
            )?;
        }
        for (position, call) in (0_i64..).zip(&record.tool_calls) {
            execute_cached(
                &transaction,
                "INSERT INTO brief_tool_calls (brief_id, position, tool, input) VALUES (?1, ?2, ?3, ?4)",
                params![brief_id, position, call.tool, call.input],
            )?;
        }
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
            code: match (row.code_access.as_str(), row.diff_only_reason) {
                ("workspace", _) => CodeAccess::Workspace,
                (_, Some(reason)) => CodeAccess::DiffOnly(reason),
                (other, None) => return Err(anyhow!("unknown code access {other}")),
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

    /// The brief's pages in order, each with its widgets in order.
    fn brief_pages(&self, brief_id: i64) -> Result<Vec<StudioPage>> {
        let mut statement = self.connection().prepare_cached(
            "SELECT position, title, summary FROM brief_pages WHERE brief_id = ?1 ORDER BY position",
        )?;
        let pages: Vec<(i64, String, String)> = statement
            .query_map([brief_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let mut statement = self.connection().prepare_cached(
            "SELECT position, page_position, kind, title, body, tone FROM brief_widgets
             WHERE brief_id = ?1 ORDER BY position",
        )?;
        let widgets: Vec<WidgetRow> = statement
            .query_map([brief_id], |row| {
                Ok(WidgetRow {
                    position: row.get(0)?,
                    page_position: row.get(1)?,
                    kind: row.get(2)?,
                    title: row.get(3)?,
                    body: row.get(4)?,
                    tone: row.get(5)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        pages
            .into_iter()
            .map(|(position, title, summary)| {
                let widgets = widgets
                    .iter()
                    .filter(|widget| widget.page_position == position)
                    .map(|widget| self.brief_widget(brief_id, widget))
                    .collect::<Result<_>>()?;
                Ok(StudioPage {
                    title,
                    summary,
                    widgets,
                })
            })
            .collect()
    }

    /// One widget from its row and its parts' rows.
    fn brief_widget(&self, brief_id: i64, row: &WidgetRow) -> Result<StudioWidget> {
        let at = row.position;
        let title = || row.title.clone().unwrap_or_default();
        Ok(match row.kind.as_str() {
            "section" => StudioWidget::Section {
                heading: title(),
                body: row.body.clone().unwrap_or_default(),
            },
            "callout" => StudioWidget::Callout {
                tone: row
                    .tone
                    .as_deref()
                    .and_then(tone_from_sql_text)
                    .context("a callout without a known tone")?,
                text: row.body.clone().unwrap_or_default(),
            },
            "code_excerpt" => StudioWidget::CodeExcerpt {
                caption: title(),
                anchor: self.slot_anchor(brief_id, at, "excerpt")?,
            },
            "code_compare" => StudioWidget::CodeCompare {
                caption: title(),
                before: self.slot_anchor(brief_id, at, "before")?,
                after: self.slot_anchor(brief_id, at, "after")?,
            },
            "table" | "trace" => StudioWidget::Table(StudioTable {
                title: title(),
                columns: self.table_columns(brief_id, at)?,
                rows: self.table_rows(brief_id, at)?,
                role: if row.kind == "trace" {
                    TableRole::Trace
                } else {
                    TableRole::Mapping
                },
            }),
            "scope" => StudioWidget::Scope(ChangeScope {
                changes: self.scope_items(brief_id, at, "changes")?,
                unchanged: self.scope_items(brief_id, at, "unchanged")?,
                mechanical: self.scope_items(brief_id, at, "mechanical")?,
            }),
            "tour" => StudioWidget::Tour(StudioTour {
                title: title(),
                steps: self.tour_steps(brief_id, at)?,
            }),
            "graph" => StudioWidget::Graph(self.graph(brief_id, at, title())?),
            "sequence" => StudioWidget::Sequence(self.sequence(brief_id, at, title())?),
            other => return Err(anyhow!("unknown widget kind {other}")),
        })
    }

    /// The anchor in `slot` of widget `at`.
    fn slot_anchor(&self, brief_id: i64, at: i64, slot: &str) -> Result<CodeAnchor> {
        let columns = self.connection().query_row(
            "SELECT path, side, first_line, last_line FROM brief_anchors
             WHERE brief_id = ?1 AND widget_position = ?2 AND slot = ?3",
            params![brief_id, at, slot],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;
        anchor_from_columns(columns)?.context("an anchor without its columns")
    }

    fn table_columns(&self, brief_id: i64, at: i64) -> Result<Vec<String>> {
        let mut statement = self.connection().prepare_cached(
            "SELECT name FROM brief_table_columns WHERE brief_id = ?1 AND widget_position = ?2
             ORDER BY position",
        )?;
        let names = statement.query_map(params![brief_id, at], |row| row.get(0))?;
        Ok(names.collect::<rusqlite::Result<_>>()?)
    }

    /// The table's rows, each its cells in column order.
    fn table_rows(&self, brief_id: i64, at: i64) -> Result<Vec<Vec<String>>> {
        let mut statement = self.connection().prepare_cached(
            "SELECT row_position, text FROM brief_table_cells
             WHERE brief_id = ?1 AND widget_position = ?2 ORDER BY row_position, column_position",
        )?;
        let cells: Vec<(i64, String)> = statement
            .query_map(params![brief_id, at], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut current = None;
        for (row_position, text) in cells {
            if current != Some(row_position) {
                rows.push(Vec::new());
                current = Some(row_position);
            }
            if let Some(row) = rows.last_mut() {
                row.push(text);
            }
        }
        Ok(rows)
    }

    fn scope_items(&self, brief_id: i64, at: i64, list: &str) -> Result<Vec<String>> {
        let mut statement = self.connection().prepare_cached(
            "SELECT text FROM brief_scope_items WHERE brief_id = ?1 AND widget_position = ?2
             AND list = ?3 ORDER BY position",
        )?;
        let items = statement.query_map(params![brief_id, at, list], |row| row.get(0))?;
        Ok(items.collect::<rusqlite::Result<_>>()?)
    }

    fn tour_steps(&self, brief_id: i64, at: i64) -> Result<Vec<TourStep>> {
        let mut statement = self.connection().prepare_cached(
            "SELECT title, explanation, weight, anchor_path, anchor_side, anchor_first, anchor_last
             FROM brief_tour_steps WHERE brief_id = ?1 AND widget_position = ?2 ORDER BY position",
        )?;
        let rows: Vec<(String, String, String, AnchorRow)> = statement
            .query_map(params![brief_id, at], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    (row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?),
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        rows.into_iter()
            .map(|(title, explanation, weight, anchor)| {
                Ok(TourStep {
                    title,
                    explanation,
                    weight: weight_from_sql_text(&weight).context("an unknown step weight")?,
                    anchor: anchor_from_columns(anchor)?,
                })
            })
            .collect()
    }

    /// Graph `at`: its groups, nodes and edges.
    fn graph(&self, brief_id: i64, at: i64, title: String) -> Result<StudioGraph> {
        let mut statement = self.connection().prepare_cached(
            "SELECT label FROM brief_graph_groups WHERE brief_id = ?1 AND widget_position = ?2
             ORDER BY position",
        )?;
        let groups = statement
            .query_map(params![brief_id, at], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        let mut statement = self.connection().prepare_cached(
            "SELECT label, detail, role, group_position, change, anchor_path, anchor_side,
                anchor_first, anchor_last
             FROM brief_graph_nodes WHERE brief_id = ?1 AND widget_position = ?2 ORDER BY position",
        )?;
        let node_rows: Vec<(String, String, String, Option<i64>, String, AnchorRow)> = statement
            .query_map(params![brief_id, at], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    (row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?),
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let nodes = node_rows
            .into_iter()
            .map(|(label, detail, role, group, change, anchor)| {
                Ok(GraphNode {
                    label,
                    detail,
                    role: role_from_sql_text(&role).context("an unknown node role")?,
                    group: group.map(usize::try_from).transpose()?,
                    anchor: anchor_from_columns(anchor)?,
                    change: change_from_sql_text(&change).context("an unknown change")?,
                })
            })
            .collect::<Result<_>>()?;
        let mut statement = self.connection().prepare_cached(
            "SELECT from_position, to_position, label, style, change FROM brief_graph_edges
             WHERE brief_id = ?1 AND widget_position = ?2 ORDER BY position",
        )?;
        let edge_rows: Vec<(i64, i64, String, String, String)> = statement
            .query_map(params![brief_id, at], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let edges = edge_rows
            .into_iter()
            .map(|(from, to, label, style, change)| {
                Ok(GraphEdge {
                    from: usize::try_from(from)?,
                    to: usize::try_from(to)?,
                    label,
                    style: edge_style_from_sql_text(&style).context("an unknown edge style")?,
                    change: change_from_sql_text(&change).context("an unknown change")?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(StudioGraph {
            title,
            groups,
            nodes,
            edges,
        })
    }

    /// Sequence `at`: its lanes and messages.
    fn sequence(&self, brief_id: i64, at: i64, title: String) -> Result<StudioSequence> {
        let mut statement = self.connection().prepare_cached(
            "SELECT label, role FROM brief_sequence_lanes WHERE brief_id = ?1 AND widget_position = ?2
             ORDER BY position",
        )?;
        let lane_rows: Vec<(String, String)> = statement
            .query_map(params![brief_id, at], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        let lanes = lane_rows
            .into_iter()
            .map(|(label, role)| {
                Ok(SequenceLane {
                    label,
                    role: role_from_sql_text(&role).context("an unknown lane role")?,
                })
            })
            .collect::<Result<_>>()?;
        let mut statement = self.connection().prepare_cached(
            "SELECT from_lane, to_lane, label, style, note, anchor_path, anchor_side, anchor_first,
                anchor_last
             FROM brief_sequence_messages WHERE brief_id = ?1 AND widget_position = ?2
             ORDER BY position",
        )?;
        let message_rows: Vec<(i64, i64, String, String, Option<String>, AnchorRow)> = statement
            .query_map(params![brief_id, at], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    (row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?),
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        let messages = message_rows
            .into_iter()
            .map(|(from, to, label, style, note, anchor)| {
                Ok(SequenceMessage {
                    from: usize::try_from(from)?,
                    to: usize::try_from(to)?,
                    label,
                    style: message_style_from_sql_text(&style)
                        .context("an unknown message style")?,
                    note,
                    anchor: anchor_from_columns(anchor)?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(StudioSequence {
            title,
            lanes,
            messages,
        })
    }

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

/// A `brief_widgets` row as read.
struct WidgetRow {
    position: i64,
    page_position: i64,
    kind: String,
    title: Option<String>,
    body: Option<String>,
    tone: Option<String>,
}

/// An anchor's four columns as read.
type AnchorRow = (Option<String>, Option<String>, Option<i64>, Option<i64>);

/// Writes widget `widget` at `position` (page `page_position`) and its parts.
fn put_widget(
    transaction: &Transaction<'_>,
    brief_id: i64,
    position: i64,
    page_position: i64,
    widget: &StudioWidget,
) -> Result<()> {
    // The shared columns: the kind, a title, a body, a tone.
    let (kind, title, body, tone): (&str, Option<&str>, Option<&str>, Option<&str>) = match widget {
        StudioWidget::Section { heading, body } => ("section", Some(heading), Some(body), None),
        StudioWidget::Callout { tone, text } => {
            ("callout", None, Some(text), Some(tone_sql_text(*tone)))
        }
        StudioWidget::CodeExcerpt { caption, .. } => ("code_excerpt", Some(caption), None, None),
        StudioWidget::CodeCompare { caption, .. } => ("code_compare", Some(caption), None, None),
        StudioWidget::Table(table) => match table.role {
            TableRole::Mapping => ("table", Some(table.title.as_str()), None, None),
            TableRole::Trace => ("trace", Some(table.title.as_str()), None, None),
        },
        StudioWidget::Scope(_) => ("scope", None, None, None),
        StudioWidget::Tour(tour) => ("tour", Some(tour.title.as_str()), None, None),
        StudioWidget::Graph(graph) => ("graph", Some(graph.title.as_str()), None, None),
        StudioWidget::Sequence(sequence) => ("sequence", Some(sequence.title.as_str()), None, None),
    };
    execute_cached(
        transaction,
        "INSERT INTO brief_widgets (brief_id, position, page_position, kind, title, body, tone)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![brief_id, position, page_position, kind, title, body, tone],
    )?;

    // The kind's own parts.
    let put_anchor = |slot: &str, anchor: &CodeAnchor| {
        execute_cached(transaction,
            "INSERT INTO brief_anchors (brief_id, widget_position, slot, path, side, first_line, last_line)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                brief_id,
                position,
                slot,
                anchor.path,
                side_sql_text(anchor.side),
                i64::from(anchor.first_line),
                i64::from(anchor.last_line)
            ],
        )
    };
    match widget {
        StudioWidget::Section { .. } | StudioWidget::Callout { .. } => {}
        StudioWidget::CodeExcerpt { anchor, .. } => {
            put_anchor("excerpt", anchor)?;
        }
        StudioWidget::CodeCompare { before, after, .. } => {
            put_anchor("before", before)?;
            put_anchor("after", after)?;
        }
        StudioWidget::Table(table) => put_table(transaction, brief_id, position, table)?,
        StudioWidget::Scope(scope) => {
            for (list, items) in [
                ("changes", &scope.changes),
                ("unchanged", &scope.unchanged),
                ("mechanical", &scope.mechanical),
            ] {
                for (item_position, item) in (0_i64..).zip(items) {
                    execute_cached(transaction,
                        "INSERT INTO brief_scope_items (brief_id, widget_position, list, position, text)
                         VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![brief_id, position, list, item_position, item],
                    )?;
                }
            }
        }
        StudioWidget::Tour(tour) => {
            for (step_position, step) in (0_i64..).zip(&tour.steps) {
                let (path, side, first, last) = anchor_columns(step.anchor.as_ref());
                execute_cached(
                    transaction,
                    "INSERT INTO brief_tour_steps (brief_id, widget_position, position, title,
                        explanation, weight, anchor_path, anchor_side, anchor_first, anchor_last)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        brief_id,
                        position,
                        step_position,
                        step.title,
                        step.explanation,
                        weight_sql_text(step.weight),
                        path,
                        side,
                        first,
                        last
                    ],
                )?;
            }
        }
        StudioWidget::Graph(graph) => put_graph(transaction, brief_id, position, graph)?,
        StudioWidget::Sequence(sequence) => {
            put_sequence(transaction, brief_id, position, sequence)?;
        }
    }
    Ok(())
}

/// A table's columns and cells.
fn put_table(
    transaction: &Transaction<'_>,
    brief_id: i64,
    position: i64,
    table: &StudioTable,
) -> Result<()> {
    for (column_position, column) in (0_i64..).zip(&table.columns) {
        execute_cached(
            transaction,
            "INSERT INTO brief_table_columns (brief_id, widget_position, position, name)
             VALUES (?1, ?2, ?3, ?4)",
            params![brief_id, position, column_position, column],
        )?;
    }
    for (row_position, row) in (0_i64..).zip(&table.rows) {
        for (column_position, cell) in (0_i64..).zip(row) {
            execute_cached(
                transaction,
                "INSERT INTO brief_table_cells (brief_id, widget_position, row_position,
                    column_position, text) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![brief_id, position, row_position, column_position, cell],
            )?;
        }
    }
    Ok(())
}

/// A graph's groups, nodes and edges.
fn put_graph(
    transaction: &Transaction<'_>,
    brief_id: i64,
    position: i64,
    graph: &StudioGraph,
) -> Result<()> {
    for (group_position, group) in (0_i64..).zip(&graph.groups) {
        execute_cached(
            transaction,
            "INSERT INTO brief_graph_groups (brief_id, widget_position, position, label)
             VALUES (?1, ?2, ?3, ?4)",
            params![brief_id, position, group_position, group],
        )?;
    }
    for (node_position, node) in (0_i64..).zip(&graph.nodes) {
        let (path, side, first, last) = anchor_columns(node.anchor.as_ref());
        execute_cached(
            transaction,
            "INSERT INTO brief_graph_nodes (brief_id, widget_position, position, label, detail,
                role, group_position, change, anchor_path, anchor_side, anchor_first, anchor_last)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                brief_id,
                position,
                node_position,
                node.label,
                node.detail,
                role_sql_text(node.role),
                node.group.map(i64::try_from).transpose()?,
                change_sql_text(node.change),
                path,
                side,
                first,
                last
            ],
        )?;
    }
    for (edge_position, edge) in (0_i64..).zip(&graph.edges) {
        execute_cached(
            transaction,
            "INSERT INTO brief_graph_edges (brief_id, widget_position, position, from_position,
                to_position, label, style, change) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                brief_id,
                position,
                edge_position,
                i64::try_from(edge.from)?,
                i64::try_from(edge.to)?,
                edge.label,
                edge_style_sql_text(edge.style),
                change_sql_text(edge.change)
            ],
        )?;
    }
    Ok(())
}

/// A sequence's lanes and messages.
fn put_sequence(
    transaction: &Transaction<'_>,
    brief_id: i64,
    position: i64,
    sequence: &StudioSequence,
) -> Result<()> {
    for (lane_position, lane) in (0_i64..).zip(&sequence.lanes) {
        execute_cached(
            transaction,
            "INSERT INTO brief_sequence_lanes (brief_id, widget_position, position, label, role)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                brief_id,
                position,
                lane_position,
                lane.label,
                role_sql_text(lane.role)
            ],
        )?;
    }
    for (message_position, message) in (0_i64..).zip(&sequence.messages) {
        let (path, side, first, last) = anchor_columns(message.anchor.as_ref());
        execute_cached(
            transaction,
            "INSERT INTO brief_sequence_messages (brief_id, widget_position, position, from_lane,
                to_lane, label, style, note, anchor_path, anchor_side, anchor_first, anchor_last)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                brief_id,
                position,
                message_position,
                i64::try_from(message.from)?,
                i64::try_from(message.to)?,
                message.label,
                message_style_sql_text(message.style),
                message.note,
                path,
                side,
                first,
                last
            ],
        )?;
    }
    Ok(())
}

/// An optional anchor as its four nullable columns.
fn anchor_columns(anchor: Option<&CodeAnchor>) -> AnchorColumns<'_> {
    match anchor {
        Some(anchor) => (
            Some(anchor.path.as_str()),
            Some(side_sql_text(anchor.side)),
            Some(i64::from(anchor.first_line)),
            Some(i64::from(anchor.last_line)),
        ),
        None => (None, None, None, None),
    }
}

/// An anchor from its four columns: `None` when they are all NULL, an error
/// when only some are or a value does not fit.
fn anchor_from_columns(columns: AnchorRow) -> Result<Option<CodeAnchor>> {
    match columns {
        (Some(path), Some(side), Some(first), Some(last)) => Ok(Some(CodeAnchor {
            path,
            side: side_from_sql_text(&side).context("an unknown anchor side")?,
            first_line: u32::try_from(first)?,
            last_line: u32::try_from(last)?,
        })),
        (None, None, None, None) => Ok(None),
        _ => Err(anyhow!("an anchor with some of its columns missing")),
    }
}

#[cfg(test)]
mod tests {
    use qd_github::RepoRef;
    use qd_studio::{
        CalloutTone, CodeSide, DiagramChange, EdgeStyle, MessageStyle, NodeRole, ReaderFamiliarity,
        StepWeight,
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
