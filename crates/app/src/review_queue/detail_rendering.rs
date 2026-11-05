use gpui::{AnyElement, Context, div, prelude::*};
use qc_github::PullRequest;

use super::ReviewQueue;
use super::changed_files::ChangedFiles;
use super::render_file_diff::render_file_diff;
use crate::markdown_view::MarkdownView;
use crate::theme;

impl ReviewQueue {
    /// The right column: the selected pull request's header, description and
    /// changed files, scrolling as one.
    pub(super) fn render_detail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Scrolling is recorded so a relaunch returns to the same place.
        // `min_w_0` lets the pane shrink to the window, so its text wraps.
        let container = div()
            .id("detail")
            .track_scroll(self.detail_scroll())
            .on_scroll_wheel(cx.listener(|queue, _, _, cx| queue.record_view_state(cx)))
            .flex_1()
            .min_w_0()
            .h_full()
            .overflow_y_scroll()
            .p_4()
            .flex()
            .flex_col()
            .gap_3();

        let Some(pull_request) = self.selected_pull_request() else {
            return container.child(
                div()
                    .text_color(theme::muted())
                    .child("Select a pull request"),
            );
        };

        container
            .child(render_detail_header(pull_request))
            .children(self.render_description())
            .child(self.render_changed_files())
    }

    /// The description rendered as Markdown; `None` when it is empty.
    fn render_description(&self) -> Option<impl IntoElement> {
        let description = self.description()?;
        Some(
            div()
                .p_3()
                .rounded_md()
                .bg(theme::panel())
                .text_sm()
                .child(MarkdownView::new("description", description.clone())),
        )
    }

    /// The changed files in their load state.
    fn render_changed_files(&self) -> AnyElement {
        match self.changed_files() {
            None | Some(ChangedFiles::Loading) => div()
                .text_color(theme::muted())
                .child("Loading files…")
                .into_any_element(),
            Some(ChangedFiles::Failed(message)) => div()
                .text_color(theme::error())
                .child(message.clone())
                .into_any_element(),
            Some(ChangedFiles::Loaded(files)) => div()
                .flex()
                .flex_col()
                .gap_3()
                .children(
                    files
                        .iter()
                        .enumerate()
                        .map(|(index, file)| render_file_diff(index, file)),
                )
                .into_any_element(),
        }
    }
}

/// Title, metadata line and a link that opens the pull request in the browser.
fn render_detail_header(pull_request: &PullRequest) -> impl IntoElement {
    let url = pull_request.html_url.clone();
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_xl().child(pull_request.title.clone()))
        .child(
            div()
                .flex()
                .gap_2()
                .text_sm()
                .text_color(theme::muted())
                .child(div().min_w_0().child(format!(
                    "{}#{} by {} · updated {}",
                    pull_request.repo.full_name(),
                    pull_request.number,
                    pull_request.author.login,
                    pull_request.updated_at
                )))
                .child(
                    div()
                        .id("open-on-github")
                        .flex_none()
                        .text_color(theme::accent())
                        .cursor_pointer()
                        .on_click(move |_, _, cx| cx.open_url(&url))
                        .child("Open on GitHub ↗"),
                ),
        )
}
