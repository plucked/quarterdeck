use std::rc::Rc;

use gpui::{
    AnyElement, Canvas, ClickEvent, Context, DispatchPhase, Div, IntoElement, MouseButton,
    MouseDownEvent, MouseMoveEvent, MouseUpEvent, Render, Styled, WeakEntity, Window, canvas, div,
    list, prelude::*, px,
};
use qd_cache::{DiffLayout, LineWrapping};
use qd_diff::{DiffLine, GapPlace};
use qd_github::LineSide;

use super::comment_range::CommentRange;
use super::composer_opening::ComposerOpening;
use super::detail_mode::DetailMode;
use super::detail_row::DetailRow;
use super::diff_line_ref::DiffLineRef;
use super::loaded_file::LoadedFile;
use super::pull_request_detail::PullRequestDetail;
use super::selection_granularity::DOUBLE_CLICK;
use crate::app_commands::Copy;
use crate::controls::{Button, NumberFlow, TextTooltip};
use crate::diff_view::{
    DiffRowStyle, DiffSide, DiffSummary, DiffSummaryBar, FileClickHandler, GapClickHandler,
    GapPart, render_file_end, render_file_header, render_file_note, render_gap_bar,
    render_hunk_header, render_split_row, render_unified_line,
};
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

/// Size of the `+` that comments on a line.
pub(super) const COMMENT_BUTTON_WIDTH: f32 = 14.;

impl Render for PullRequestDetail {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // This frame's code texts register again as they paint.
        self.code_segments().clear();

