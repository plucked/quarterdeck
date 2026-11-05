use std::rc::Rc;

use gpui::{
    AnyElement, ClickEvent, Context, Div, IntoElement, Render, Styled, Window, div, list,
    prelude::*,
};
use qc_cache::{DiffLayout, LineWrapping};
use qc_github::PullRequest;

use super::changed_files::ChangedFiles;
use super::detail_row::DetailRow;
use super::loaded_file::LoadedFile;
use super::pull_request_detail::PullRequestDetail;
use crate::diff_view::{
    DiffRowStyle, DiffSummary, DiffToolbar, FileClickHandler, render_file_end, render_file_header,
    render_file_note, render_hunk_header, render_split_row, render_unified_line,
};
use crate::markdown_view::MarkdownView;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

impl Render for PullRequestDetail {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // `min_w_0` lets the pane shrink to the window, so its text wraps.
        div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .child(
                list(
                    self.list().clone(),
                    cx.processor(|detail, index, _window, cx| detail.render_row(index, cx)),
                )
                .size_full(),
            )
            .child(VerticalScrollbar::new(
                "detail-scrollbar",
                ScrollbarTarget::List(self.list().clone()),
            ))
    }
}

impl PullRequestDetail {
    /// Row `index` of the list, padded like the pane. Called only for rows
    /// in or near view.
    fn render_row(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.rows().get(index) else {
            return div().into_any_element();
        };
        let is_last = index + 1 == self.rows().len();
        let content = match row {
            DetailRow::NoSelection => div()
                .text_color(theme::muted())
                .child("Select a pull request")
                .into_any_element(),
            DetailRow::Header => self.render_header(),
            DetailRow::Description => self.render_description(),
            DetailRow::FilesLoading => render_status_line("Loading files…", theme::muted()),
            DetailRow::FilesFailed => self.render_failure(),
            DetailRow::Toolbar => div()
                .pb_3()
                .child(self.render_toolbar(cx))
                .into_any_element(),
            DetailRow::FileHeader { file } => self.render_file_header_row(file, cx),
            DetailRow::FileNote { file, note } => {
                render_file_note(file, note, Self::toggle_collapsed_handler(file, cx))
            }
            DetailRow::HunkHeader { file, hunk } => self.render_hunk_header_row(file, hunk),
            DetailRow::UnifiedLine { file, hunk, line } => {
                self.with_horizontal_scroll(file, cx, |loaded, style| {
                    let diff_line = &loaded.diff.hunks()[hunk].lines[line];
                    let spans = loaded
                        .syntax
                        .as_deref()
                        .map_or(&[][..], |syntax| syntax.spans(hunk, line));
                    render_unified_line(diff_line, spans, style)
                })
            }
            DetailRow::SplitLine { file, hunk, row } => {
                self.with_horizontal_scroll(file, cx, |loaded, style| {
                    render_split_row(
                        &loaded.diff.hunks()[hunk],
                        hunk,
                        row,
                        loaded.syntax.as_deref(),
                        style,
                    )
                })
            }
            DetailRow::FileEnd { .. } => render_file_end().into_any_element(),
        };
        div()
            .px_4()
            .when(index == 0, Styled::pt_4)
            .when(is_last, Styled::pb_4)
            .child(content)
            .into_any_element()
    }

    /// Title, metadata line and a link that opens the pull request in the browser.
    fn render_header(&self) -> AnyElement {
        let Some(pull_request) = self.pull_request() else {
            return div().into_any_element();
        };
        div()
            .pb_3()
            .child(render_pull_request_header(pull_request))
            .into_any_element()
    }

    /// The description rendered as Markdown.
    fn render_description(&self) -> AnyElement {
        let Some(description) = self.description() else {
            return div().into_any_element();
        };
        div()
            .pb_3()
            .child(
                div()
                    .p_3()
                    .rounded_md()
                    .bg(theme::panel())
                    .text_sm()
                    .child(MarkdownView::new("description", description.clone())),
            )
            .into_any_element()
    }

