use gpui::{AnyElement, Context, FontWeight, IntoElement, Styled, div, prelude::*, px};
use qc_github::PullRequest;

use super::pull_request_detail::PullRequestDetail;
use crate::button::Button;
use crate::display_date::short_date;
use crate::markdown_view::MarkdownView;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

/// The tallest the expanded description gets before it scrolls on its own,
/// so the diff always keeps most of the pane.
const DESCRIPTION_MAX_HEIGHT: f32 = 320.;

impl PullRequestDetail {
    /// The header that stays above the diff: chevron, title, links, what is
    /// loading and the files toggle; a line of metadata and progress; the
    /// stack line of a stacked pull request; the description while
    /// expanded; and a progress line along the bottom while loading. `None`
    /// without a pull request.
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
                    .text_xs()
                    .text_color(theme::subtle())
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
            .children(self.render_activity_pill())
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(Button::new("open-on-github", "GitHub ↗").ghost().on_click(
                        move |_, _, cx| {
                            cx.open_url(&url);
                        },
                    ))
                    .children(self.render_action_buttons(cx)),
            );

        // The padded content, then the progress line flush with the bottom edge.
        let content = div()
            .flex()
            .flex_col()
            .gap_1p5()
            .px_4()
            .pt_3()
            .pb_2p5()
            .child(title_row)
            .child(self.render_meta_line(pull_request))
            .children(self.render_stack_bar(cx))
            .children(self.render_action_panel(cx))
            .when(is_expanded, |header| {
                header.child(self.render_description_box())
            });
        Some(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .bg(theme::panel())
                .border_b_1()
                .border_color(theme::border())
                .child(content)
                .children(self.render_progress_line())
                .into_any_element(),
        )
    }

    /// The metadata under the title: draft, where, who, when, and how far
    /// the review is.
    fn render_meta_line(&self, pull_request: &PullRequest) -> impl IntoElement {
        div()
            .pl_5()
            .flex()
            .items_center()
            .gap_2()
            .text_xs()
            .text_color(theme::muted())
            .when(pull_request.draft, |meta| {
                meta.child(
                    div()
                        .flex_none()
                        .px_1p5()
                        .rounded_full()
                        .bg(theme::surface())
                        .text_color(theme::muted())
                        .child("Draft"),
                )
            })
            .child(div().min_w_0().truncate().child(format!(
                "{} #{}  ·  {}  ·  updated {}{}",
                pull_request.repo.full_name(),
                pull_request.number,
                pull_request.author.login,
                short_date(&pull_request.updated_at),
                self.viewed_progress_text()
            )))
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
        format!("  ·  {viewed}/{} viewed", files.len())
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
