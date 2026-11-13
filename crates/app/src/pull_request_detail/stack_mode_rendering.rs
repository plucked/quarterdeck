use gpui::{AnyElement, Context, div, prelude::*};
use qd_cache::StackReviewMode;

use super::detail_mode::DetailMode;
use super::pull_request_detail::PullRequestDetail;
use super::review_target::ReviewTarget;
use crate::controls::{SegmentedControl, TextTooltip, view_option};

impl PullRequestDetail {
    /// The second row of the view bubble. Range details and failures remain
    /// available on hover; layer navigation belongs to the sidebar.
    pub(super) fn render_stack_mode_switch(&self, cx: &mut Context<Self>) -> AnyElement {
        let mode = self.review_mode();
        let is_diff = self.brief().mode == DetailMode::Diff;
        let detail = cx.entity().downgrade();
        let pick = |mode| {
            let select = view_option(&detail, mode, PullRequestDetail::set_review_mode);
            move |window: &mut gpui::Window, cx: &mut gpui::App| {
                // A row fading out after a tab change must no longer act.
                if is_diff {
                    select(window, cx);
                }
            }
        };
        let hint = match (self.target(), self.stack_error()) {
            (_, Some(error)) => format!("Combined review unavailable: {error}"),
            (ReviewTarget::AwaitingStack, _) => "Waiting for the stack's layers…".to_owned(),
            (ReviewTarget::Combined(range), _) => format!(
                "Layers {}–{} · #{} to #{} · ⇧-click a layer in the sidebar to move the start",
                range.first_index + 1,
                range.last_index + 1,
                range.first_number,
                range.last_number,
            ),
            (ReviewTarget::Layer, _) => {
                "Review this layer or combine it with the layers below. Shortcut: c".to_owned()
            }
        };
        div()
            .id("stack-review-context")
            .h_full()
            .px_2()
            .pb_1p5()
            .tooltip(TextTooltip::builder(hint))
            .child(
                SegmentedControl::new("stack-review-mode")
                    .quiet()
                    .fill_width()
                    .option(
                        "Layer",
                        mode == StackReviewMode::Layer,
                        pick(StackReviewMode::Layer),
                    )
                    .option(
                        "Combined",
                        mode == StackReviewMode::Combined,
                        pick(StackReviewMode::Combined),
                    ),
            )
            .into_any_element()
    }
}
