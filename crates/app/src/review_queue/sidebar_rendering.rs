use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, Context, FontWeight, SharedString, Styled, div, list,
    prelude::*, pulsating_between, px,
};
use qc_cache::QueueTab;
use qc_github::{PullRequest, StackLayer};

use super::ReviewQueue;
use super::queue_row::{QueueRow, StackGroup};
use super::refresh_status::RefreshStatus;
use crate::button::Button;
use crate::resize_handle::{ResizeEdge, ResizeHandle};
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::segmented_control::SegmentedControl;
use crate::theme;

/// How narrow and how wide the sidebar can be dragged.
const MIN_SIDEBAR_WIDTH: f32 = 240.;
const MAX_SIDEBAR_WIDTH: f32 = 720.;
/// The breath of the pulsing "Refreshing…" status.
const PULSE_PERIOD: Duration = Duration::from_millis(1200);
/// Width of the column holding a stack layer's position and rail.
const LAYER_GUTTER_WIDTH: f32 = 28.;
/// Height of the rail mark beside a layer's position.
const LAYER_RAIL_HEIGHT: f32 = 16.;

impl ReviewQueue {
    /// The left column: the tabs, refresh button and state, then the rows.
    pub(super) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w(px(self.sidebar_width()))
            .h_full()
            .flex_shrink_0()
            .bg(theme::sidebar())
            .child(self.render_sidebar_header(cx))
            .child(
                // The list fills the rest of the column, with a scrollbar over its right edge.
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        list(
                            self.queue_list().clone(),
                            cx.processor(|queue, index, _window, cx| {
                                queue.render_queue_row(index, cx)
                            }),
                        )
                        .size_full(),
                    )
                    .child(VerticalScrollbar::new(
                        "pull-requests-scrollbar",
                        ScrollbarTarget::List(self.queue_list().clone()),
                    )),
            )
    }

    /// The divider right of the sidebar; dragging it resizes the sidebar.
    pub(super) fn render_sidebar_handle(&self, cx: &mut Context<Self>) -> ResizeHandle {
        let queue = cx.entity().downgrade();
        ResizeHandle::new(
            "sidebar-resize",
            ResizeEdge::Right,
            self.sidebar_width(),
            MIN_SIDEBAR_WIDTH,
            MAX_SIDEBAR_WIDTH,
            move |width, _, cx| {
                queue
                    .update(cx, |queue, cx| queue.set_sidebar_width(width, cx))
                    .ok();
            },
        )
    }

    /// The tabs with their counts, the refresh button, and what is going on.
    fn render_sidebar_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = self.tab();
        let authored = self.lists().authored.len();
        let review_requested = self.lists().review_requested.len();
        let queue = cx.entity().downgrade();
        let show_tab = move |tab: QueueTab| {
            let queue = queue.clone();
            move |_: &mut gpui::Window, cx: &mut gpui::App| {
                queue.update(cx, |queue, cx| queue.show_tab(tab, cx)).ok();
            }
        };

        div()
            .flex()
            .flex_col()
            .gap_1p5()
            .px_3()
            .pt_3()
            .pb_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        SegmentedControl::new("queue-tabs")
                            .option(
                                format!("Mine  {authored}"),
                                tab == QueueTab::Authored,
                                show_tab(QueueTab::Authored),
                            )
                            .option(
                                format!("Review  {review_requested}"),
                                tab == QueueTab::ReviewRequested,
                                show_tab(QueueTab::ReviewRequested),
                            ),
                    )
                    .child(
                        Button::new("refresh", "↻ Refresh")
                            .ghost()
                            .on_click(cx.listener(|queue, _, _, cx| queue.refresh(cx))),
                    ),
            )
            .children(self.render_status_line())
    }

    /// The refresh state, an opening stack layer, or why one did not open.
    fn render_status_line(&self) -> Option<AnyElement> {
        if let Some(number) = self.opening_layer() {
            return Some(render_pulsing_status(
                "opening-layer",
                format!("Opening #{number} from GitHub…").into(),
            ));
        }
        if let Some(error) = self.layer_error() {
            return Some(render_error_status(error.clone()));
        }
        match self.refresh_status() {
            RefreshStatus::Idle => None,
            RefreshStatus::Refreshing => Some(render_pulsing_status(
                "refreshing",
                "Refreshing from GitHub…".into(),
            )),
            RefreshStatus::Failed(message) => Some(render_error_status(message.clone())),
        }
    }

    /// Row `index` of the list; an empty element when it is gone.
    fn render_queue_row(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        self.render_known_queue_row(index, cx)
            .unwrap_or_else(|| div().into_any_element())
    }

    /// Row `index`, inset from the sidebar's edges; `None` when the row or
    /// what it shows is gone.
    fn render_known_queue_row(&self, index: usize, cx: &mut Context<Self>) -> Option<AnyElement> {
        let selected_id = self.selected_pull_request().map(|selected| selected.id);
        let row = self.rows().get(index)?;
        let element = match row {
            QueueRow::PullRequest(list_index) => {
                let pull_request = self.shown_list().get(*list_index)?;
                render_pull_request_row(
                    pull_request,
                    *list_index,
                    selected_id == Some(pull_request.id),
                    cx,
                )
            }
            QueueRow::StackLayer(list_index) => {
                let pull_request = self.shown_list().get(*list_index)?;
                let list_index = *list_index;
                let draft_marker = if pull_request.draft { " · draft" } else { "" };
                render_layer_row(
                    LayerRow {
                        id: ("stack-layer", list_index).into(),
                        position: pull_request
                            .stack
                            .as_ref()
                            .map_or(0, |membership| membership.position),
                        title: pull_request.title.clone().into(),
                        meta: format!("#{}{draft_marker}", pull_request.number).into(),
                        is_listed: true,
                        needs_review: self.needs_review(&pull_request.repo, pull_request.number),
                        is_selected: selected_id == Some(pull_request.id),
                    },
                    cx.listener(move |queue, _, _, cx| queue.select(list_index, cx)),
                )
            }
            QueueRow::OtherLayer { key, layer } => {
                let stack_layer = self.stack(key)?.layers.get(*layer)?;
                let (key, layer) = (key.clone(), *layer);
                render_layer_row(
                    LayerRow {
                        id: ("other-layer", index).into(),
                        position: u64::try_from(layer + 1).unwrap_or(u64::MAX),
                        title: stack_layer.title.clone().into(),
                        meta: format!("#{}{}", stack_layer.number, layer_state_marker(stack_layer))
                            .into(),
                        is_listed: false,
                        needs_review: self.needs_review(&key.repo, stack_layer.number),
                        is_selected: self.is_selected(&key.repo, stack_layer.number),
                    },
                    cx.listener(move |queue, _, _, cx| {
                        queue.open_other_layer(&key, layer, cx);
                    }),
                )
            }
            QueueRow::StackHeader(group) => {
                let is_reviewed = self.is_stack_reviewed(&group.key, cx);
                render_stack_header(index, group, self.tab(), is_reviewed, cx)
            }
        };
        // Inset from the sidebar's edges; the row keeps the full width inside.
        Some(div().px_2().pb_0p5().child(element).into_any_element())
    }
}

