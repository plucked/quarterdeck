use anyhow::Result;
use qd_studio::{
    ChangeScope, CodeAnchor, StudioGraph, StudioSequence, StudioTable, StudioTour, StudioWidget,
};
use rusqlite::{Transaction, params};

use crate::brief_anchor_columns::AnchorColumns;
use crate::brief_sql_text::{
    AnchorSlot, BriefWidgetKind, ScopeList, change_as_sql_text, edge_style_as_sql_text,
    message_style_as_sql_text, role_as_sql_text, tone_as_sql_text, weight_as_sql_text,
};
use crate::cached_statements::execute_cached;

/// Writes widget `widget` at `position` (page `page_position`) and its parts.
pub(crate) fn put_widget(
    transaction: &Transaction<'_>,
    brief_id: i64,
    position: i64,
    page_position: i64,
    widget: &StudioWidget,
) -> Result<()> {
    // The shared columns: the kind, a title, a body, a tone.
    let (title, body, tone): (Option<&str>, Option<&str>, Option<&str>) = match widget {
        StudioWidget::Section { heading, body } => (Some(heading), Some(body), None),
        StudioWidget::Callout { tone, text } => (None, Some(text), Some(tone_as_sql_text(*tone))),
        StudioWidget::CodeExcerpt { caption, .. } | StudioWidget::CodeCompare { caption, .. } => {
            (Some(caption), None, None)
        }
        StudioWidget::Table(table) => (Some(table.title.as_str()), None, None),
        StudioWidget::Scope(_) => (None, None, None),
        StudioWidget::Tour(tour) => (Some(tour.title.as_str()), None, None),
        StudioWidget::Graph(graph) => (Some(graph.title.as_str()), None, None),
        StudioWidget::Sequence(sequence) => (Some(sequence.title.as_str()), None, None),
    };
    let kind = BriefWidgetKind::of(widget).as_sql_text();
    execute_cached(
        transaction,
        "INSERT INTO brief_widgets (brief_id, position, page_position, kind, title, body, tone)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![brief_id, position, page_position, kind, title, body, tone],
    )?;

    // The kind's own parts.
    match widget {
        StudioWidget::Section { .. } | StudioWidget::Callout { .. } => {}
        StudioWidget::CodeExcerpt { anchor, .. } => {
            put_slot_anchor(transaction, brief_id, position, AnchorSlot::Excerpt, anchor)?;
        }
        StudioWidget::CodeCompare { before, after, .. } => {
            put_slot_anchor(transaction, brief_id, position, AnchorSlot::Before, before)?;
            put_slot_anchor(transaction, brief_id, position, AnchorSlot::After, after)?;
        }
        StudioWidget::Table(table) => put_table(transaction, brief_id, position, table)?,
        StudioWidget::Scope(scope) => put_scope_items(transaction, brief_id, position, scope)?,
        StudioWidget::Tour(tour) => put_tour_steps(transaction, brief_id, position, tour)?,
        StudioWidget::Graph(graph) => put_graph(transaction, brief_id, position, graph)?,
        StudioWidget::Sequence(sequence) => {
            put_sequence(transaction, brief_id, position, sequence)?;
        }
    }
    Ok(())
}

/// A code widget's anchor in `slot` (excerpt, before or after).
fn put_slot_anchor(
    transaction: &Transaction<'_>,
    brief_id: i64,
    position: i64,
    slot: AnchorSlot,
    anchor: &CodeAnchor,
) -> Result<()> {
    let anchor = AnchorColumns::of(Some(anchor));
    execute_cached(
        transaction,
        "INSERT INTO brief_anchors (brief_id, widget_position, slot, path, side, first_line, last_line)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            brief_id,
            position,
            slot.as_sql_text(),
            anchor.path,
            anchor.side,
            anchor.first_line,
            anchor.last_line
        ],
    )?;
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

/// A scope's three lists: what changes, what stays the same, what is mechanical.
fn put_scope_items(
    transaction: &Transaction<'_>,
    brief_id: i64,
    position: i64,
    scope: &ChangeScope,
) -> Result<()> {
    for (list, items) in [
        (ScopeList::Changes, &scope.changes),
        (ScopeList::Unchanged, &scope.unchanged),
        (ScopeList::Mechanical, &scope.mechanical),
    ] {
        for (item_position, item) in (0_i64..).zip(items) {
            execute_cached(
                transaction,
                "INSERT INTO brief_scope_items (brief_id, widget_position, list, position, text)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![brief_id, position, list.as_sql_text(), item_position, item],
            )?;
        }
    }
    Ok(())
}

/// A reading order's steps, each with its optional anchor.
fn put_tour_steps(
    transaction: &Transaction<'_>,
    brief_id: i64,
    position: i64,
    tour: &StudioTour,
) -> Result<()> {
    for (step_position, step) in (0_i64..).zip(&tour.steps) {
        let anchor = AnchorColumns::of(step.anchor.as_ref());
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
                weight_as_sql_text(step.weight),
                anchor.path,
                anchor.side,
                anchor.first_line,
                anchor.last_line
            ],
        )?;
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
        let anchor = AnchorColumns::of(node.anchor.as_ref());
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
                role_as_sql_text(node.role),
                node.group.map(i64::try_from).transpose()?,
                change_as_sql_text(node.change),
                anchor.path,
                anchor.side,
                anchor.first_line,
                anchor.last_line
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
                edge_style_as_sql_text(edge.style),
                change_as_sql_text(edge.change)
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
                role_as_sql_text(lane.role)
            ],
        )?;
    }
    for (message_position, message) in (0_i64..).zip(&sequence.messages) {
        let anchor = AnchorColumns::of(message.anchor.as_ref());
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
                message_style_as_sql_text(message.style),
                message.note,
                anchor.path,
                anchor.side,
                anchor.first_line,
                anchor.last_line
            ],
        )?;
    }
    Ok(())
}
