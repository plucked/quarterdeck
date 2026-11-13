use gpui::{AnyElement, div, prelude::*, px};

use super::pull_request_detail::PullRequestDetail;
use crate::markdown_view::MarkdownView;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

/// Keep prose readable while giving tables and code enough room.
const SUMMARY_MAX_WIDTH: f32 = 880.;

impl PullRequestDetail {
    /// The pull request's own description, with its scroll position kept
    /// independently of the diff and brief. Empty descriptions keep the tab.
    pub(super) fn render_summary_area(&self) -> AnyElement {
        let content = match self.description() {
            Some(description) => {
                MarkdownView::new("pull-request-summary", description.clone()).into_any_element()
            }
            None => div()
                .text_color(theme::muted())
                .child(if self.pull_request().is_some() {
                    "No summary provided for this pull request."
                } else {
                    "Select a pull request"
                })
                .into_any_element(),
        };

        div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(
                div()
                    .id("summary-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(self.summary_scroll())
                    .child(
                        div()
                            .w_full()
                            .max_w(px(SUMMARY_MAX_WIDTH))
                            .mx_auto()
                            .p_6()
                            .text_sm()
                            .child(content),
                    ),
            )
            .child(VerticalScrollbar::new(
                "summary-scrollbar",
                ScrollbarTarget::Handle(self.summary_scroll().clone()),
            ))
            .into_any_element()
    }
}
