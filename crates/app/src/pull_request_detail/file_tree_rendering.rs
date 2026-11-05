use std::ops::Range;

use gpui::{
    AnyElement, Context, Div, FontWeight, IntoElement, Rgba, Styled, div, prelude::*, px, relative,
    uniform_list,
};
use qc_file_tree::{TreeNodeKind, TreeRow};

use super::folder_viewed_state::FolderViewedState;
use super::pull_request_detail::PullRequestDetail;
use crate::resize_handle::{ResizeEdge, ResizeHandle};
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

/// How narrow and how wide the side panel can be dragged.
const MIN_PANEL_WIDTH: f32 = 220.;
const MAX_PANEL_WIDTH: f32 = 640.;
/// Height of one tree row; the tree is a uniform list.
const TREE_ROW_HEIGHT: f32 = 24.;
/// Indent per tree level.
const TREE_INDENT: f32 = 12.;
/// Height of the viewed progress bar.
const PROGRESS_BAR_HEIGHT: f32 = 4.;

impl PullRequestDetail {
    /// The panel beside the diff: the stack map of a stacked pull request,
    /// then progress, filter chips, and the files as a folder tree in diff order.
    pub(super) fn render_file_tree_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // The stack map on top; the tree once files are loaded.
        let has_files = !self.loaded_files().is_empty();
        div()
            .flex_none()
            .w(px(self.side_panel_width()))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::panel())
            .text_sm()
            .children(self.render_stack_map(cx))
            .when(has_files, |panel| {
                panel
                    .child(self.render_tree_header(cx))
                    .child(self.render_tree_body(cx))
            })
    }

    /// The divider left of the side panel; dragging it resizes the panel.
    pub(super) fn render_side_panel_handle(&self, cx: &mut Context<Self>) -> ResizeHandle {
        let detail = cx.entity().downgrade();
        ResizeHandle::new(
            "side-panel-resize",
            ResizeEdge::Left,
            self.side_panel_width(),
            MIN_PANEL_WIDTH,
            MAX_PANEL_WIDTH,
            move |width, _, cx| {
                detail
                    .update(cx, |detail, cx| detail.set_side_panel_width(width, cx))
                    .ok();
            },
        )
    }

    /// Title, viewed progress and the filter controls.
    fn render_tree_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let files = self.loaded_files();
        let viewed = files
            .iter()
            .filter(|loaded| self.file_review().is_viewed(loaded.filename()))
            .count();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(theme::border())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("Files"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme::muted())
                            .child(format!("{viewed}/{} viewed", files.len())),
                    ),
            )
            .child(render_progress_bar(viewed, files.len()))
            .child(self.render_filter_controls(cx))
    }

    /// The tree rows as a uniform list with its scrollbar, or a note when
    /// the filters hide everything.
    fn render_tree_body(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.file_tree().rows().is_empty() {
            return div()
                .p_3()
                .text_xs()
                .text_color(theme::muted())
                .child("No file matches the filters.")
                .into_any_element();
        }
        div()
            .relative()
            .flex_1()
            .min_h_0()
            .child(
                uniform_list(
                    "file-tree",
                    self.file_tree().rows().len(),
                    cx.processor(|detail, range, _window, cx| detail.render_tree_rows(range, cx)),
                )
                .track_scroll(self.file_tree().scroll().clone())
                .size_full(),
            )
            .child(VerticalScrollbar::new(
                "file-tree-scrollbar",
                ScrollbarTarget::Handle(self.file_tree().scroll().0.borrow().base_handle.clone()),
            ))
            .into_any_element()
    }

    /// The visible tree rows in `range`.
    fn render_tree_rows(&self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        range
            .filter_map(|index| {
                let row = *self.file_tree().rows().get(index)?;
                Some(self.render_tree_row(index, row, cx))
            })
            .collect()
    }

    /// A folder row (chevron, viewed box, name) or a file row (viewed box,
    /// status, name), indented by depth. The file at the top of the diff is
    /// highlighted.
    fn render_tree_row(&self, index: usize, row: TreeRow, cx: &mut Context<Self>) -> AnyElement {
        let tree = self.file_tree().tree();
        let node = tree.node(row.node);
        let base = div()
            .id(index)
            .flex()
            .items_center()
            .gap_1()
            .h(px(TREE_ROW_HEIGHT))
            .pl(px(TREE_INDENT
                * f32::from(u16::try_from(row.depth).unwrap_or(u16::MAX))
                + 8.))
            .pr_2()
            .cursor_pointer()
            .hover(|style| style.bg(theme::hover()));

        match &node.kind {
            TreeNodeKind::Folder { .. } => {
                let path = node.path.clone();
                let is_collapsed = self.file_tree().is_folder_collapsed(&path);
                let state = self.file_tree().folder_viewed_state(
                    row.node,
                    self.loaded_files(),
                    self.file_review(),
                );
                let folder = row.node;
                base.on_click(cx.listener(move |detail, _, _, cx| detail.toggle_folder(&path, cx)))
                    .child(
                        div()
                            .flex_none()
                            .w_3()
                            .text_color(theme::muted())
                            .child(if is_collapsed { "▸" } else { "▾" }),
                    )
                    .child(
                        render_checkbox(folder_glyph(state))
                            .id(("folder-viewed", index))
                            .on_click(cx.listener(move |detail, _, _, cx| {
                                cx.stop_propagation();
                                detail.toggle_folder_viewed(folder, cx);
                            })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(theme::muted())
                            .child(node.name.clone()),
                    )
                    .into_any_element()
            }
            TreeNodeKind::File { file } => {
                let file = *file;
                let Some(loaded) = self.loaded_files().get(file) else {
                    return base.into_any_element();
                };
                let is_viewed = self.file_review().is_viewed(loaded.filename());
                let is_current = self.current_file() == Some(file);
                let (letter, color) = status_badge(&loaded.file.status);
                base.when(is_current, |row| row.bg(theme::selected()))
                    .on_click(cx.listener(move |detail, _, _, cx| detail.reveal_file(file, cx)))
                    .child(div().flex_none().w_3())
                    .child(
                        render_checkbox(if is_viewed { "☑" } else { "☐" })
                            .id(("file-viewed", index))
                            .on_click(cx.listener(move |detail, _, _, cx| {
                                cx.stop_propagation();
                                detail.toggle_file_viewed(file, cx);
                            })),
                    )
                    .child(
                        div()
                            .flex_none()
                            .w_3()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(color)
                            .child(letter),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .when(is_viewed, |name| name.text_color(theme::muted()))
                            .child(node.name.clone()),
                    )
                    .into_any_element()
            }
        }
    }
}

