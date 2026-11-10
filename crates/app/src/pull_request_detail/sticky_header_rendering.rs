use gpui::{AnyElement, Context, FontWeight, IntoElement, Styled, div, prelude::*, px};
use qd_github::PullRequest;

use super::detail_mode::DetailMode;
use super::pull_request_detail::PullRequestDetail;
use crate::avatar_images::Avatar;
use crate::controls::{FOLD_CHEVRON_SIZE, FoldChevron, FoldState, SegmentedControl, view_option};
use crate::display_date::short_date;
use crate::github_link::GitHubLink;
use crate::markdown_view::MarkdownView;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

/// The tallest the expanded description gets before it scrolls on its own,
/// so the diff always keeps most of the pane.
const DESCRIPTION_MAX_HEIGHT: f32 = 320.;

/// The author's avatar in the header's meta line.
const HEADER_AVATAR_SIZE: f32 = 16.;

impl PullRequestDetail {
    /// The header that stays above the diff: chevron, title with GitHub's
    /// mark linking to the pull request, what is loading and the Diff / Brief
    /// switch; a line of metadata; the stack line of a stacked pull request;
    /// the description while expanded; and a progress line along the bottom
    /// while loading. `None` without a pull request. Merging and closing are
    /// in the side panel's PR Actions.
    pub(super) fn render_sticky_header(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pull_request = self.pull_request()?;
        let has_description = self.description().is_some();
        let is_expanded = self.is_description_expanded() && has_description;
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
            // The chevron's slot stays without a description, so the title keeps its place.
            .child(
                div()
                    .flex_none()
                    .w(px(FOLD_CHEVRON_SIZE))
                    .when(has_description, |slot| {
                        slot.child(FoldChevron::new(
                            "description-fold",
                            FoldState::of(!is_expanded),
                        ))
                    }),
            )
            // The title, GitHub's mark right after it (both shrink to the
            // title's length), then the room left before the switch.
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .min_w_0()
                            .text_lg()
                            .font_weight(FontWeight::SEMIBOLD)
                            .when(!is_expanded, Styled::truncate)
                            .child(pull_request.title.clone()),
                    )
                    .child(GitHubLink::new("open-on-github", url)),
            )
            .children(self.render_activity_pill())
            .child(self.render_mode_switch(cx));

        // The padded content, then the progress line flush with the bottom edge.
        let content = div()
            .flex()
            .flex_col()
            .gap_1p5()
            .px_4()
            .pt_3()
            .pb_2p5()
            .child(title_row)
            .child(Self::render_meta_line(pull_request))
            .children(self.render_stack_bar(cx))
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

    /// Diff or Brief: what the main column shows.
    fn render_mode_switch(&self, cx: &mut Context<Self>) -> AnyElement {
        let mode = self.brief().mode;
        let detail = cx.entity().downgrade();
        let pick = |mode| view_option(&detail, mode, PullRequestDetail::set_detail_mode);
        SegmentedControl::new("detail-mode")
            .large()
            .option("Diff", mode == DetailMode::Diff, pick(DetailMode::Diff))
            .option("Brief", mode == DetailMode::Brief, pick(DetailMode::Brief))
            .into_any_element()
    }

    /// The metadata under the title: draft, where, who and when. How far
    /// the review is shows in the side panel's file tree.
    fn render_meta_line(pull_request: &PullRequest) -> impl IntoElement {
        div()
            .pl_8()
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
                "{} #{}  ·",
                pull_request.repo.full_name(),
                pull_request.number,
            )))
            .child(Avatar::new(
                pull_request.author.avatar_url.clone(),
                px(HEADER_AVATAR_SIZE),
            ))
            .child(div().min_w_0().truncate().child(format!(
                "{}  ·  updated {}",
                pull_request.author.login,
                short_date(&pull_request.updated_at),
            )))
    }

    /// The description, scrolling inside the header past its maximum height.
    fn render_description_box(&self) -> AnyElement {
        let Some(description) = self.description() else {
            return div().into_any_element();
        };
        div()
            .relative()
            .mt_1()
            .ml_8()
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
