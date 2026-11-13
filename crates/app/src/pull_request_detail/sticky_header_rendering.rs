use gpui::{AnyElement, Context, FontWeight, IntoElement, Styled, div, prelude::*, px};
use qd_github::{PullRequest, PullRequestState};

use super::detail_mode::DetailMode;
use super::github_link::GitHubLink;
use super::pull_request_detail::PullRequestDetail;
use crate::avatar_images::Avatar;
use crate::controls::{
    AnimatedReveal, NumberFlow, Pill, PillStyle, SegmentedControl, TextTooltip, view_option,
};
use crate::display_text::{short_date, time_ago, unix_now, unix_seconds};
use crate::theme;

/// The author's avatar in the header's meta line.
const HEADER_AVATAR_SIZE: f32 = 20.;
/// The title leads the header without taking space away from the diff.
const HEADER_TITLE_SIZE: f32 = 22.;
/// The stack selector and its bottom inset inside the expanded view bubble.
const STACK_MODE_HEIGHT: f32 = 32.;

impl PullRequestDetail {
    /// Identity on the left, centered view tabs, metadata on the right;
    /// the title on its own row. The view bubble expands for stack diff options.
    /// `None` without a pull request.
    pub(super) fn render_sticky_header(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let pull_request = self.pull_request()?;
        // Equal flexible sides keep the tabs centered even when one side
        // has longer text. Metadata truncates within its own side.
        let toolbar = div()
            .flex()
            .items_start()
            .gap_3()
            .px_6()
            .pt_3()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .pt_1()
                    .child(self.render_identity_line(pull_request)),
            )
            .child(self.render_mode_switch(cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .pt_1p5()
                    .child(Self::render_meta_line(pull_request)),
            );

        let title_row = div()
            .flex()
            .items_center()
            .gap_3()
            .px_6()
            .pt_3()
            .pb_4()
            .child(
                div()
                    .id("pull-request-title")
                    .flex_1()
                    .min_w_0()
                    .text_size(px(HEADER_TITLE_SIZE))
                    .font_weight(FontWeight::MEDIUM)
                    .line_clamp(2)
                    .tooltip(TextTooltip::builder(pull_request.title.clone()))
                    .child(pull_request.title.clone()),
            )
            .children(self.render_activity_pill());

        Some(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .bg(theme::background())
                .border_b_1()
                .border_color(theme::border())
                .child(toolbar)
                .child(title_row)
                .children(self.render_progress_line())
                .into_any_element(),
        )
    }

    /// Summary, Diff or Brief: what the main column shows.
    fn render_mode_switch(&self, cx: &mut Context<Self>) -> AnyElement {
        let mode = self.brief().mode;
        let detail = cx.entity().downgrade();
        let pick = |mode| view_option(&detail, mode, PullRequestDetail::set_detail_mode);
        let tabs = SegmentedControl::new("detail-mode")
            .large()
            .quiet()
            .option(
                "Summary",
                mode == DetailMode::Summary,
                pick(DetailMode::Summary),
            )
            .option("Diff", mode == DetailMode::Diff, pick(DetailMode::Diff))
            .option("Brief", mode == DetailMode::Brief, pick(DetailMode::Brief));
        let is_stacked = self.pull_request().is_some_and(|pr| pr.stack.is_some());
        div()
            .id("detail-mode-bubble")
            .flex_none()
            .flex()
            .flex_col()
            .rounded(px(18.))
            .bg(theme::panel())
            .overflow_hidden()
            .child(tabs)
            .when(is_stacked, |bubble| {
                bubble.child(AnimatedReveal::new(
                    "stack-mode-reveal",
                    mode == DetailMode::Diff,
                    px(STACK_MODE_HEIGHT),
                    self.render_stack_mode_switch(cx),
                ))
            })
            .into_any_element()
    }

    /// Status and repository identify the PR without competing with its title.
    fn render_identity_line(&self, pull_request: &PullRequest) -> impl IntoElement {
        let state = self
            .action_panel()
            .and_then(|panel| panel.options.as_ref())
            .map_or(PullRequestState::Open, |options| options.state);
        let (label, color) = match state {
            PullRequestState::Merged => ("Merged", theme::accent()),
            PullRequestState::Closed => ("Closed", theme::danger()),
            PullRequestState::Open if pull_request.draft => ("Draft", theme::muted()),
            PullRequestState::Open => ("Open", theme::success()),
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .text_xs()
            .text_color(theme::muted())
            .child(
                Pill::new(label, PillStyle::Filled)
                    .roomy()
                    .text_color(color)
                    .mark(div().size_2().rounded_full().border_1().border_color(color)),
            )
            .child(
                div()
                    .id("header-repository")
                    .min_w_0()
                    .truncate()
                    .tooltip(TextTooltip::builder(format!(
                        "{} #{}",
                        pull_request.repo.full_name(),
                        pull_request.number,
                    )))
                    .child(NumberFlow::label(format!(
                        "{} #{}",
                        pull_request.repo.full_name(),
                        pull_request.number,
                    ))),
            )
            .child(GitHubLink::new(
                "open-on-github",
                pull_request.html_url.clone(),
            ))
    }

    /// Compact, right-aligned author, branch direction and update time.
    /// Full names remain available on hover as the pane narrows.
    fn render_meta_line(pull_request: &PullRequest) -> impl IntoElement {
        let updated = unix_seconds(&pull_request.updated_at).map_or_else(
            || short_date(&pull_request.updated_at),
            |then| time_ago(then, unix_now()),
        );
        div()
            .flex()
            .items_center()
            .justify_end()
            .gap_2()
            .min_w_0()
            .text_xs()
            .text_color(theme::muted())
            .child(Avatar::new(
                pull_request.author.avatar_url.clone(),
                px(HEADER_AVATAR_SIZE),
            ))
            .child(
                div()
                    .id("header-author")
                    .min_w_0()
                    .max_w(px(88.))
                    .truncate()
                    .text_color(theme::text())
                    .tooltip(TextTooltip::builder(pull_request.author.login.clone()))
                    .child(pull_request.author.login.clone()),
            )
            .when_some(pull_request.branches.as_ref(), |meta, branches| {
                let direction = format!("{} → {}", branches.head_ref, branches.base_ref);
                meta.child(
                    div()
                        .id("header-branches")
                        .min_w_0()
                        .truncate()
                        .tooltip(TextTooltip::builder(direction.clone()))
                        .child(direction),
                )
            })
            .child(
                div()
                    .id("pull-request-updated")
                    .flex_none()
                    .tooltip(TextTooltip::builder(format!(
                        "Updated {}",
                        pull_request.updated_at
                    )))
                    .child(NumberFlow::label(updated)),
            )
    }
}
