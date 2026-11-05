use std::rc::Rc;

use gpui::{
    AnyElement, Canvas, ClickEvent, Context, DispatchPhase, Div, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render, Styled, WeakEntity, Window, canvas, div,
    list, prelude::*,
};
use qc_cache::{DiffLayout, LineWrapping, PanelVisibility};

use super::changed_files::ChangedFiles;
use super::detail_row::DetailRow;
use super::loaded_file::LoadedFile;
use super::pull_request_detail::PullRequestDetail;
use crate::app_commands::Copy;
use crate::diff_view::{
    DiffRowStyle, DiffSummary, DiffSummaryBar, FileClickHandler, render_file_end,
    render_file_header, render_file_note, render_hunk_header, render_split_row,
    render_unified_line,
};
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

impl Render for PullRequestDetail {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // This frame's code texts register again as they paint.
        self.code_segments().clear();

        // The diff: the virtualized rows, their scrollbar, and code selection.
        let diff_area = div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|detail, event: &MouseDownEvent, window, cx| {
                    detail.begin_diff_selection(event.position, event.click_count, window, cx);
                }),
            )
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
            .child(
                drag_listener(cx.entity().downgrade())
                    .absolute()
                    .size_full(),
            );

        // The sticky header above; the file tree beside the diff when shown.
        // `min_w_0` and `min_h_0` let the pane shrink to the window.
        // The side panel holds the stack map and the file tree; it shows when either has content.
        let shows_tree = self.file_tree().visibility() == PanelVisibility::Shown
            && (!self.loaded_files().is_empty() || self.stack().is_some());
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .track_focus(self.focus())
            .on_action(cx.listener(|detail, _: &Copy, _, cx| detail.copy_diff_selection(cx)))
            .children(self.render_sticky_header(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(diff_area)
                    .when(shows_tree, |pane| {
                        pane.child(self.render_file_tree_panel(cx))
                    }),
            )
    }
}

/// An empty overlay whose paint listens to mouse moves and releases
/// anywhere in the window, so a code selection keeps following a drag that
/// leaves the pane (which also scrolls it).
fn drag_listener(detail: WeakEntity<PullRequestDetail>) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |_, (), window, _| {
            let moving = detail.clone();
            window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
                let is_drag = event.pressed_button == Some(MouseButton::Left);
                if phase != DispatchPhase::Bubble || !is_drag {
                    return;
                }
                moving
                    .update(cx, |detail, cx| {
                        if detail.is_selecting() {
                            detail.extend_diff_selection(event.position, cx);
                        }
                    })
                    .ok();
            });
            window.on_mouse_event(move |event: &MouseUpEvent, phase, _window, cx| {
                if phase == DispatchPhase::Bubble && event.button == MouseButton::Left {
                    detail
                        .update(cx, PullRequestDetail::finish_diff_selection)
                        .ok();
                }
            });
        },
    )
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
            DetailRow::FilesLoading => self.render_loading_card(),
            DetailRow::FilesFailed => self.render_failure(),
            DetailRow::AllFilesHidden => self.render_all_files_hidden(),
            DetailRow::SummaryBar => div()
                .pb_3()
                .child(self.render_summary_bar(cx))
                .into_any_element(),
            DetailRow::FileHeader { file } => self.render_file_header_row(file, cx),
            DetailRow::FileNote { file, note } => {
                render_file_note(file, note, Self::toggle_collapsed_handler(file, cx))
            }
            DetailRow::HunkHeader { file, hunk } => self.render_hunk_header_row(file, hunk),
            DetailRow::UnifiedLine { file, hunk, line } => {
                let cells = self.code_cells(index);
                self.with_horizontal_scroll(file, cx, |loaded, style| {
                    let diff_line = &loaded.diff.hunks()[hunk].lines[line];
                    let spans = loaded
                        .syntax
                        .as_deref()
                        .map_or(&[][..], |syntax| syntax.spans(hunk, line));
                    render_unified_line(diff_line, spans, style, &cells)
                })
            }
            DetailRow::SplitLine { file, hunk, row } => {
                let cells = self.code_cells(index);
                self.with_horizontal_scroll(file, cx, |loaded, style| {
                    render_split_row(
                        &loaded.diff.hunks()[hunk],
                        hunk,
                        row,
                        loaded.syntax.as_deref(),
                        style,
                        &cells,
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

    /// The row in place of the files when the filters hide them all.
    fn render_all_files_hidden(&self) -> AnyElement {
        let all_viewed = self
            .loaded_files()
            .iter()
            .all(|loaded| self.file_review().is_viewed(loaded.filename()));
        let filter = self.file_tree().filter();
        let message = if all_viewed && filter.hide_viewed && !filter.has_chips_set() {
            "Every file is viewed.".to_owned()
        } else {
            format!("No file matches: {}.", filter.describe().join(" · "))
        };
        render_status_line(message, theme::muted())
    }

    fn render_failure(&self) -> AnyElement {
        let message = match self.changed_files() {
            Some(ChangedFiles::Failed(message)) => message.clone(),
            Some(ChangedFiles::Loading(_) | ChangedFiles::Loaded(_)) | None => "".into(),
        };
        render_status_line(message, theme::error())
    }

    /// The totals above the files, with collapse and expand all.
    fn render_summary_bar(&self, cx: &mut Context<Self>) -> DiffSummaryBar {
        let files = self.loaded_files();
        let summary = DiffSummary {
            file_count: files.len(),
            viewed_count: self.file_review().viewed_count(),
            additions: files.iter().map(|loaded| loaded.file.additions).sum(),
            deletions: files.iter().map(|loaded| loaded.file.deletions).sum(),
        };
        let collapsing = cx.entity().downgrade();
        let expanding = collapsing.clone();
        DiffSummaryBar::new(summary)
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