/// A status in the header that pulses while it lasts.
fn render_pulsing_status(id: &'static str, text: SharedString) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme::accent())
        .child(text)
        .with_animation(
            id,
            Animation::new(PULSE_PERIOD)
                .repeat()
                .with_easing(pulsating_between(0.4, 1.)),
            Styled::opacity,
        )
        .into_any_element()
}

/// A failure in the header.
fn render_error_status(text: SharedString) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme::error())
        .child(text)
        .into_any_element()
}

/// A row's frame (placed inset by the list): rounded corners, a hover tint
/// and the selection.
fn row_frame(id: impl Into<gpui::ElementId>, is_selected: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px_2p5()
        .py_1p5()
        .rounded_md()
        .cursor_pointer()
        .when(is_selected, |row| row.bg(theme::selected()))
        .when(!is_selected, |row| {
            row.hover(|style| style.bg(theme::hover()))
        })
}

/// A row's second line: small and quiet.
fn render_meta(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .text_xs()
        .text_color(theme::subtle())
        .truncate()
        .child(text.into())
}

/// One standalone pull request: title, then `owner/repo#number · author`;
/// clicking selects it.
fn render_pull_request_row(
    pull_request: &PullRequest,
    list_index: usize,
    is_selected: bool,
    cx: &mut Context<ReviewQueue>,
) -> AnyElement {
    let draft_marker = if pull_request.draft { " · draft" } else { "" };
    row_frame(("pull-request", list_index), is_selected)
        .on_click(cx.listener(move |queue, _, _, cx| queue.select(list_index, cx)))
        .child(div().text_sm().truncate().child(pull_request.title.clone()))
        .child(render_meta(format!(
            "{} #{} · {}{draft_marker}",
            pull_request.repo.name, pull_request.number, pull_request.author.login,
        )))
        .into_any_element()
}

