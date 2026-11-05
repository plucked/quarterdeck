use std::ops::Range;

use gpui::{
    AnyElement, Context, Div, FontWeight, IntoElement, Rgba, SharedString, Styled, div, prelude::*,
    px, relative, uniform_list,
};
use qc_file_tree::{TreeNodeKind, TreeRow};

use super::filter_toggle::FilterToggle;
use super::folder_viewed_state::FolderViewedState;
use super::pull_request_detail::PullRequestDetail;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

/// Width of the file tree panel.
const PANEL_WIDTH: f32 = 300.;
/// Height of one tree row; the tree is a uniform list.
const TREE_ROW_HEIGHT: f32 = 24.;
/// Indent per tree level.
const TREE_INDENT: f32 = 12.;
/// Height of the viewed progress bar.
const PROGRESS_BAR_HEIGHT: f32 = 4.;

impl PullRequestDetail {
    /// The panel beside the diff: progress, filter chips, and the files as
    /// a folder tree in diff order.
    pub(super) fn render_file_tree_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_none()
            .w(px(PANEL_WIDTH))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::panel())
            .border_l_1()
            .border_color(theme::border())
            .text_sm()
            .child(self.render_tree_header(cx))
            .child(self.render_tree_body(cx))
    }

    /// Title, viewed progress and the filter chips. Chips show what is
    /// shown: clicking one hides that group from the tree and the diff.
    fn render_tree_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let files = self.loaded_files();
        let viewed = files
            .iter()
            .filter(|loaded| self.file_review().is_viewed(loaded.filename()))
            .count();
        let filter = self.file_tree().filter();

        let toggle_chip = |id: &'static str,
                           label: &'static str,
                           is_hidden: bool,
                           toggle: FilterToggle,
                           cx: &mut Context<Self>| {
            render_chip(id.into(), label.into(), !is_hidden)
                .on_click(cx.listener(move |detail, _, _, cx| detail.toggle_filter(toggle, cx)))
        };
        let mut chips = vec![
            toggle_chip(
                "chip-viewed",
                "Viewed",
                filter.hide_viewed,
                FilterToggle::Viewed,
                cx,
            ),
            toggle_chip(
                "chip-tests",
                "Tests",
                filter.hide_tests,
                FilterToggle::Tests,
                cx,
            ),
            toggle_chip(
                "chip-generated",
                "Generated",
                filter.hide_generated,
                FilterToggle::Generated,
                cx,
            ),
        ];
        for count in self.file_tree().extensions() {
            let extension = count.extension.clone();
            let is_shown = !filter.hidden_extensions.contains(&extension);
            chips.push(
                render_chip(
                    SharedString::from(format!("chip-extension-{extension}")),
                    format!("{extension} {}", count.count).into(),
                    is_shown,
                )
                .on_click(
                    cx.listener(move |detail, _, _, cx| detail.toggle_extension(&extension, cx)),
                ),
            );
        }

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
            .child(div().flex().flex_wrap().gap_1().children(chips))
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

/// A filter chip: bright when its group is shown, dimmed and struck
/// through when hidden.
fn render_chip(id: SharedString, label: SharedString, is_shown: bool) -> gpui::Stateful<Div> {
    div()
        .id(id)
        .px_2()
        .rounded_md()
        .border_1()
        .border_color(theme::border())
        .text_xs()
        .cursor_pointer()
        .when(is_shown, |chip| {
            chip.bg(theme::selected()).text_color(theme::text())
        })
        .when(!is_shown, |chip| {
            chip.text_color(theme::muted()).line_through()
        })
        .child(label)
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
