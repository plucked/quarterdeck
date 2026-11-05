use gpui::{AnyElement, Context, FontWeight, IntoElement, SharedString, div, prelude::*};
use qc_cache::StackReviewMode;

use super::pull_request_detail::PullRequestDetail;
use super::review_target::ReviewTarget;
use crate::button::Button;
use crate::segmented_control::SegmentedControl;
use crate::theme;

impl PullRequestDetail {
    /// The header's stack line for a stacked pull request: which layer this
    /// is, buttons to the layers below and above, the review mode, and in a
    /// combined review the range it covers. `None` for a standalone one.
    pub(super) fn render_stack_bar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pull_request = self.pull_request()?;
        let membership = pull_request.stack.as_ref()?;
        let current = self.current_layer_index();
        let layers = self.stack().map(|stack| &stack.layers);

        // The neighbours, once the layers are known.
        let below = current
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| Some((index, layers?.get(index)?.number)));
        let above = current.and_then(|index| Some((index + 1, layers?.get(index + 1)?.number)));
        let position = current.map_or(membership.position, |index| {
            u64::try_from(index + 1).unwrap_or(u64::MAX)
        });
        let size = layers.map_or(membership.size, |layers| {
            u64::try_from(layers.len()).unwrap_or(u64::MAX)
        });

        let mode = self.review_mode();
        let range_text = self.render_range_text();
        Some(
            div()
                .flex()
                .items_center()
                .gap_2()
                .pl_5()
                .text_xs()
                .child(
                    div()
                        .flex_none()
                        .px_2()
                        .py_0p5()
                        .rounded_full()
                        .bg(theme::selected())
                        .text_color(theme::accent())
                        .font_weight(FontWeight::MEDIUM)
                        .child(format!("Layer {position} of {size}")),
                )
                .child(
                    div()
                        .flex_none()
                        .text_color(theme::subtle())
                        .child(format!("→ {}", membership.base_ref)),
                )
                .child(render_neighbour_button(
                    "layer-below",
                    below.map(|(_, number)| format!("‹ #{number}").into()),
                    "‹ bottom",
                    cx.listener(|detail, _, _, cx| detail.open_previous_layer(cx)),
                ))
                .child(render_neighbour_button(
                    "layer-above",
                    above.map(|(_, number)| format!("#{number} ›").into()),
                    "top ›",
                    cx.listener(|detail, _, _, cx| detail.open_next_layer(cx)),
                ))
                .child(
                    SegmentedControl::new("stack-review-mode")
                        .option("Layer", mode == StackReviewMode::Layer, {
                            let detail = cx.entity().downgrade();
                            move |_, cx| {
                                detail
                                    .update(cx, |detail, cx| {
                                        detail.set_review_mode(StackReviewMode::Layer, cx);
                                    })
                                    .ok();
                            }
                        })
                        .option("Combined", mode == StackReviewMode::Combined, {
                            let detail = cx.entity().downgrade();
                            move |_, cx| {
                                detail
                                    .update(cx, |detail, cx| {
                                        detail.set_review_mode(StackReviewMode::Combined, cx);
                                    })
                                    .ok();
                            }
                        }),
                )
                .children(range_text)
                .into_any_element(),
        )
    }

    /// What a combined review covers, or why it waits; `None` in layer mode.
    fn render_range_text(&self) -> Option<AnyElement> {
        let text: SharedString = match self.target() {
            ReviewTarget::Layer => match (self.review_mode(), self.stack_error()) {
                (StackReviewMode::Combined, Some(error)) => {
                    format!("Combined review unavailable: {error}").into()
                }
                (StackReviewMode::Layer | StackReviewMode::Combined, _) => return None,
            },
            ReviewTarget::AwaitingStack => "Waiting for the stack's layers…".into(),
            ReviewTarget::Combined(range) => {
                let count = range.layer_count();
                let plural = if count == 1 { "" } else { "s" };
                format!(
                    "Layers {}–{} · {count} pull request{plural} · #{} to #{} · ⇧-click a layer to move the start",
                    range.first_index + 1,
                    range.last_index + 1,
                    range.first_number,
                    range.last_number
                )
                .into()
            }
        };
        Some(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_color(theme::subtle())
                .child(text)
                .into_any_element(),
        )
    }
}

/// A button to the layer below or above; disabled at the stack's end (or
/// before the layers are known).
fn render_neighbour_button(
    id: &'static str,
    label: Option<SharedString>,
    end_label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> Button {
    match label {
        Some(label) => Button::new(id, label).on_click(on_click),
        None => Button::new(id, end_label).disabled(true),
    }
}