/// The header of a stack's group: its number, size and trunk, and how many
/// of its layers this list has. Clicking opens the stack's combined review
/// (highlighted while shown); the chevron collapses or expands the group.
fn render_stack_header(
    row_index: usize,
    group: &StackGroup,
    tab: QueueTab,
    is_reviewed: bool,
    cx: &mut Context<ReviewQueue>,
) -> AnyElement {
    let chevron = if group.is_collapsed { "▸" } else { "▾" };
    let open_key = group.key.clone();
    let toggle_key = group.key.clone();
    let is_all = u64::try_from(group.listed).is_ok_and(|listed| listed == group.size);
    let listed = match (tab, is_all) {
        (QueueTab::Authored, true) => "all layers yours".to_owned(),
        (QueueTab::Authored, false) => format!("{} yours", group.listed),
        (QueueTab::ReviewRequested, true) => "all layers to review".to_owned(),
        (QueueTab::ReviewRequested, false) => format!("{} to review", group.listed),
    };
    row_frame(("stack-header", row_index), is_reviewed)
        .mt_1()
        .on_click(cx.listener(move |queue, _, _, cx| queue.open_stack(&open_key, cx)))
        .child(
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .text_sm()
                .child(
                    div()
                        .id(("stack-chevron", row_index))
                        .flex_none()
                        .w_4()
                        .text_center()
                        .rounded_sm()
                        .text_xs()
                        .text_color(theme::subtle())
                        .hover(|style| style.text_color(theme::text()).bg(theme::hover()))
                        .on_click(cx.listener(move |queue, _, _, cx| {
                            cx.stop_propagation();
                            queue.toggle_stack_group(&toggle_key, cx);
                        }))
                        .child(chevron),
                )
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
                        .text_color(theme::muted())
                        .child(format!("→ {}", group.base_ref)),
                ),
        )
        .child(div().pl(px(22.)).child(render_meta(format!(
            "{} · {} layers · {listed}",
            group.key.repo.name, group.size
        ))))
        .into_any_element()
}

/// What one layer row under a stack's header shows.
struct LayerRow {
    id: gpui::ElementId,
    /// 1 for the layer on the trunk.
    position: u64,
    title: SharedString,
    meta: SharedString,
    /// Whether the shown list has it; other layers show muted.
    is_listed: bool,
    /// Whether it asks for the viewer's review (a badge says so).
    needs_review: bool,
    is_selected: bool,
}

/// One layer of a stack, under its header, on a single line so long stacks
/// stay compact: its position on a rail, the title, a Review badge when it
/// asks for the viewer's review, and `#number` (with its state) on the
/// right. Layers the list lacks are muted; clicking any opens it.
fn render_layer_row(
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
        .when(row.needs_review, |line| line.child(render_review_badge()))
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
fn layer_state_marker(layer: &StackLayer) -> &'static str {
    match (layer.is_merged, layer.is_open, layer.is_draft) {
        (true, _, _) => " · merged",
        (false, false, _) => " · closed",
        (false, true, true) => " · draft",
        (false, true, false) => "",
    }
}
