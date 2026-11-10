use gpui::{AnyElement, Context, FontWeight, SharedString, div, prelude::*, px};
use qd_cache::QueueTab;
use qd_github::{StackLayer, ViewerReview};

use super::ReviewQueue;
use super::pull_request_row_rendering::{VERDICT_BADGE_ID, render_brief_mark};
use super::sidebar_rendering::{render_meta, row_frame};
use super::stack_group::StackGroup;
use crate::fold_chevron::{FoldChevron, FoldState};
use crate::review_verdict_badge::ReviewVerdictBadge;
use crate::theme;

/// Width of the column holding a stack layer's position and rail.
const LAYER_GUTTER_WIDTH: f32 = 28.;
/// The stack glyph beside a stack's header.
const STACK_GLYPH_SIZE: f32 = 16.;
/// Height of the rail mark beside a layer's position.
const LAYER_RAIL_HEIGHT: f32 = 16.;
/// Edge length of each of the stack glyph's two squares (they overlap).
const STACK_GLYPH_SQUARE_SIZE: f32 = STACK_GLYPH_SIZE - 5.;
/// Corner radius of the stack glyph's squares.
const STACK_GLYPH_CORNER_RADIUS: f32 = 3.;

/// The header of a stack's group: a stack glyph, its number and trunk,
/// how many layers it has and how many of them this list has, and a
/// chevron. Clicking opens the stack's combined review (highlighted while
/// shown); the chevron folds or unfolds the group.
pub(super) fn render_stack_header(
    row_index: usize,
    group: &StackGroup,
    tab: QueueTab,
    is_reviewed: bool,
    cx: &mut Context<ReviewQueue>,
) -> AnyElement {
    let open_key = group.key.clone();
    let toggle_key = group.key.clone();
    let is_all = u64::try_from(group.listed).is_ok_and(|listed| listed == group.size);
    let listed = match (tab, is_all) {
        (QueueTab::Authored, true) => "all yours".to_owned(),
        (QueueTab::Authored, false) => format!("{} yours", group.listed),
        (QueueTab::ReviewRequested, true) => "all to review".to_owned(),
        (QueueTab::ReviewRequested, false) => format!("{} to review", group.listed),
        (QueueTab::Others, true) => "all listed".to_owned(),
        (QueueTab::Others, false) => format!("{} listed", group.listed),
    };
    row_frame(("stack-header", row_index), is_reviewed)
        .flex()
        .items_center()
        .gap_2p5()
        .on_click(cx.listener(move |queue, _, _, cx| queue.open_stack(&open_key, cx)))
        .child(render_stack_glyph())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap_1p5()
                        .text_sm()
                        .child(
                            div()
                                .flex_none()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!("Stack #{}", group.key.number)),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_xs()
                                .text_color(theme::muted())
                                .child(format!("→ {}", group.base_ref)),
                        ),
                )
                .child(render_meta(format!("{} layers · {listed}", group.size))),
        )
        .child(
            FoldChevron::new(("stack-fold", row_index), FoldState::of(group.is_collapsed))
                .on_click(cx.listener(move |queue, _, _, cx| {
                    queue.toggle_stack_group(&toggle_key, cx);
                })),
        )
        .into_any_element()
}

/// Two overlapping rounded squares: the mark of a stack.
fn render_stack_glyph() -> impl IntoElement {
    div()
        .flex_none()
        .relative()
        .size(px(STACK_GLYPH_SIZE))
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size(px(STACK_GLYPH_SQUARE_SIZE))
                .rounded(px(STACK_GLYPH_CORNER_RADIUS))
                .border_1()
                .border_color(theme::accent()),
        )
        .child(
            div()
                .absolute()
                .bottom_0()
                .right_0()
                .size(px(STACK_GLYPH_SQUARE_SIZE))
                .rounded(px(STACK_GLYPH_CORNER_RADIUS))
                .bg(theme::accent()),
        )
}

/// What one layer row under a stack's header shows.
pub(super) struct LayerRow {
    pub(super) id: gpui::ElementId,
    /// 1 for the layer on the trunk.
    pub(super) position: u64,
    pub(super) title: SharedString,
    pub(super) meta: SharedString,
    /// Whether the shown list has it; other layers show muted.
    pub(super) is_listed: bool,
    pub(super) marks: LayerMarks,
    pub(super) is_selected: bool,
}

/// What a layer row marks beside its title.
pub(super) struct LayerMarks {
    /// It asks for the viewer's review (a badge says so).
    pub(super) needs_review: bool,
    /// It has a stored brief (a star says so).
    pub(super) has_brief: bool,
    /// The viewer's latest review (a badge says which), when known.
    pub(super) viewer_review: Option<ViewerReview>,
}

/// One layer of a stack, under its header, on a single line so long stacks
/// stay compact: its position on a rail, the title, a Review badge when it
/// asks for the viewer's review, a star with a stored brief, the viewer's
/// review verdict, and `#number` (with its state) on the
/// right. Layers the list lacks are muted; clicking any opens it.
pub(super) fn render_layer_row(
    row: LayerRow,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    let rail_color = if row.is_listed {
        theme::accent()
    } else {
        theme::border_strong()
    };
    row_frame(row.id, row.is_selected)
        .pl_1()
        .py_1()
        .flex()
        .items_center()
        .gap_2()
        .on_click(on_click)
        .child(
            div()
                .flex_none()
                .w(px(LAYER_GUTTER_WIDTH))
                .flex()
                .items_center()
                .justify_end()
                .gap_1p5()
                .text_xs()
                .text_color(theme::subtle())
                .child(row.position.to_string())
                .child(
                    div()
                        .w(px(2.))
                        .h(px(LAYER_RAIL_HEIGHT))
                        .rounded_full()
                        .bg(rail_color),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .truncate()
                .when(!row.is_listed, |title| title.text_color(theme::muted()))
                .child(row.title),
        )
        .when(row.marks.needs_review, |line| {
            line.child(render_review_badge())
        })
        .when(row.marks.has_brief, |line| line.child(render_brief_mark()))
        .children(
            row.marks
                .viewer_review
                .map(|review| ReviewVerdictBadge::new(VERDICT_BADGE_ID, review)),
        )
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(theme::subtle())
                .child(row.meta),
        )
        .into_any_element()
}

/// The badge on a layer that asks for the viewer's review.
fn render_review_badge() -> impl IntoElement {
    div()
        .flex_none()
        .px_1p5()
        .rounded_full()
        .bg(theme::selected())
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme::accent())
        .child("Review")
}

/// ` · merged`, ` · closed` or ` · draft` for a layer not simply open.
pub(super) fn layer_state_marker(layer: &StackLayer) -> &'static str {
    match (layer.is_merged, layer.is_open, layer.is_draft) {
        (true, _, _) => " · merged",
        (false, false, _) => " · closed",
        (false, true, true) => " · draft",
        (false, true, false) => "",
    }
}
