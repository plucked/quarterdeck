use gpui::{
    AnyElement, Context, Div, FontWeight, IntoElement, SharedString, Stateful, div, prelude::*, px,
};
use qd_github::{CheckOutcome, StatusCheck};

use super::check_outcome_mark::CheckOutcomeMark;
use super::check_row::TreeDepth;
use super::check_tally::CheckTally;
use super::check_tree::CheckTree;
use super::checks_panel::ChecksPanel;
use crate::controls::{FOLD_CHEVRON_SIZE, FoldChevron, FoldState};
use crate::display_text::{short_duration, unix_now, unix_seconds};
use crate::markdown_view::is_openable;
use crate::theme;

/// Diameter of a row's mark.
const ROW_MARK_SIZE: f32 = 14.;
/// Height of a tree row.
const TREE_ROW_HEIGHT: f32 = 26.;
/// Indent per tree level.
pub(super) const TREE_INDENT: f32 = 12.;
/// Left padding of a tree row at the top level.
const TREE_PADDING: f32 = 6.;
/// How far a flagged row's mark and its right column sit down, to line up
/// with the check's name.
const FLAGGED_ROW_DROP: f32 = 2.;
/// Size of the text in a `Required` pill.
const REQUIRED_PILL_TEXT_SIZE: f32 = 10.;

/// The parts of a workflow or group row.
pub(super) struct NodeRow {
    pub(super) id: gpui::ElementId,
    pub(super) indent: f32,
    pub(super) name: SharedString,
    pub(super) tally: CheckTally,
    /// The required checks inside, which the row's pill counts.
    pub(super) required: CheckTally,
    pub(super) fold: FoldState,
    pub(super) weight: FontWeight,
}

/// A foldable row: chevron, outcome mark, name, a pill when required
/// checks are inside, then its progress on the right: how many failed,
/// else how many of those that ran are done.
pub(super) fn render_node_row(row: NodeRow) -> Stateful<Div> {
    let (progress, color) = match (row.tally.broken(), row.tally.ran()) {
        (0, 0) => ("skipped".to_owned(), theme::subtle()),
        (0, ran) => (format!("{}/{ran}", row.tally.finished()), theme::subtle()),
        (broken, _) => (format!("{broken} failed"), theme::danger()),
    };
    tree_row(row.id, row.indent)
        .child(FoldChevron::new("fold", row.fold))
        .child(CheckOutcomeMark::new(
            "mark",
            row.tally.outcome().unwrap_or(CheckOutcome::Neutral),
            ROW_MARK_SIZE,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .font_weight(row.weight)
                .text_color(theme::text())
                .child(row.name),
        )
        .children(render_required_count(&row.required))
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(color)
                .child(progress),
        )
}

/// A check in the tree: indented past the chevrons, its mark, its name in
/// its group, how long it took (or runs); a click opens its page.
pub(super) fn render_job_row(
    tree: &CheckTree,
    index: usize,
    depth: TreeDepth,
    cx: &mut Context<ChecksPanel>,
) -> AnyElement {
    let Some(check) = tree.check(index) else {
        return div().into_any_element();
    };
    let indent = match depth {
        TreeDepth::InWorkflow => TREE_INDENT,
        TreeDepth::InGroup => 2. * TREE_INDENT,
    };
    let is_skipped = check.outcome == CheckOutcome::Skipped;
    tree_row(("check-job", index).into(), indent)
        .child(div().flex_none().w(px(FOLD_CHEVRON_SIZE)))
        .child(CheckOutcomeMark::new("mark", check.outcome, ROW_MARK_SIZE))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_color(if is_skipped {
                    theme::subtle()
                } else {
                    theme::text()
                })
                .child(tree.job_name(index).to_owned()),
        )
        .children(render_required_tag(check))
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(theme::subtle())
                .child(timing(check, unix_now())),
        )
        .map(|row| with_details_link(row, check, cx))
        .into_any_element()
}

