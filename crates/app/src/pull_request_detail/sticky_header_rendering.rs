use gpui::{AnyElement, Context, FontWeight, IntoElement, Styled, div, prelude::*, px};

use super::pull_request_detail::PullRequestDetail;
use crate::markdown_view::MarkdownView;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

/// The tallest the expanded description gets before it scrolls on its own,
/// so the diff always keeps most of the pane.
const DESCRIPTION_MAX_HEIGHT: f32 = 320.;

impl PullRequestDetail {
    /// The header that stays above the diff: chevron, title, links and the
    /// files toggle, a line of metadata and progress, and the description
    /// while expanded. `None` without a pull request.
    pub(super) fn render_sticky_header(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pull_request = self.pull_request()?;
        let has_description = self.description().is_some();
        let is_expanded = self.is_description_expanded() && has_description;
        let chevron = match (has_description, is_expanded) {
            (false, _) => " ",
            (true, false) => "▸",
            (true, true) => "▾",
        };
        let url = pull_request.html_url.clone();

        // Title row: clicking it (or the chevron) shows or hides the description.
        let title_row = div()
            .id("pull-request-title")
            .flex()
            .items_center()
            .gap_2()
            .when(has_description, |row| {
                row.cursor_pointer()
                    .on_click(cx.listener(|detail, _, _, cx| detail.toggle_description(cx)))
            })
            .child(
                div()
                    .flex_none()
                    .w_3()
                    .text_color(theme::muted())
                    .child(chevron),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .when(!is_expanded, Styled::truncate)
                    .child(pull_request.title.clone()),
            )
            .child(
                div()
                    .id("open-on-github")
                    .flex_none()
                    .text_sm()
                    .text_color(theme::accent())
                    .cursor_pointer()
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        cx.open_url(&url);
                    })
                    .child("Open on GitHub ↗"),
            )
            .child(Self::render_tree_toggle(cx));

        let meta = div()
            .pl_5()
            .text_xs()
            .text_color(theme::muted())
            .truncate()
            .child(format!(
                "{}#{} by {} · updated {}{}",
                pull_request.repo.full_name(),
                pull_request.number,
                pull_request.author.login,
                pull_request.updated_at,
                self.viewed_progress_text()
            ));

        Some(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .gap_1()
                .px_4()
                .py_2()
                .bg(theme::panel())
                .border_b_1()
                .border_color(theme::border())
                .child(title_row)
                .child(meta)
                .when(is_expanded, |header| {
                    header.child(self.render_description_box())
                })
                .into_any_element(),
        )
    }

    /// The button that shows or hides the file tree.
    fn render_tree_toggle(cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("toggle-file-tree")
            .flex_none()
            .px_2()
            .rounded_md()
            .border_1()
            .border_color(theme::border())
            .text_xs()
            .text_color(theme::muted())
            .cursor_pointer()
            .hover(|style| style.bg(theme::hover()))
            .on_click(cx.listener(|detail, _, _, cx| {
                cx.stop_propagation();
                detail.toggle_file_tree(cx);
            }))
            .child("Files ⌘B")
    }

    /// ` · 3/12 viewed` once files are loaded.
    fn viewed_progress_text(&self) -> String {
        let files = self.loaded_files();
        if files.is_empty() {
            return String::new();
        }
        let viewed = files
            .iter()
            .filter(|loaded| self.file_review().is_viewed(loaded.filename()))
            .count();
        format!(" · {viewed}/{} viewed", files.len())
    }

    /// The description, scrolling inside the header past its maximum height.
    fn render_description_box(&self) -> AnyElement {
        let Some(description) = self.description() else {
            return div().into_any_element();
        };
        div()
            .relative()
            .mt_1()
            .ml_5()
            .child(
                div()
                    .id("description")
                    .track_scroll(self.description_scroll())
                    .max_h(px(DESCRIPTION_MAX_HEIGHT))
                    .overflow_y_scroll()
                    .p_3()
                    .rounded_md()
                    .bg(theme::background())
                    .text_sm()
                    .child(MarkdownView::new("description", description.clone())),
            )
            .child(VerticalScrollbar::new(
                "description-scrollbar",
                ScrollbarTarget::Handle(self.description_scroll().clone()),
            ))
            .into_any_element()
    }
}
