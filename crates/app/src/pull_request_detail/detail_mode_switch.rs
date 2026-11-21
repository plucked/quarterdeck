//! The pull request's view tabs, with Layer / Combined inside the selected
//! diff tab for a stack.

use gpui::{Context, IntoElement, WeakEntity, div, prelude::*};
use qd_cache::StackReviewMode;

use super::detail_mode::DetailMode;
use super::pull_request_detail::PullRequestDetail;
use super::review_target::ReviewTarget;
use crate::controls::{SegmentedControl, TextTooltip, view_option};

impl PullRequestDetail {
    /// The four views of the main column. For a stack, the selected Diff
    /// or Guided review tab also holds the stack mode choice.
    pub(super) fn render_mode_tabs(&self, cx: &Context<Self>) -> SegmentedControl {
        let handle = cx.entity().downgrade();
        let shown = self.brief().mode;
        let pick = |mode| view_option(&handle, mode, Self::select_detail_mode);
        let tabs = SegmentedControl::new("detail-mode")
            .large()
            .quiet()
            .option(
                "Summary",
                shown == DetailMode::Summary,
                pick(DetailMode::Summary),
            )
            .option("Diff", shown == DetailMode::Diff, pick(DetailMode::Diff))
            .option(
                "Guided review",
                shown == DetailMode::GuidedReview,
                pick(DetailMode::GuidedReview),
            )
            .option("Brief", shown == DetailMode::Brief, pick(DetailMode::Brief));
        match stack_mode_choice(self, &handle) {
            Some(choice) => tabs.selected_detail(choice),
            None => tabs,
        }
    }
}

/// Layer / Combined as a small switch for a stacked pull request while a
/// diff view shows, `None` otherwise. Its tooltip holds the range, loading and failure
/// details; layer navigation belongs to the sidebar.
fn stack_mode_choice(
    detail: &PullRequestDetail,
    handle: &WeakEntity<PullRequestDetail>,
) -> Option<impl IntoElement + use<>> {
    if !detail.has_review_stack() || !detail.brief().mode.shows_diff() {
        return None;
    }
    let mode = detail.review_mode();
    let pick = |mode| view_option(handle, mode, PullRequestDetail::set_review_mode);
    let hint = match (detail.target(), detail.stack_error()) {
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
    Some(
        div()
            .id("stack-review-context")
            .flex_none()
            .tooltip(TextTooltip::builder(hint))
            .child(
                SegmentedControl::new("stack-review-mode")
                    .inset()
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
            ),
    )
}
