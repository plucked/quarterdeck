use std::ops::Range;

use gpui::{AnyElement, Context, FontWeight, Styled, div, prelude::*, px, uniform_list};
use qc_github::PullRequest;

use super::ReviewQueue;
use super::refresh_status::RefreshStatus;
use crate::theme;

const SIDEBAR_WIDTH: f32 = 380.;

impl ReviewQueue {
    /// The left column: a header with count and refresh state, then the queue.
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
                uniform_list(
                    "pull-requests",
                    self.pull_requests().len(),
                    cx.processor(|queue, range, _window, cx| queue.render_queue_rows(range, cx)),
                )
                .flex_1(),
            )
    }

    /// Title with the queue length, the refresh button and the refresh state.
    fn render_sidebar_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status_line = match self.refresh_status() {
            RefreshStatus::Idle => None,
            RefreshStatus::Refreshing => {
                Some(div().text_color(theme::muted()).child("Refreshing…"))
            }
            RefreshStatus::Failed(message) => {
                Some(div().text_color(theme::error()).child(message.clone()))
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
                    .justify_between()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("Review requested ({})", self.pull_requests().len())),
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
            .children(status_line.map(Styled::text_xs))
    }

    /// The visible rows of the queue list; `range` is what `uniform_list` asks for.
    fn render_queue_rows(&self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        range
            .filter_map(|index| {
                let pull_request = self.pull_requests().get(index)?;
                let is_selected = self.selected_index() == Some(index);
                Some(render_queue_row(pull_request, index, is_selected, cx))
            })
            .collect()
    }
}

/// One queue row: title, then `owner/repo#number · author`; clicking selects it.
fn render_queue_row(
    pull_request: &PullRequest,
    index: usize,
    is_selected: bool,
    cx: &mut Context<ReviewQueue>,
) -> AnyElement {
    let draft_marker = if pull_request.draft { " · draft" } else { "" };
    div()
        .id(index)
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(theme::border())
        .cursor_pointer()
        .when(is_selected, |row| row.bg(theme::selected()))
        .when(!is_selected, |row| {
            row.hover(|style| style.bg(theme::hover()))
        })
        .on_click(cx.listener(move |queue, _, _, cx| queue.select(index, cx)))
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
