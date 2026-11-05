use std::ops::Range;

use gpui::{AnyElement, Context, FontWeight, IntoElement, Rgba, div, prelude::*, px, uniform_list};
use qc_github::StackLayer;

use super::pull_request_detail::PullRequestDetail;
use super::review_target::ReviewTarget;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

/// Height of one layer row; the map is a uniform list.
const LAYER_ROW_HEIGHT: f32 = 24.;
/// Rows shown before the map scrolls, so the file tree keeps most of the panel.
const VISIBLE_LAYER_ROWS: usize = 8;
/// Width of the rail drawn down the left of the map.
const RAIL_WIDTH: f32 = 2.;

/// How one layer row shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LayerRowRole {
    /// The shown pull request.
    Current,
    /// Inside the combined range, not the shown one.
    InRange,
    Other,
}

impl PullRequestDetail {
    /// The stack map at the top of the side panel: every layer bottom
    /// first, the shown one highlighted and a combined range marked. A click
    /// opens a layer; a shift-click starts the combined range there.
    /// `None` until the stack's layers are known.
    pub(super) fn render_stack_map(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let stack = self.stack()?;
        let layer_count = stack.layers.len();
        let visible_rows = layer_count.min(VISIBLE_LAYER_ROWS);
        let list_height =
            LAYER_ROW_HEIGHT * f32::from(u16::try_from(visible_rows).unwrap_or(u16::MAX));
        Some(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .border_b_1()
                .border_color(theme::border())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .pt_3()
                        .pb_1p5()
                        .child(
                            div()
                                .flex_1()
                                .text_xs()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!("Stack #{}", stack.number)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::subtle())
                                .child(format!("{layer_count} layers → {}", stack.base_ref)),
                        ),
                )
                .child(
                    div()
                        .relative()
                        .h(px(list_height))
                        .mb_2()
                        .child(
                            uniform_list(
                                "stack-map",
                                layer_count,
                                cx.processor(|detail, range, _window, cx| {
                                    detail.render_layer_rows(range, cx)
                                }),
                            )
                            .track_scroll(self.stack_map_scroll().clone())
                            .size_full(),
                        )
                        .child(VerticalScrollbar::new(
                            "stack-map-scrollbar",
                            ScrollbarTarget::Handle(
                                self.stack_map_scroll().0.borrow().base_handle.clone(),
                            ),
                        )),
                )
                .into_any_element(),
        )
    }

    /// The visible layer rows in `range`.
    fn render_layer_rows(&self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(stack) = self.stack() else {
            return Vec::new();
        };
        let current = self.current_layer_index();
        let combined_range = match self.target() {
            ReviewTarget::Combined(range) => Some(range.first_index..=range.last_index),
            ReviewTarget::Layer | ReviewTarget::AwaitingStack => None,
        };
        range
            .filter_map(|index| {
                let layer = stack.layers.get(index)?;
                let role = match (current == Some(index), &combined_range) {
                    (true, _) => LayerRowRole::Current,
                    (false, Some(range)) if range.contains(&index) => LayerRowRole::InRange,
                    (false, _) => LayerRowRole::Other,
                };
                Some(render_layer_row(index, layer, role, cx))
            })
            .collect()
    }
}

/// One layer: the rail, its position, state, number and title. Clicking
/// opens it; shift-clicking starts the combined range there.
fn render_layer_row(
    index: usize,
    layer: &StackLayer,
    role: LayerRowRole,
    cx: &mut Context<PullRequestDetail>,
) -> AnyElement {
    let (state_glyph, state_color) = layer_state(layer);
    let rail_color = match role {
        LayerRowRole::Current | LayerRowRole::InRange => theme::accent(),
        LayerRowRole::Other => theme::border_strong(),
    };
    div()
        .id(("stack-layer", index))
        .flex()
        .items_center()
        .gap_2()
        .h(px(LAYER_ROW_HEIGHT))
        .mx_2()
        .px_1p5()
        .rounded_md()
        .text_sm()
        .cursor_pointer()
        .when(role == LayerRowRole::Current, |row| {
            row.bg(theme::selected()).font_weight(FontWeight::MEDIUM)
        })
        .when(role != LayerRowRole::Current, |row| {
            row.hover(|style| style.bg(theme::hover()))
        })
        .on_click(cx.listener(move |detail, event: &gpui::ClickEvent, _, cx| {
            if event.modifiers().shift {
                detail.set_combined_start(index, cx);
            } else {
                detail.open_layer(index, cx);
            }
        }))
        .child(
            div()
                .flex_none()
                .w(px(RAIL_WIDTH))
                .h(px(LAYER_ROW_HEIGHT - 8.))
                .rounded_full()
                .bg(rail_color),
        )
        .child(
            div()
                .flex_none()
                .w_4()
                .text_xs()
                .text_color(theme::subtle())
                .child(format!("{}", index + 1)),
        )
        .child(
            div()
                .flex_none()
                .w_3()
                .text_xs()
                .text_color(state_color)
                .child(state_glyph),
        )
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(theme::subtle())
                .child(format!("#{}", layer.number)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_color(match (role, layer.is_merged) {
                    (LayerRowRole::Current, _) => theme::text(),
                    (_, true) => theme::subtle(),
                    (_, false) => theme::muted(),
                })
                .child(layer.title.clone()),
        )
        .into_any_element()
}

/// A layer's state as a glyph and color: merged, closed, draft or open.
fn layer_state(layer: &StackLayer) -> (&'static str, Rgba) {
    match (layer.is_merged, layer.is_open, layer.is_draft) {
        (true, _, _) => ("✓", theme::renamed()),
        (false, false, _) => ("✕", theme::removed()),
        (false, true, true) => ("◌", theme::muted()),
        (false, true, false) => ("●", theme::added()),
    }
}