/// A failed or pending check above the tree: its mark, name and where it
/// sits (`Workflow › Group`) under it, how long it ran; a click opens its page.
pub(super) fn render_flagged_row(
    tree: &CheckTree,
    index: usize,
    cx: &mut Context<ChecksPanel>,
) -> AnyElement {
    let Some(check) = tree.check(index) else {
        return div().into_any_element();
    };
    div()
        .id(("check-flagged", index))
        .flex()
        .items_start()
        .gap_2()
        .px_3()
        .py_1()
        .text_sm()
        .hover(|style| style.bg(theme::hover()))
        .child(div().pt(px(FLAGGED_ROW_DROP)).child(CheckOutcomeMark::new(
            "mark",
            check.outcome,
            ROW_MARK_SIZE,
        )))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .truncate()
                        .text_color(theme::text())
                        .child(tree.job_name(index).to_owned()),
                )
                .child(
                    div()
                        .truncate()
                        .text_xs()
                        .text_color(theme::subtle())
                        .child(tree.path_of(index)),
                ),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap_1p5()
                .pt(px(FLAGGED_ROW_DROP))
                .children(render_required_tag(check))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::subtle())
                        .child(timing(check, unix_now())),
                ),
        )
        .map(|row| with_details_link(row, check, cx))
        .into_any_element()
}

/// The frame of a tree row: one line, indented, hovering.
fn tree_row(id: gpui::ElementId, indent: f32) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .items_center()
        .gap_1p5()
        .h(px(TREE_ROW_HEIGHT))
        .pl(px(TREE_PADDING + indent))
        .pr_3()
        .text_sm()
        .cursor_pointer()
        .hover(|style| style.bg(theme::hover()))
}

/// Makes `row` open the check's page on GitHub (or its app) when clicked;
/// only `http(s)` links open, since the URL comes from GitHub's data.
fn with_details_link(
    row: Stateful<Div>,
    check: &StatusCheck,
    cx: &mut Context<ChecksPanel>,
) -> Stateful<Div> {
    let Some(url) = check.details_url.clone().filter(|url| is_openable(url)) else {
        return row.cursor_default();
    };
    row.cursor_pointer()
        .on_click(cx.listener(move |_, _, _, cx| cx.open_url(&url)))
}

/// A small `Required` pill for a check branch protection requires, in the
/// danger color when it failed or was cancelled (as its workflow's pill).
fn render_required_tag(check: &StatusCheck) -> Option<impl IntoElement> {
    let state = match check.outcome {
        CheckOutcome::Failed | CheckOutcome::Cancelled => RequiredState::Broken,
        _ => RequiredState::Fine,
    };
    check
        .is_required
        .then(|| render_required_pill("Required".to_owned(), state))
}

/// The pill of a workflow or group holding required checks, so a folded
/// row shows them: `Required` for one, `3 required` for more, in the
/// danger color while one of them failed or was cancelled.
fn render_required_count(required: &CheckTally) -> Option<impl IntoElement> {
    let label = match required.total() {
        0 => return None,
        1 => "Required".to_owned(),
        count => format!("{count} required"),
    };
    let state = match required.broken() {
        0 => RequiredState::Fine,
        _ => RequiredState::Broken,
    };
    Some(render_required_pill(label, state))
}

/// Whether the required checks a pill stands for are failing.
#[derive(Clone, Copy)]
enum RequiredState {
    Fine,
    Broken,
}

/// The pill marking required checks.
fn render_required_pill(label: String, state: RequiredState) -> impl IntoElement {
    let (border, text) = match state {
        RequiredState::Fine => (theme::border_strong(), theme::muted()),
        RequiredState::Broken => (theme::danger(), theme::danger()),
    };
    div()
        .flex_none()
        .px_1p5()
        .rounded_full()
        .border_1()
        .border_color(border)
        .text_size(px(REQUIRED_PILL_TEXT_SIZE))
        .text_color(text)
        .child(label)
}

/// How long a finished check took, how long a running one runs, or that it waits.
fn timing(check: &StatusCheck, now: i64) -> String {
    let started = check.started_at.as_deref().and_then(unix_seconds);
    let completed = check.completed_at.as_deref().and_then(unix_seconds);
    match (check.outcome, started, completed) {
        (CheckOutcome::Queued, _, _) => "queued".to_owned(),
        (CheckOutcome::Skipped, _, _) => String::new(),
        (CheckOutcome::Running, Some(started), _) => short_duration(now - started),
        (_, Some(started), Some(completed)) => short_duration(completed - started),
        _ => String::new(),
    }
}