        // The sticky header over the diff, in one column; the side panel
        // beside that column at full height when shown. `min_w_0` and
        // `min_h_0` let the pane shrink to the window.
        // The side panel holds the files, the review and the actions; it
        // stays while another pull request's files load, so it does not flicker.
        let shows_side_panel = self.is_side_panel_shown() && self.pull_request().is_some();
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
                        DetailMode::Summary => self.render_summary_area(),
                        DetailMode::Diff => match &self.focus_drawer.preview {
                            Some(preview) => self.render_search_preview(preview, cx),
                            None => self.render_diff_area(cx),
                        },
                        DetailMode::GuidedReview => self.render_guided_review_area(cx),
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
            .when(shows_side_panel, |pane| {
                pane.child(self.render_side_panel_handle(cx))
                    .child(self.render_side_panel(cx))
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
    /// Shared virtualized diff surface, used by both the full review and inline group disclosures.
    pub(super) fn render_diff_area(&mut self, cx: &mut Context<Self>) -> AnyElement {
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
            .child(self.render_sticky_file_header(cx))
            .child(VerticalScrollbar::new(
                "detail-scrollbar",
                ScrollbarTarget::EstimatedList(self.list().clone(), self.current_row_tops()),
            ))
            .child(
                drag_listener(cx.entity().downgrade())
                    .absolute()
                    .size_full(),
            )
            .when_some(self.selected_comment_range, |area, range| {
                area.child(
                    div()
                        .absolute()
                        .bottom_2()
                        .right_4()
                        .debug_selector(|| "selected-lines-comment".into())
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .child(
                            Button::new(
                                "comment-on-selected-lines",
                                format!("Comment on lines {}–{}", range.first, range.last),
                            )
                            .on_click(cx.listener(
                                move |detail, _, window, cx| {
                                    detail.open_range_composer(range, window, cx);
                                },
                            )),
                        ),
                )
            });

        div()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .flex()
            .flex_col()
            .children(self.render_diff_find(cx))
            .child(diff_area.min_h_0())
            .into_any_element()
    }

    /// Row `index` of the list, padded like the pane. Called only for rows
    /// in or near view.
    fn render_row(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.rows().get(index) else {
            return div().into_any_element();
        };
        let row_id = gpui::SharedString::from(format!("detail-row-{row:?}"));
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
                self.with_horizontal_scroll(file, cx, |loaded, style| {
                    let diff_line = &loaded.diff.hunks()[hunk].lines[line];
                    let spans = loaded
                        .syntax
                        .as_deref()
                        .map_or(&[][..], |syntax| syntax.spans(hunk, line));
                    let content = render_unified_line(diff_line, spans, style, &cells);
                    self.with_comment_button(index, file, DiffSide::Unified, diff_line, content, cx)
                })
            }
            DetailRow::SplitLine { file, hunk, row } => {
                let cells = self.code_cells(index);
                self.with_horizontal_scroll(file, cx, |loaded, mut style| {
                    // Each split column reserves its own slot before its line numbers.
                    style.gutter_width += COMMENT_BUTTON_WIDTH;
                    render_split_row(
                        &loaded.diff.hunks()[hunk],
                        hunk,
                        row,
                        loaded.syntax.as_deref(),
                        style,
                        &cells,
                        |side, line, half| {
                            self.with_comment_button(index, file, side, line, half, cx)
                        },
                    )
                    .into_any_element()
                })
            }
            DetailRow::FileEnd { .. } => render_file_end().into_any_element(),
            DetailRow::CommentCard { card, .. } => self.render_comment_card_row(card, cx),
            DetailRow::Composer { .. } => self.render_composer_row(cx),
        };
        div()
            .id(row_id)
            .when(!self.guided_diff_active(), Styled::px_4)
            .when(index == 0 && !self.guided_diff_active(), Styled::pt_4)
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
        if !self.file_tree.search.query.trim().is_empty() {
            return render_status_line(
                "No files match this search and the active filters.",
                theme::muted(),
            );
        }
        let message = if all_viewed && filter.hide_viewed && !self.file_tree().has_chips_set() {
            "Every file is viewed.".to_owned()
        } else {
            format!(
                "No file matches: {}.",
                self.file_tree().describe_filter().join(" · ")
            )
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
    pub(super) fn render_file_header_row(&self, file: usize, cx: &mut Context<Self>) -> AnyElement {
        if self.guided_diff_active() {
            return self.render_navigation_file(
                file,
                "group-diff-file",
                super::navigation_file_target::NavigationFileTarget::GroupDiff,
                cx,
            );
        }
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
            DiffLayout::Split => loaded.gutter_width + COMMENT_BUTTON_WIDTH,
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
        cx: &Context<Self>,
        render: impl FnOnce(&LoadedFile, DiffRowStyle) -> AnyElement,
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
            LineWrapping::Wrap => row,
            LineWrapping::Scroll => div()
                .child(row)
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

    /// The selected lines this gutter action can comment on, respecting the selected split column.
    fn selected_range_at(
        &self,
        index: usize,
        file: usize,
        column: DiffSide,
        line: &DiffLine,
    ) -> Option<CommentRange> {
        self.selected_comment_range.filter(|range| {
            self.diff_selection.is_some_and(|selection| {
                selection.side == column && selection.range_in_row(index).is_some()
            }) && crate::diff_line_sides::line_number_on(line, range.side).is_some_and(|number| {
                range.contains(DiffLineRef {
                    file,
                    side: range.side,
                    line: number,
                })
            })
        })
    }

    /// Adds the line's own gutter action and optional double click. Split columns receive
    /// independent handlers so blank space and line numbers always target the side under the pointer.
    fn with_comment_button(
        &self,
        index: usize,
        file: usize,
        column: DiffSide,
        line: &DiffLine,
        content: Div,
        cx: &Context<Self>,
    ) -> AnyElement {
        let Some(at) = DiffLineRef::on_patch(file, column, line) else {
            return content.into_any_element();
        };
        let (group, button_id, tooltip) = match column {
            DiffSide::Unified => ("diff-line", "comment-line", "Comment on this line"),
            DiffSide::Old => (
                "old-diff-line",
                "comment-old-line",
                "Comment on the left line",
            ),
            DiffSide::New => (
                "new-diff-line",
                "comment-new-line",
                "Comment on the right line",
            ),
        };
        let selected = self.selected_range_at(index, file, column, line);
        let is_in_comment = self.comment_range().is_some_and(|range| {
            let on_side = matches!(
                (column, range.side),
                (DiffSide::Unified, _)
                    | (DiffSide::Old, LineSide::Old)
                    | (DiffSide::New, LineSide::New)
            );
            on_side
                && range.file == file
                && crate::diff_line_sides::line_number_on(line, range.side)
                    .is_some_and(|number| (range.first..=range.last).contains(&number))
        });
        let content = div()
            .when(column != DiffSide::Unified, |outer| {
                outer.flex().flex_1().min_w_0()
            })
            .when(column == DiffSide::Unified, |outer| {
                outer
                    .ml(px(-COMMENT_BUTTON_WIDTH))
                    .pl(px(COMMENT_BUTTON_WIDTH))
            })
            .id((group, index))
            .relative()
            .group(group)
            .on_click(cx.listener(move |detail, event: &ClickEvent, window, cx| {
                if event.click_count() == DOUBLE_CLICK
                    && detail.diff_settings().double_click_to_comment
                {
                    detail.clear_diff_selection();
                    detail.open_composer(at, ComposerOpening::New, window, cx);
                }
            }))
            .child(content);
        content
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
                    .id((button_id, index))
                    .debug_selector(move || format!("{button_id}-{index}"))
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
                    .group_hover(group, |style| style.opacity(1.))
                    .tooltip(TextTooltip::builder(tooltip))
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(cx.listener(move |detail, event: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        if event.modifiers().shift {
                            detail.open_composer(at, ComposerOpening::ExtendOpen, window, cx);
                        } else if let Some(range) = selected {
                            detail.open_range_composer(range, window, cx);
                        } else {
                            detail.open_composer(at, ComposerOpening::New, window, cx);
                        }
                    }))
                    .child("+"),
            )
            .into_any_element()
    }
}

/// A one-line status (a failure, every file hidden) in place of the files.
fn render_status_line(message: impl Into<gpui::SharedString>, color: gpui::Rgba) -> AnyElement {
    div()
        .pb_3()
        .text_color(color)
        .child(NumberFlow::label(message))
        .into_any_element()
}
