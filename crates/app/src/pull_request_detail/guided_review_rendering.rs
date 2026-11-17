use gpui::{AnyElement, Context, IntoElement, div, prelude::*, px};

use super::pull_request_detail::PullRequestDetail;
use crate::theme;

impl PullRequestDetail {
    /// One guided surface for empty state, source context, single files and grouped steps.
    pub(super) fn render_guided_review_area(&mut self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(preview) = &self.focus_drawer.preview {
            return self.render_search_preview(preview, cx);
        }
        if let Some(id) = self.walkthrough.selected_group
            && self.walkthrough.item(id).is_some()
        {
            return self.render_group_overview(id, cx);
        }
        if self.loaded_files().is_empty() {
            return self.render_diff_area(cx);
        }
        if self
            .current_smart_filter()
            .is_some_and(|state| state.filter.is_some())
        {
            return div().flex_1().min_w_0().p_6().flex().flex_col().gap_3()
                .child(div().text_lg().child("Choose a review step"))
                .child(div().text_sm().text_color(theme::muted())
                    .child("Select a step in the Guide panel. If none are visible, adjust the search or filters to show them."))
                .into_any_element();
        }
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .justify_center()
            .p_4()
            .child(
                div()
                    .w_full()
                    .max_w(px(640.))
                    .flex()
                    .flex_col()
                    .child(self.render_walkthrough(cx)),
            )
            .into_any_element()
    }

    /// The navigator waits for authored steps while creation and repair status occupy the main pane.
    pub(super) fn render_guided_navigation_placeholder() -> AnyElement {
        div()
            .flex_1()
            .p_4()
            .text_sm()
            .text_color(theme::muted())
            .child("Your review steps will appear here, in the suggested reading order.")
            .into_any_element()
    }
}
