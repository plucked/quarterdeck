use gpui::{
    AnyElement, App, ClickEvent, Context, FontWeight, SharedString, Window, div, prelude::*, px,
};
use qd_cache::QueueTab;
use qd_github::{PullRequest, RepoRef, StackLayer, ViewerReview};

use super::ReviewQueue;
use super::pull_request_row_rendering::render_brief_mark;
use super::queue_trees::LayerRail;
use super::review_verdict_badge::ReviewVerdictBadge;
use super::sidebar_rendering::row_frame;
use super::stack_group::StackGroup;
use crate::controls::{FoldChevron, FoldState, NumberFlow, Pill, PillStyle, TextTooltip};
use crate::count_as_f32::count_as_f32;
use crate::theme;

const ROW_HEIGHT: f32 = 42.;
const LANE_WIDTH: f32 = 16.;
const MAX_RAIL_INDENT: f32 = 84.;
const NODE_RADIUS: f32 = 4.;

impl ReviewQueue {
    /// What the row of a stack layer in the shown list
    /// shows; context uses the same fields in a quieter color.
    pub(super) fn listed_layer_row(
        &self,
        pull_request: &PullRequest,
        selected_id: Option<u64>,
        is_listed: bool,
    ) -> LayerRow {
        let draft_marker = if pull_request.draft { " · draft" } else { "" };
        LayerRow {
            number: pull_request.number,
            id: ("stack-layer", pull_request.id).into(),
            title: pull_request.title.clone().into(),
            meta: format!(
                "#{}{draft_marker}{}",
                pull_request.number,
                pull_request
                    .branches
                    .as_ref()
                    .map(|branches| format!(" · {}", branches.head_ref))
                    .unwrap_or_default()
            )
            .into(),
            is_listed,
            marks: LayerMarks {
                show_review_badge: match self.tab() {
                    // A prior verdict already gives this requested layer a "Review again" badge.
                    QueueTab::ReviewRequested => is_listed && pull_request.viewer_review.is_none(),
                    QueueTab::Authored | QueueTab::Others => {
                        self.needs_review(&pull_request.repo, pull_request.number)
                    }
                },
                has_brief: self.has_brief(&pull_request.repo, pull_request.number),
                viewer_review: pull_request.viewer_review,
            },
            is_selected: selected_id == Some(pull_request.id),
        }
    }

    /// What the row of `stack_layer`, layer
    /// `layer` of a stack in `repo` that the shown list lacks, shows.
    pub(super) fn other_layer_row(&self, repo: &RepoRef, stack_layer: &StackLayer) -> LayerRow {
        LayerRow {
            number: stack_layer.number,
            id: SharedString::from(format!(
                "other-layer-{}-{}",
                repo.full_name(),
                stack_layer.number
            ))
            .into(),
            title: stack_layer.title.clone().into(),
            meta: format!(
                "#{}{} · {}",
                stack_layer.number,
                layer_state_marker(stack_layer),
                stack_layer.head_ref
            )
            .into(),
            is_listed: false,
            marks: LayerMarks {
                show_review_badge: self.needs_review(repo, stack_layer.number),
                has_brief: self.has_brief(repo, stack_layer.number),
                viewer_review: self
                    .lists()
                    .find_by_number(repo, stack_layer.number)
                    .and_then(|listed| listed.viewer_review),
            },
            is_selected: self.is_selected(repo, stack_layer.number),
        }
    }
}

/// The click on the row of layer `number` of a stack in `repo`: ⇧-click
/// starts the shown stack's combined review there; a plain click, or a
/// ⇧-click on a stack not shown, runs `open`.
pub(super) fn on_layer_click(
    repo: RepoRef,
    number: u64,
    open: impl Fn(&mut ReviewQueue, &mut Context<ReviewQueue>) + 'static,
    cx: &mut Context<ReviewQueue>,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    cx.listener(move |queue, event: &ClickEvent, _, cx| {
        if !(event.modifiers().shift && queue.start_combined_at(&repo, number, cx)) {
            open(queue, cx);
        }
    })
}

/// The header of a stack's group: a stack glyph, its number and trunk,
/// how many layers it has and how many of them this list has, and a
/// chevron. Clicking the header or chevron folds or unfolds the group.
pub(super) fn render_stack_header(
    row_index: usize,
    group: &StackGroup,
    tab: QueueTab,
    cx: &mut Context<ReviewQueue>,
) -> AnyElement {
    let fold_key = group.key.clone();
    let toggle_key = group.key.clone();
    let is_all = u64::try_from(group.listed).is_ok_and(|listed| listed == group.size);
    let summary = match (tab, is_all) {
        (QueueTab::Authored, true) => format!("{} layers · all yours", group.size),
        (QueueTab::Authored, false) => format!("{} layers · {} yours", group.size, group.listed),
        (QueueTab::ReviewRequested, _) => format!("{} of {} to review", group.listed, group.size),
        (QueueTab::Others, true) => format!("{} layers · all listed", group.size),
        (QueueTab::Others, false) => format!("{} layers · {} listed", group.size, group.listed),
    };
    let title = if group.source == super::stack_group::StackSource::GitHub {
        format!("Stack #{}", group.key.number)
    } else {
        format!("Stack from #{}", group.root)
    };
    let source = group.source.label();
    let row_id = SharedString::from(format!(
        "stack-header-{}-{}-{}",
        group.key.branch_root,
        group.key.repo.full_name(),
        group.key.number
    ));
    row_frame(row_id, false)
        .flex()
        .items_center()
        .gap_2p5()
        .on_click(cx.listener(move |queue, _, _, cx| queue.toggle_stack_group(&fold_key, cx)))
        .child(render_stack_glyph(group.branching))
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
                                .child(NumberFlow::label(title)),
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
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::subtle())
                        .child(format!("{source} · {summary}")),
                ),
        )
        .child(
            FoldChevron::new(("stack-fold", row_index), FoldState::of(group.is_collapsed))
                .on_click(
                    cx.listener(move |queue, _, _, cx| queue.toggle_stack_group(&toggle_key, cx)),
                ),
        )
        .into_any_element()
}

