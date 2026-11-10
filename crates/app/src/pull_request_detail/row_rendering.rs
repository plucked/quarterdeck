use std::rc::Rc;

use gpui::{
    AnyElement, Canvas, ClickEvent, Context, DispatchPhase, Div, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render, Styled, WeakEntity, Window, canvas, div,
    list, prelude::*, px,
};
use qd_cache::{DiffLayout, LineWrapping, PanelVisibility};
use qd_diff::{GapPlace, LineOrigin};
use qd_github::LineSide;

use super::detail_mode::DetailMode;
use super::detail_row::DetailRow;
use super::loaded_file::LoadedFile;
use super::pull_request_detail::PullRequestDetail;
use super::selection_granularity::DOUBLE_CLICK;
use crate::app_commands::Copy;
use crate::diff_view::{
    DiffRowStyle, DiffSide, DiffSummary, DiffSummaryBar, FileClickHandler, GapClickHandler,
    GapPart, render_file_end, render_file_header, render_file_note, render_gap_bar,
    render_hunk_header, render_split_row, render_unified_line,
};
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

/// Size of the `+` that comments on a line.
const COMMENT_BUTTON_WIDTH: f32 = 14.;

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
                ScrollbarTarget::EstimatedList(self.list().clone(), self.current_row_tops()),
            ))
            .child(
                drag_listener(cx.entity().downgrade())
                    .absolute()
                    .size_full(),
            );

        // The sticky header over the diff, in one column; the side panel
        // beside that column at full height when shown. `min_w_0` and
        // `min_h_0` let the pane shrink to the window.
        // The side panel holds the files, the review and the actions; it
        // stays while another pull request's files load, so it does not flicker.
        let shows_tree = self.file_tree().visibility() == PanelVisibility::Shown
            && self.pull_request().is_some();
        let main_column = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .h_full()
            .children(self.render_sticky_header(cx))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(match self.brief().mode {
                        DetailMode::Diff => diff_area.into_any_element(),
                        DetailMode::Brief => self.render_brief_area(cx),
                    }),
            );
        div()
            .flex()
            .flex_1()
            .min_w_0()
            .h_full()
            .track_focus(self.focus())
            .on_action(cx.listener(|detail, _: &Copy, _, cx| detail.copy_diff_selection(cx)))
            .child(main_column)
            .when(shows_tree, |pane| {
                pane.child(self.render_side_panel_handle(cx))
                    .child(self.render_file_tree_panel(cx))
            })
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
            DetailRow::HunkHeader { file, hunk } => {
                self.render_hunk_header_row(index, file, hunk, cx)
            }
            DetailRow::EndGap { file } => self.render_end_gap_row(index, file, cx),
            DetailRow::UnifiedLine { file, hunk, line } => {
                let cells = self.code_cells(index);
                let numbers = self
                    .loaded_files()
                    .get(file)
                    .and_then(|loaded| loaded.diff.hunks().get(hunk)?.lines.get(line))
                    // GitHub takes comments only on lines of the patch.
                    .filter(|diff_line| diff_line.origin == LineOrigin::Patch)
                    .map_or_else(RowLines::default, |diff_line| RowLines {
                        old: diff_line.old_number,
                        new: diff_line.new_number,
                    });
                let content = self.with_horizontal_scroll(file, cx, |loaded, style| {
                    let diff_line = &loaded.diff.hunks()[hunk].lines[line];
                    let spans = loaded
                        .syntax
                        .as_deref()
                        .map_or(&[][..], |syntax| syntax.spans(hunk, line));
                    render_unified_line(diff_line, spans, style, &cells)
                });
                self.with_comment_button(index, file, numbers, content, cx)
            }
            DetailRow::SplitLine { file, hunk, row } => {
                let cells = self.code_cells(index);
                let numbers = self
                    .loaded_files()
                    .get(file)
                    .and_then(|loaded| loaded.diff.hunks().get(hunk))
                    // GitHub takes comments only on lines of the patch.
                    .filter(|hunk| {
                        let line = row.new.or(row.old).and_then(|index| hunk.lines.get(index));
                        line.is_some_and(|line| line.origin == LineOrigin::Patch)
                    })
                    .map_or_else(RowLines::default, |hunk| RowLines {
                        old: row.old.and_then(|index| hunk.lines.get(index)?.old_number),
                        new: row.new.and_then(|index| hunk.lines.get(index)?.new_number),
                    });
                let content = self.with_horizontal_scroll(file, cx, |loaded, style| {
                    render_split_row(
                        &loaded.diff.hunks()[hunk],
                        hunk,
                        row,
                        loaded.syntax.as_deref(),
                        style,
                        &cells,
                    )
                });
                self.with_comment_button(index, file, numbers, content, cx)
            }
            DetailRow::FileEnd { .. } => render_file_end().into_any_element(),
            DetailRow::CommentCard { card, .. } => self.render_comment_card_row(card, cx),
            DetailRow::Composer { .. } => self.render_composer_row(cx),
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
        let message = self.load_failure().cloned().unwrap_or_default();
        render_status_line(message, theme::danger())
    }

    /// The totals above the files, with collapse and expand all.
    fn render_summary_bar(&self, cx: &mut Context<Self>) -> DiffSummaryBar {
        let files = self.loaded_files();
        let summary = DiffSummary {
            file_count: files.len(),
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

    /// File `file`'s header, wired to its collapse and viewed toggles and
    /// to showing it in the file tree.
    fn render_file_header_row(&self, file: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(loaded) = self.loaded_files().get(file) else {
            return div().into_any_element();
        };
        let on_toggle_viewed: FileClickHandler =
            Rc::new(cx.listener(move |detail, _: &ClickEvent, _, cx| {
                detail.toggle_file_viewed(file, cx);
            }));
        let on_show_in_tree: FileClickHandler =
            Rc::new(cx.listener(move |detail, _: &ClickEvent, _, cx| {
                detail.show_file_in_tree(file, cx);
            }));
        render_file_header(
            file,
            &loaded.file,
            self.file_review().state_of(loaded),
            Self::toggle_collapsed_handler(file, cx),
            on_toggle_viewed,
            on_show_in_tree,
        )
        .into_any_element()
    }

    /// A hunk's `@@` header, indented past the gutters; with the buttons
    /// revealing the hidden lines above it, when there are some.
    fn render_hunk_header_row(
        &self,
        row: usize,
        file: usize,
        hunk: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(loaded) = self.loaded_files().get(file) else {
            return div().into_any_element();
        };
        let header = &loaded.diff.hunks()[hunk].header;
        let place = GapPlace::AboveHunk(hunk);
        if let Some(gap) = loaded.gap_at(place) {
            let on_click = Self::gap_click_handler(file, place, cx);
            let load = self.context_loads().load_of(file);
            return render_gap_bar(row, gap, Some(header), load, &on_click).into_any_element();
        }
        let indent = match self.diff_settings().layout {
            DiffLayout::Unified => 2. * loaded.gutter_width,
            DiffLayout::Split => loaded.gutter_width,
        };
        render_hunk_header(header, indent).into_any_element()
    }

    /// The bar over the hidden lines below file `file`'s last hunk.
    fn render_end_gap_row(&self, row: usize, file: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(gap) = self
            .loaded_files()
            .get(file)
            .and_then(|loaded| loaded.gap_at(GapPlace::End))
        else {
            return div().into_any_element();
        };
        let on_click = Self::gap_click_handler(file, GapPlace::End, cx);
        let load = self.context_loads().load_of(file);
        render_gap_bar(row, gap, None, load, &on_click).into_any_element()
    }

    /// Reveals the clicked part of the gap at `place` in file `file`.
    fn gap_click_handler(file: usize, place: GapPlace, cx: &mut Context<Self>) -> GapClickHandler {
        Rc::new(cx.listener(move |detail, part: &GapPart, _, cx| {
            detail.expand_context(file, place, *part, cx);
        }))
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

    /// A diff line with the `+` that comments on it, shown left of the line
    /// while the line or the `+` is hovered (shift-clicking it stretches the
    /// open comment); a double click on the line opens a comment too. The
    /// lines of the comment being written are tinted.
    fn with_comment_button(
        &self,
        index: usize,
        file: usize,
        numbers: RowLines,
        content: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some((side, line)) = numbers.primary() else {
            return content;
        };
        let is_in_comment =
            self.comment_range()
                .is_some_and(|(range_file, range_side, first, last)| {
                    range_file == file
                        && numbers
                            .on(range_side)
                            .is_some_and(|number| (first..=last).contains(&number))
                });

        // The hover area reaches into the pane's padding where the `+` sits,
        // so moving the pointer onto the `+` keeps it shown.
        div()
            .id(("diff-line", index))
            .relative()
            .group("diff-line")
            .ml(px(-COMMENT_BUTTON_WIDTH))
            .pl(px(COMMENT_BUTTON_WIDTH))
            .on_click(cx.listener(move |detail, event: &ClickEvent, window, cx| {
                if event.click_count() != DOUBLE_CLICK {
                    return;
                }
                // The side clicked in a split row, else the line's main side.
                let clicked_side = detail
                    .code_segments()
                    .hit(event.position())
                    .map(|(side, _)| side);
                let (side, line) = match (clicked_side, numbers.old) {
                    (Some(DiffSide::Old), Some(old)) => (LineSide::Old, old),
                    (Some(DiffSide::Old | DiffSide::New | DiffSide::Unified) | None, _) => {
                        (side, line)
                    }
                };
                detail.clear_diff_selection();
                detail.open_composer(file, side, line, false, window, cx);
            }))
            .child(content)
            .when(is_in_comment, |row| {
                row.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(px(COMMENT_BUTTON_WIDTH))
                        .right_0()
                        .bg(theme::comment_range())
                        .border_l_2()
                        .border_color(theme::accent()),
                )
            })
            .child(
                div()
                    .id(("comment-line", index))
                    .absolute()
                    .top_0()
                    .left_0()
                    .w(px(COMMENT_BUTTON_WIDTH))
                    .h(px(COMMENT_BUTTON_WIDTH))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .text_xs()
                    .text_color(theme::background())
                    .bg(theme::accent())
                    .cursor_pointer()
                    .opacity(0.)
                    .group_hover("diff-line", |style| style.opacity(1.))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |detail, event: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        detail.open_composer(file, side, line, event.modifiers().shift, window, cx);
                    }))
                    .child("+"),
            )
            .into_any_element()
    }
}

/// The line numbers a diff row shows on each side.
#[derive(Clone, Copy, Default)]
struct RowLines {
    old: Option<u32>,
    new: Option<u32>,
}

impl RowLines {
    /// The line a comment on the row goes to: the new side's, else the old side's.
    fn primary(self) -> Option<(LineSide, u32)> {
        self.new
            .map(|line| (LineSide::New, line))
            .or(self.old.map(|line| (LineSide::Old, line)))
    }

    /// The row's line on `side`.
    fn on(self, side: LineSide) -> Option<u32> {
        match side {
            LineSide::Old => self.old,
            LineSide::New => self.new,
        }
    }
}

/// A one-line status (a failure, every file hidden) in place of the files.
fn render_status_line(message: impl Into<gpui::SharedString>, color: gpui::Rgba) -> AnyElement {
    div()
        .pb_3()
        .text_color(color)
        .child(message.into())
        .into_any_element()
}
