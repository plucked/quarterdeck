use anyhow::{Context as _, Result, anyhow};
use qd_studio::{
    ChangeScope, CodeAnchor, GraphEdge, GraphNode, SequenceLane, SequenceMessage, StudioGraph,
    StudioPage, StudioSequence, StudioTable, StudioTour, StudioWidget, TableRole, TourStep,
};
use rusqlite::params;

use crate::UiStateStore;
use crate::brief_anchor_columns::AnchorColumns;
use crate::brief_sql_text::{
    change_from_sql_text, edge_style_from_sql_text, message_style_from_sql_text,
    role_from_sql_text, tone_from_sql_text, weight_from_sql_text,
};

/// A `brief_widgets` row as read.
struct WidgetRow {
    position: i64,
    page_position: i64,
    kind: String,
    title: Option<String>,
    body: Option<String>,
    tone: Option<String>,
}

/// A `brief_pages` row as read.
struct PageRow {
    position: i64,
    title: String,
    summary: String,
}

impl UiStateStore {
    /// The brief's pages in order, each with its widgets in order.
    pub(crate) fn brief_pages(&self, brief_id: i64) -> Result<Vec<StudioPage>> {
        // The page rows and the widget rows of the whole brief.
        let mut statement = self.connection().prepare_cached(
            "SELECT position, title, summary FROM brief_pages WHERE brief_id = ?1 ORDER BY position",
        )?;
        let pages: Vec<PageRow> = statement
            .query_map([brief_id], |row| {
                Ok(PageRow {
                    position: row.get(0)?,
                    title: row.get(1)?,
                    summary: row.get(2)?,
                })
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

        // Each page with the widgets on it, read with their parts.
        pages
            .into_iter()
            .map(|page| {
                let widgets = widgets
                    .iter()
                    .filter(|widget| widget.page_position == page.position)
                    .map(|widget| self.brief_widget(brief_id, widget))
                    .collect::<Result<_>>()?;
                Ok(StudioPage {
                    title: page.title,
                    summary: page.summary,
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
            |row| AnchorColumns::read(row, 0),
        )?;
        columns
            .into_anchor()?
            .context("an anchor without its columns")
    }

    /// The table's column names in order.
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
        // Every cell in row, then column order, with its row's position.
        let mut statement = self.connection().prepare_cached(
            "SELECT row_position, text FROM brief_table_cells
             WHERE brief_id = ?1 AND widget_position = ?2 ORDER BY row_position, column_position",
        )?;
        let cells: Vec<(i64, String)> = statement
            .query_map(params![brief_id, at], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;

        // A new row starts wherever the row position changes.
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

    /// The items of one of a scope's lists (changes, unchanged, mechanical).
    fn scope_items(&self, brief_id: i64, at: i64, list: &str) -> Result<Vec<String>> {
        let mut statement = self.connection().prepare_cached(
            "SELECT text FROM brief_scope_items WHERE brief_id = ?1 AND widget_position = ?2
             AND list = ?3 ORDER BY position",
        )?;
        let items = statement.query_map(params![brief_id, at, list], |row| row.get(0))?;
        Ok(items.collect::<rusqlite::Result<_>>()?)
    }

    /// The reading order's steps in order.
    fn tour_steps(&self, brief_id: i64, at: i64) -> Result<Vec<TourStep>> {
        let mut statement = self.connection().prepare_cached(
            "SELECT title, explanation, weight, anchor_path, anchor_side, anchor_first, anchor_last
             FROM brief_tour_steps WHERE brief_id = ?1 AND widget_position = ?2 ORDER BY position",
        )?;
        let rows: Vec<(String, String, String, AnchorColumns)> = statement
            .query_map(params![brief_id, at], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    AnchorColumns::read(row, 3)?,
                ))
            })?
            .collect::<rusqlite::Result<_>>()?;
        rows.into_iter()
            .map(|(title, explanation, weight, anchor)| {
                Ok(TourStep {
                    title,
                    explanation,
                    weight: weight_from_sql_text(&weight).context("an unknown step weight")?,
                    anchor: anchor.into_anchor()?,
                })
            })
            .collect()
    }

    /// Graph `at`: its groups, nodes and edges.
    fn graph(&self, brief_id: i64, at: i64, title: String) -> Result<StudioGraph> {
        // The groups' labels.
        let mut statement = self.connection().prepare_cached(
            "SELECT label FROM brief_graph_groups WHERE brief_id = ?1 AND widget_position = ?2
             ORDER BY position",
        )?;
        let groups = statement
            .query_map(params![brief_id, at], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;

        // The nodes, each with its optional group and anchor.
        let mut statement = self.connection().prepare_cached(
            "SELECT label, detail, role, group_position, change, anchor_path, anchor_side,
                anchor_first, anchor_last
             FROM brief_graph_nodes WHERE brief_id = ?1 AND widget_position = ?2 ORDER BY position",
        )?;
        let node_rows: Vec<(String, String, String, Option<i64>, String, AnchorColumns)> =
            statement
                .query_map(params![brief_id, at], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        AnchorColumns::read(row, 5)?,
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
                    anchor: anchor.into_anchor()?,
                    change: change_from_sql_text(&change).context("an unknown change")?,
                })
            })
            .collect::<Result<_>>()?;

        // The edges between node positions.
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
        // The lanes.
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

        // The messages between lane positions, in time order.
        let mut statement = self.connection().prepare_cached(
            "SELECT from_lane, to_lane, label, style, note, anchor_path, anchor_side, anchor_first,
                anchor_last
             FROM brief_sequence_messages WHERE brief_id = ?1 AND widget_position = ?2
             ORDER BY position",
        )?;
        let message_rows: Vec<(i64, i64, String, String, Option<String>, AnchorColumns)> =
            statement
                .query_map(params![brief_id, at], |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        AnchorColumns::read(row, 5)?,
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
                    anchor: anchor.into_anchor()?,
                })
            })
            .collect::<Result<_>>()?;
        Ok(StudioSequence {
            title,
            lanes,
            messages,
        })
    }
}
