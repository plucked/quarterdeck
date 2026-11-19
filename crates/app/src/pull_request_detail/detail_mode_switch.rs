//! `DetailModeSwitch`: the shown pull request's view tabs, with a stack's
//! Layer / Combined choice inside the selected diff tab, for the window's
//! top bar.

use gpui::{App, Entity, IntoElement, RenderOnce, WeakEntity, Window, div, prelude::*};
use qd_cache::StackReviewMode;

use super::detail_mode::DetailMode;
use super::pull_request_detail::PullRequestDetail;
use super::review_target::ReviewTarget;
use crate::controls::{SegmentedControl, TextTooltip, view_option};

/// Summary / Diff / Guided review / Brief for the pull request a
/// [`PullRequestDetail`] shows, centered in the space the switch is given.
/// For a stacked pull request the selected Diff or Guided review tab extends
/// to hold Layer / Combined as a small switch after its label.
#[derive(IntoElement)]
pub struct DetailModeSwitch {
    detail: Entity<PullRequestDetail>,
}

impl DetailModeSwitch {
    /// The switch of the pull request `detail` shows.
    pub fn new(detail: Entity<PullRequestDetail>) -> Self {
        Self { detail }
    }
}

impl RenderOnce for DetailModeSwitch {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let handle = self.detail.downgrade();
        let detail = self.detail.read(cx);
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(mode_tabs(detail, &handle))
    }
}

/// The four views of the main column, the shown one selected and, on a diff
/// view of a stacked pull request, holding the stack mode choice.
fn mode_tabs(
    detail: &PullRequestDetail,
    handle: &WeakEntity<PullRequestDetail>,
) -> SegmentedControl {
    let shown = detail.brief().mode;
    let pick = |mode| view_option(handle, mode, PullRequestDetail::select_detail_mode);
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
    match stack_mode_choice(detail, handle) {
        Some(choice) => tabs.selected_detail(choice),
        None => tabs,
    }
}

/// Layer / Combined as a small switch for a stacked pull request while a
/// diff view shows, `None` otherwise. Its tooltip holds the range, loading and failure
/// details; layer navigation belongs to the sidebar.
fn stack_mode_choice(
    detail: &PullRequestDetail,
    handle: &WeakEntity<PullRequestDetail>,
) -> Option<impl IntoElement + use<>> {
    let is_stacked = detail
        .pull_request()
        .is_some_and(|pull_request| pull_request.stack.is_some());
    if !is_stacked || !detail.brief().mode.shows_diff() {
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
