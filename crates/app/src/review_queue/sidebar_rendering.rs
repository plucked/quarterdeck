use std::ops::Range;
use std::time::Duration;

use gpui::{
    Animation, AnimationExt as _, AnyElement, Context, FontWeight, SharedString, Styled, div,
    prelude::*, pulsating_between, px, uniform_list,
};
use qc_cache::QueueTab;
use qc_github::PullRequest;

use super::ReviewQueue;
use super::queue_row::{QueueRow, StackGroup};
use super::refresh_status::RefreshStatus;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::segmented_control::SegmentedControl;
use crate::theme;

const SIDEBAR_WIDTH: f32 = 380.;
/// The breath of the pulsing "Refreshing…" status.
const PULSE_PERIOD: Duration = Duration::from_millis(1200);
/// Width of the column holding a stack layer's position and rail.
const LAYER_GUTTER_WIDTH: f32 = 28.;

impl ReviewQueue {
    /// The left column: the tabs, refresh button and state, then the rows.
    pub(super) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .flex_shrink_0()
            .bg(theme::panel())
            .border_r_1()
            .border_color(theme::border())
            .child(self.render_sidebar_header(cx))
            .child(
                // The list fills the rest of the column, with a scrollbar over its right edge.
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        uniform_list(
                            "pull-requests",
                            self.rows().len(),
                            cx.processor(|queue, range, _window, cx| {
                                queue.render_queue_rows(range, cx)
                            }),
                        )
                        .track_scroll(self.queue_list_scroll().clone())
                        .size_full(),
                    )
                    .child(VerticalScrollbar::new(
                        "pull-requests-scrollbar",
                        ScrollbarTarget::Handle(
                            self.queue_list_scroll().0.borrow().base_handle.clone(),
                        ),
                    )),
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
            .gap_1()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(theme::border())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        SegmentedControl::new("queue-tabs")
                            .option(
                                format!("Mine ({authored})  ⌘1"),
                                tab == QueueTab::Authored,
                                show_tab(QueueTab::Authored),
                            )
                            .option(
                                format!("Review ({review_requested})  ⌘2"),
                                tab == QueueTab::ReviewRequested,
                                show_tab(QueueTab::ReviewRequested),
                            ),
                    )
                    .child(
                        div()
                            .id("refresh")
                            .text_color(theme::accent())
                            .cursor_pointer()
                            .on_click(cx.listener(|queue, _, _, cx| queue.refresh(cx)))
                            .child("⟳ Refresh"),
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

    /// The visible rows of the list; `range` is what `uniform_list` asks for.
    fn render_queue_rows(&self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let selected_id = self.selected_pull_request().map(|selected| selected.id);
        range
            .filter_map(|index| {
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
                        render_stack_layer_row(
                            pull_request,
                            *list_index,
                            selected_id == Some(pull_request.id),
                            cx,
                        )
                    }
                    QueueRow::StackHeader(group) => render_stack_header(index, group, cx),
                };
                Some(element)
            })
            .collect()
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

/// A row's frame: padding, separator, hover and selection.
fn row_frame(id: impl Into<gpui::ElementId>, is_selected: bool) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme::border())
        .cursor_pointer()
        .when(is_selected, |row| row.bg(theme::selected()))
        .when(!is_selected, |row| {
            row.hover(|style| style.bg(theme::hover()))
        })
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
        .child(div().truncate().child(pull_request.title.clone()))
        .child(
            div()
                .text_xs()
                .text_color(theme::muted())
                .truncate()
                .child(format!(
                    "{}#{} · {}{draft_marker}",
                    pull_request.repo.full_name(),
                    pull_request.number,
                    pull_request.author.login,
                )),
        )
        .into_any_element()
}

/// The header of a stack's group: its number, size and trunk, and how many
/// of its layers this list has; clicking collapses or expands it.
fn render_stack_header(
    row_index: usize,
    group: &StackGroup,
    cx: &mut Context<ReviewQueue>,
) -> AnyElement {
    let chevron = if group.is_collapsed { "▸" } else { "▾" };
    let key = group.key.clone();
    let listed = if u64::try_from(group.listed).is_ok_and(|listed| listed == group.size) {
        "all layers listed".to_owned()
    } else {
        format!("{} of {} layers listed", group.listed, group.size)
    };
    div()
        .id(("stack-header", row_index))
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme::border())
        .bg(theme::background())
        .cursor_pointer()
        .hover(|style| style.bg(theme::hover()))
        .on_click(cx.listener(move |queue, _, _, cx| queue.toggle_stack_group(&key, cx)))
        .child(
            div()
                .flex()
                .gap_1p5()
                .font_weight(FontWeight::SEMIBOLD)
                .child(div().w_3().text_color(theme::muted()).child(chevron))
                .child(div().text_color(theme::accent()).child("⧉"))
                .child(div().truncate().child(format!(
                    "Stack #{} · {} layers → {}",
                    group.key.number, group.size, group.base_ref
                ))),
        )
        .child(
            div()
                .pl_4()
                .text_xs()
                .text_color(theme::muted())
                .truncate()
                .child(format!("{} · {listed}", group.key.repo.full_name())),
        )
        .into_any_element()
}

/// One listed layer of a stack, under its header: the layer's position on
/// a rail, then title and `#number · author`; clicking selects it.
fn render_stack_layer_row(
    pull_request: &PullRequest,
    list_index: usize,
    is_selected: bool,
    cx: &mut Context<ReviewQueue>,
) -> AnyElement {
    let draft_marker = if pull_request.draft { " · draft" } else { "" };
    let position = pull_request
        .stack
        .as_ref()
        .map_or(String::new(), |membership| membership.position.to_string());
    row_frame(("stack-layer", list_index), is_selected)
        .pl_2()
        .flex()
        .gap_2()
        .on_click(cx.listener(move |queue, _, _, cx| queue.select(list_index, cx)))
        .child(
            div()
                .flex_none()
                .w(px(LAYER_GUTTER_WIDTH))
                .flex()
                .items_center()
                .justify_end()
                .gap_1()
                .text_xs()
                .text_color(theme::muted())
                .child(position)
                .child(div().w(px(2.)).h_full().bg(theme::accent())),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(div().truncate().child(pull_request.title.clone()))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme::muted())
                        .truncate()
                        .child(format!(
                            "#{} · {}{draft_marker}",
                            pull_request.number, pull_request.author.login,
                        )),
                ),
        )
        .into_any_element()
}