/// The same line and nodes as the rows below, in miniature.
fn render_stack_glyph(branching: bool) -> impl IntoElement {
    let color = theme::accent();
    let node = |x, y| {
        div()
            .absolute()
            .left(px(x))
            .top(px(y))
            .size(px(5.))
            .rounded_full()
            .bg(color)
    };
    let left = if branching { 3. } else { 7. };
    div()
        .relative()
        .flex_none()
        .w(px(18.))
        .h(px(20.))
        .child(
            div()
                .absolute()
                .left(px(left + 2.))
                .top(px(3.))
                .w(px(1.))
                .h(px(14.))
                .bg(color),
        )
        .child(node(left, 1.))
        .child(node(left, 15.))
        .when(branching, |icon| {
            icon.child(
                div()
                    .absolute()
                    .left(px(left + 2.))
                    .top(px(10.))
                    .w(px(10.))
                    .h(px(1.))
                    .bg(color),
            )
            .child(node(left + 9., 8.))
        })
}

/// The gutter has no vertical padding, so lines meet the neighboring virtual rows.
fn render_rail(
    rail: &LayerRail,
    max_depth: usize,
    selected: bool,
    on_path: bool,
) -> impl IntoElement {
    let depth = rail.lanes.len();
    let pitch = LANE_WIDTH.min(MAX_RAIL_INDENT / count_as_f32(max_depth.max(1)));
    let x = |lane| 6. + count_as_f32(lane) * pitch;
    let wire = theme::border_strong();
    let half = ROW_HEIGHT / 2.;
    let vertical = |lane, top, height| {
        div()
            .absolute()
            .left(px(x(lane)))
            .top(px(top))
            .w(px(1.))
            .h(px(height))
            .bg(wire)
    };
    let mut gutter = div()
        .relative()
        .flex_none()
        .w(px(x(depth) + 6.))
        .h(px(ROW_HEIGHT));
    for (lane, continues) in rail.lanes.iter().enumerate() {
        if lane + 1 == depth {
            let height = if *continues { ROW_HEIGHT } else { half };
            gutter = gutter.child(vertical(lane, 0., height)).child(
                div()
                    .absolute()
                    .left(px(x(lane)))
                    .top(px(half))
                    .w(px(pitch))
                    .h(px(1.))
                    .bg(wire),
            );
        } else if *continues {
            gutter = gutter.child(vertical(lane, 0., ROW_HEIGHT));
        }
    }
    gutter
        .when(rail.straight_in, |g| g.child(vertical(depth, 0., half)))
        .when(rail.straight_out, |g| g.child(vertical(depth, half, half)))
        .child(
            div()
                .absolute()
                .left(px(x(depth) - NODE_RADIUS))
                .top(px(half - NODE_RADIUS))
                .size(px(NODE_RADIUS * 2.))
                .rounded_full()
                .border_2()
                .border_color(if selected || on_path {
                    theme::accent()
                } else {
                    theme::subtle()
                })
                .bg(if selected {
                    theme::accent()
                } else {
                    theme::sidebar()
                }),
        )
}

/// What one layer row under a stack's header shows.
pub(super) struct LayerRow {
    id: gpui::ElementId,
    number: u64,
    title: SharedString,
    meta: SharedString,
    /// Whether the shown list has it; other layers show muted.
    is_listed: bool,
    marks: LayerMarks,
    is_selected: bool,
}

/// What a layer row marks beside its title.
struct LayerMarks {
    /// Show the Review badge beside the title.
    show_review_badge: bool,
    /// It has a stored brief (a star says so).
    has_brief: bool,
    /// The viewer's latest review (a badge says which), when known.
    viewer_review: Option<ViewerReview>,
}

/// The selectable PR, its branch and review marks beside a continuous dependency rail.
pub(super) fn render_layer_row(
    tab: QueueTab,
    row: LayerRow,
    rail: &LayerRail,
    max_depth: usize,
    on_path: bool,
    parent_hint: Option<String>,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    row_frame(row.id, row.is_selected)
        .when_some(parent_hint, |frame, hint| {
            frame.tooltip(TextTooltip::builder(hint))
        })
        .pl_1()
        .py_0()
        .h(px(ROW_HEIGHT))
        .flex()
        .items_center()
        .gap_2()
        .on_click(on_click)
        .child(render_rail(rail, max_depth, row.is_selected, on_path))
        .child(
            div()
                .debug_selector(move || format!("queue-layer-title-{}", row.number))
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .when(!row.is_listed, |title| title.text_color(theme::muted()))
                        .child(row.title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::subtle())
                        .font_family(theme::MONO_FONT)
                        .truncate()
                        .child(row.meta),
                ),
        )
        .when(row.marks.show_review_badge, |line| {
            line.child(Pill::new("Review", PillStyle::Accent))
        })
        .when(row.marks.has_brief, |line| line.child(render_brief_mark()))
        .children(
            row.marks
                .viewer_review
                .map(|review| ReviewVerdictBadge::new(row.is_listed.then_some(tab), review)),
        )
        .into_any_element()
}

/// ` · merged`, ` · closed` or ` · draft` for a layer not simply open.
fn layer_state_marker(layer: &StackLayer) -> &'static str {
    match (layer.is_merged, layer.is_open, layer.is_draft) {
        (true, _, _) => " · merged",
        (false, false, _) => " · closed",
        (false, true, true) => " · draft",
        (false, true, false) => "",
    }
}
