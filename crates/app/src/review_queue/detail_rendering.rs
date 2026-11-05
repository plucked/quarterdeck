use gpui::{AnyElement, Context, Div, Stateful, div, prelude::*};
use qc_github::PullRequest;

use super::ReviewQueue;
use super::changed_files::ChangedFiles;
use super::loaded_file::LoadedFile;
use crate::diff_view::{DiffSummary, DiffToolbar, FileDiffView};
use crate::markdown_view::MarkdownView;
use crate::scrollbar::VerticalScrollbar;
use crate::theme;

impl ReviewQueue {
    /// The right column: the selected pull request's header, description and
    /// changed files, scrolling as one, with a scrollbar on its right edge.
    pub(super) fn render_detail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // `min_w_0` lets the pane shrink to the window, so its text wraps.
        div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(self.render_detail_scroll_area(cx))
            .child(VerticalScrollbar::new(
                "detail-scrollbar",
                self.detail_scroll().clone(),
            ))
    }

    /// The scrolling content of the right column.
    fn render_detail_scroll_area(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        // Scrolling is recorded so a relaunch returns to the same place.
        let container = div()
            .id("detail")
            .track_scroll(self.detail_scroll())
            .on_scroll_wheel(cx.listener(|queue, _, _, cx| queue.record_view_state(cx)))
            .size_full()
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
            .child(self.render_changed_files(cx))
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
    fn render_changed_files(&self, cx: &mut Context<Self>) -> AnyElement {
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
                .child(self.render_diff_toolbar(files, cx))
                .children(
                    files
                        .iter()
                        .enumerate()
                        .map(|(index, loaded)| self.render_file(index, loaded, cx)),
                )
                .into_any_element(),
        }
    }

    /// The totals and view options above the files.
    fn render_diff_toolbar(&self, files: &[LoadedFile], cx: &mut Context<Self>) -> DiffToolbar {
        let summary = DiffSummary {
            file_count: files.len(),
            viewed_count: self.file_review().viewed_count(),
            additions: files.iter().map(|loaded| loaded.file.additions).sum(),
            deletions: files.iter().map(|loaded| loaded.file.deletions).sum(),
        };
        let queue = cx.entity().downgrade();
        let collapsing = queue.clone();
        let expanding = queue.clone();
        DiffToolbar::new(self.diff_settings(), summary, move |settings, _, cx| {
            queue
                .update(cx, |queue, cx| queue.set_diff_settings(*settings, cx))
                .ok();
        })
        .on_collapse_all(move |_, cx| {
            collapsing.update(cx, ReviewQueue::collapse_all_files).ok();
        })
        .on_expand_all(move |_, cx| {
            expanding.update(cx, ReviewQueue::expand_all_files).ok();
        })
    }

    /// One file's diff, wired to this view's collapse and viewed state.
    fn render_file(
        &self,
        index: usize,
        loaded: &LoadedFile,
        cx: &mut Context<Self>,
    ) -> FileDiffView {
        FileDiffView::new(
            index,
            loaded.file.clone(),
            loaded.diff.clone(),
            self.diff_settings(),
            self.file_review().state_of(loaded),
        )
        .syntax(loaded.syntax.clone())
        .on_toggle_collapsed(cx.listener(move |queue, _, _, cx| {
            queue.toggle_file_collapsed(index, cx);
        }))
        .on_toggle_viewed(cx.listener(move |queue, _, _, cx| {
            queue.toggle_file_viewed(index, cx);
        }))
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