    fn render_failure(&self) -> AnyElement {
        let message = match self.changed_files() {
            Some(ChangedFiles::Failed(message)) => message.clone(),
            Some(ChangedFiles::Loading | ChangedFiles::Loaded(_)) | None => "".into(),
        };
        render_status_line(message, theme::error())
    }

    /// The totals and view options above the files.
    fn render_toolbar(&self, cx: &mut Context<Self>) -> DiffToolbar {
        let files = self.loaded_files();
        let summary = DiffSummary {
            file_count: files.len(),
            viewed_count: self.file_review().viewed_count(),
            additions: files.iter().map(|loaded| loaded.file.additions).sum(),
            deletions: files.iter().map(|loaded| loaded.file.deletions).sum(),
        };
        let detail = cx.entity().downgrade();
        let collapsing = detail.clone();
        let expanding = detail.clone();
        DiffToolbar::new(self.diff_settings(), summary, move |settings, _, cx| {
            detail
                .update(cx, |detail, cx| detail.set_diff_settings(*settings, cx))
                .ok();
        })
        .on_collapse_all(move |_, cx| {
            collapsing
                .update(cx, PullRequestDetail::collapse_all_files)
                .ok();
        })
        .on_expand_all(move |_, cx| {
            expanding
                .update(cx, PullRequestDetail::expand_all_files)
                .ok();
        })
    }

    /// File `file`'s header, wired to its collapse and viewed toggles.
    fn render_file_header_row(&self, file: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(loaded) = self.loaded_files().get(file) else {
            return div().into_any_element();
        };
        let on_toggle_viewed: FileClickHandler =
            Rc::new(cx.listener(move |detail, _: &ClickEvent, _, cx| {
                detail.toggle_file_viewed(file, cx);
            }));
        render_file_header(
            file,
            &loaded.file,
            self.file_review().state_of(loaded),
            Self::toggle_collapsed_handler(file, cx),
            on_toggle_viewed,
        )
        .into_any_element()
    }

    /// A hunk's `@@` header, indented past the gutters.
    fn render_hunk_header_row(&self, file: usize, hunk: usize) -> AnyElement {
        let Some(loaded) = self.loaded_files().get(file) else {
            return div().into_any_element();
        };
        let indent = match self.diff_settings().layout {
            DiffLayout::Unified => 2. * loaded.gutter_width,
            DiffLayout::Split => loaded.gutter_width,
        };
        render_hunk_header(&loaded.diff.hunks()[hunk].header, indent).into_any_element()
    }

    /// A diff row of file `file` drawn by `render`, which scrolls the
    /// file's lines sideways on horizontal gestures when lines do not wrap.
    fn with_horizontal_scroll(
        &self,
        file: usize,
        cx: &mut Context<Self>,
        render: impl FnOnce(&LoadedFile, DiffRowStyle) -> Div,
    ) -> AnyElement {
        let Some(loaded) = self.loaded_files().get(file) else {
            return div().into_any_element();
        };
        let wrapping = self.diff_settings().wrapping;
        let style = DiffRowStyle {
            gutter_width: loaded.gutter_width,
            wrapping,
            horizontal_offset: self.horizontal_offset(file),
        };
        let row = render(loaded, style);
        match wrapping {
            LineWrapping::Wrap => row.into_any_element(),
            LineWrapping::Scroll => row
                .id(("diff-row", file))
                .on_scroll_wheel(cx.listener(move |detail, event, window, cx| {
                    detail.scroll_file_horizontally(file, event, window, cx);
                }))
                .into_any_element(),
        }
    }

    fn toggle_collapsed_handler(file: usize, cx: &mut Context<Self>) -> FileClickHandler {
        Rc::new(cx.listener(move |detail, _: &ClickEvent, _, cx| {
            detail.toggle_file_collapsed(file, cx);
        }))
    }
}

/// A one-line status (loading, failure) below the description.
fn render_status_line(message: impl Into<gpui::SharedString>, color: gpui::Rgba) -> AnyElement {
    div()
        .pb_3()
        .text_color(color)
        .child(message.into())
        .into_any_element()
}

/// Title, metadata line and a link that opens the pull request in the browser.
fn render_pull_request_header(pull_request: &PullRequest) -> impl IntoElement {
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