/// A viewed checkbox glyph that can be clicked on its own.
fn render_checkbox(glyph: &'static str) -> Div {
    div()
        .flex_none()
        .w_4()
        .text_color(theme::muted())
        .hover(|style| style.text_color(theme::text()))
        .child(glyph)
}

/// The checkbox glyph for a folder's viewed state.
fn folder_glyph(state: FolderViewedState) -> &'static str {
    match state {
        FolderViewedState::None => "☐",
        FolderViewedState::Some => "◪",
        FolderViewedState::All => "☑",
    }
}

/// The share of files viewed, as a thin bar.
fn render_progress_bar(viewed: usize, total: usize) -> impl IntoElement {
    let fraction = if total == 0 {
        0.
    } else {
        f32::from(u16::try_from(viewed).unwrap_or(u16::MAX))
            / f32::from(u16::try_from(total).unwrap_or(u16::MAX))
    };
    div()
        .h(px(PROGRESS_BAR_HEIGHT))
        .rounded_md()
        .bg(theme::border())
        .child(
            div()
                .h_full()
                .w(relative(fraction.min(1.)))
                .rounded_md()
                .bg(theme::added()),
        )
}

/// GitHub's file status as a letter and its color, like the diff headers show.
fn status_badge(status: &str) -> (&'static str, Rgba) {
    match status {
        "added" => ("A", theme::added()),
        "removed" => ("D", theme::removed()),
        "renamed" => ("R", theme::renamed()),
        "copied" => ("C", theme::renamed()),
        _ => ("M", theme::muted()),
    }
}
