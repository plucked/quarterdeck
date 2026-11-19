use gpui::{AnyElement, Context, FontWeight, IntoElement, Styled, div, prelude::*, px};
use qd_github::{PullRequest, PullRequestState};

use super::github_link::GitHubLink;
use super::pull_request_detail::PullRequestDetail;
use crate::avatar_images::Avatar;
use crate::controls::{NumberFlow, Pill, PillStyle, TextTooltip};
use crate::display_text::{short_date, time_ago, unix_now, unix_seconds};
use crate::theme;

/// The header's one row, as tall as the window's top bar.
const HEADER_HEIGHT: f32 = 40.;
/// The author's avatar in the header.
const HEADER_AVATAR_SIZE: f32 = 20.;
/// The title, a step above the interface's body text.
const HEADER_TITLE_SIZE: f32 = 15.;
/// The widest an author's name is shown before it truncates.
const AUTHOR_MAX_WIDTH: f32 = 88.;
/// How much sooner the branch names give up width than the title and the
/// repository, which shrink at the default factor of 1: a narrow pane
/// truncates the branches away before it touches the title.
const BRANCHES_SHRINK_FACTOR: f32 = 1000.;

impl PullRequestDetail {
    /// One row: status, title, repository and the GitHub link on the left;
    /// author, branches, update time and the activity pill on the right. The
    /// view tabs are in the window's top bar. `None` without a pull request.
    pub(super) fn render_sticky_header(&self, _cx: &mut Context<Self>) -> Option<AnyElement> {
        let pull_request = self.pull_request()?;
        // Every text is a direct child of the row, so the branches, the
        // repository and the title share one order of giving up width. The
        // row's own text style is the small muted one of all but the title.
        let row = div()
            .flex()
            .items_center()
            .gap_2()
            .h(px(HEADER_HEIGHT))
            .px_6()
            .text_xs()
            .text_color(theme::muted())
            .child(self.render_state_pill(pull_request))
            .children(Self::render_identity_items(pull_request))
            // The room left over separates what the pull request is from
            // who wrote it and when.
            .child(div().flex_1())
            .children(Self::render_meta_items(pull_request))
            .children(self.render_activity_pill());

        Some(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .bg(theme::background())
                .border_b_1()
                .border_color(theme::border())
                .child(row)
                .children(self.render_progress_line())
                .into_any_element(),
        )
    }

    /// Open, Draft, Merged or Closed, from GitHub's options once the Actions
    /// tab has read them.
    fn render_state_pill(&self, pull_request: &PullRequest) -> impl IntoElement {
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
        Pill::new(label, PillStyle::Filled)
            .roomy()
            .text_color(color)
            .mark(div().size_2().rounded_full().border_1().border_color(color))
    }

    /// What the pull request is, left to right: its title, its repository
    /// and number, and the link to it on GitHub. Title and repository
    /// truncate together once the branches have; full text stays available
    /// on hover.
    fn render_identity_items(pull_request: &PullRequest) -> [AnyElement; 3] {
        let repository = format!("{} #{}", pull_request.repo.full_name(), pull_request.number);
        [
            div()
                .id("pull-request-title")
                .min_w_0()
                .truncate()
                .text_size(px(HEADER_TITLE_SIZE))
                .font_weight(FontWeight::MEDIUM)
                .text_color(theme::text())
                .tooltip(TextTooltip::builder(pull_request.title.clone()))
                .child(pull_request.title.clone())
                .into_any_element(),
            div()
                .id("header-repository")
                .min_w_0()
                .truncate()
                .tooltip(TextTooltip::builder(repository.clone()))
                .child(NumberFlow::label(repository))
                .into_any_element(),
            GitHubLink::new("open-on-github", pull_request.html_url.clone()).into_any_element(),
        ]
    }

    /// Who wrote it and when, left to right: the author, the branch
    /// direction and the update time. The branches are the first text of the
    /// row to truncate as the pane narrows; full names stay available on
    /// hover.
    fn render_meta_items(pull_request: &PullRequest) -> Vec<AnyElement> {
        let updated = unix_seconds(&pull_request.updated_at).map_or_else(
            || short_date(&pull_request.updated_at),
            |then| time_ago(then, unix_now()),
        );
        let mut items = vec![
            Avatar::new(
                pull_request.author.avatar_url.clone(),
                px(HEADER_AVATAR_SIZE),
            )
            .into_any_element(),
            div()
                .id("header-author")
                .flex_none()
                .max_w(px(AUTHOR_MAX_WIDTH))
                .truncate()
                .text_color(theme::text())
                .tooltip(TextTooltip::builder(pull_request.author.login.clone()))
                .child(pull_request.author.login.clone())
                .into_any_element(),
        ];
        if let Some(branches) = &pull_request.branches {
            let direction = format!("{} → {}", branches.head_ref, branches.base_ref);
            let mut branch_direction = div()
                .id("header-branches")
                .min_w_0()
                .truncate()
                .tooltip(TextTooltip::builder(direction.clone()))
                .child(direction);
            branch_direction.style().flex_shrink = Some(BRANCHES_SHRINK_FACTOR);
            items.push(branch_direction.into_any_element());
        }
        items.push(
            div()
                .id("pull-request-updated")
                .flex_none()
                .tooltip(TextTooltip::builder(format!(
                    "Updated {}",
                    pull_request.updated_at
                )))
                .child(NumberFlow::label(updated))
                .into_any_element(),
        );
        items
    }
}
