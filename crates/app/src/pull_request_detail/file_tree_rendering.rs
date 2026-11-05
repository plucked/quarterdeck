use std::ops::Range;

use gpui::{
    AnyElement, Context, Div, IntoElement, Rgba, Stateful, Styled, div, prelude::*, px, relative,
    uniform_list,
};
use qc_file_tree::{TreeNodeKind, TreeRow};

use super::folder_viewed_state::FolderViewedState;
use super::line_counts::LineCounts;
use super::pull_request_detail::PullRequestDetail;
use super::side_panel_tab::SidePanelTab;
use crate::checkbox::{CheckState, Checkbox};
use crate::diff_view::{FILE_STATUS_BADGE_SIZE, render_file_status_badge};
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
/// Size of the line counts' monospace font, and the width of one of its characters.
const COUNT_FONT_SIZE: f32 = 10.;
const COUNT_CHARACTER_WIDTH: f32 = 6.1;
/// Height of the viewed progress bar.
const PROGRESS_BAR_HEIGHT: f32 = 4.;

/// Widths of the added and removed columns, wide enough for the largest
/// count a row shows.
#[derive(Clone, Copy)]
struct CountColumns {
    additions_width: f32,
    deletions_width: f32,
}

/// What every tree row of one frame shares: the count columns' widths and
/// the file at the top of the diff (highlighted), found once per frame.
#[derive(Clone, Copy)]
struct TreeFrame {
    columns: CountColumns,
    current_file: Option<usize>,
}

impl CountColumns {
    /// Columns that fit `largest` with its sign.
    fn fitting(largest: LineCounts) -> Self {
        Self {
            additions_width: count_width(largest.additions),
            deletions_width: count_width(largest.deletions),
        }
    }
}

impl PullRequestDetail {
    /// The panel beside the diff: the Files and Review tabs; Files holds
    /// progress, filter chips, and the files as a folder tree in diff order.
    pub(super) fn render_file_tree_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // The tabs and their content once files are loaded.
        let has_files = !self.loaded_files().is_empty();
        div()
            .flex_none()
            .w(px(self.side_panel_width()))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::panel())
            .text_sm()
            .when(has_files, |panel| {
                let panel = panel.child(self.render_side_panel_tabs(cx));
                match self.review().tab {
                    SidePanelTab::Files => panel
                        .child(self.render_tree_header(cx))
                        .child(self.render_tree_body(cx)),
                    SidePanelTab::Review => panel.child(self.render_review_panel(cx)),
                }
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
        let viewed = self.file_review().viewed_count();
        div()
            .flex()
            .flex_col()
            .gap_2p5()
            .px_3()
            .pt_2p5()
            .pb_3()
            .border_b_1()
            .border_color(theme::border())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .flex_none()
                            .text_xs()
                            .text_color(theme::muted())
                            .child(format!("{viewed} of {} viewed", files.len())),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(render_progress_bar(viewed, files.len())),
                    ),
            )
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

    /// The visible tree rows in `range`, their count columns sized for the
    /// largest counts any row shows so every row's numbers line up.
    fn render_tree_rows(&self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let frame = TreeFrame {
            columns: CountColumns::fitting(self.file_tree().widest_line_counts()),
            current_file: self.current_file(),
        };
        range
            .filter_map(|index| {
                let row = *self.file_tree().rows().get(index)?;
                Some(self.render_tree_row(index, row, frame, cx))
            })
            .collect()
    }

    /// A folder row (chevron, name) or a file row (name), indented by
    /// depth, then right-aligned columns: added and removed lines, the
    /// file's status badge, the viewed box. The file at the top of the diff
    /// is highlighted.
    fn render_tree_row(
        &self,
        index: usize,
        row: TreeRow,
        frame: TreeFrame,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let base = div()
            .id(index)
            .flex()
            .items_center()
            .gap_1p5()
            .w_full()
            .h(px(TREE_ROW_HEIGHT))
            .pl(px(TREE_INDENT
                * f32::from(u16::try_from(row.depth).unwrap_or(u16::MAX))
                + 10.))
            .pr_3()
            .text_sm()
            .cursor_pointer()
            .hover(|style| style.bg(theme::hover()));
        match &self.file_tree().tree().node(row.node).kind {
            TreeNodeKind::Folder { .. } => {
                self.render_folder_row(base, index, row, frame.columns, cx)
            }
            TreeNodeKind::File { file } => self.render_file_row(base, index, *file, row, frame, cx),
        }
    }

    /// A folder: chevron, name, the lines changed under it, and its
    /// tri-state viewed box.
    fn render_folder_row(
        &self,
        base: Stateful<Div>,
        index: usize,
        row: TreeRow,
        columns: CountColumns,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let tree = self.file_tree().tree();
        let node = tree.node(row.node);
        let path = node.path.clone();
        let is_collapsed = self.file_tree().is_folder_collapsed(&path);
        // Worked out once per row refresh, not per frame: a top folder covers every file.
        let summary = self.file_tree().folder_summary(row.node);
        let state = summary.map_or(FolderViewedState::None, |summary| summary.viewed);
        let counts = summary.map(|summary| summary.lines).unwrap_or_default();
        let folder = row.node;
        base.on_click(cx.listener(move |detail, _, _, cx| detail.toggle_folder(&path, cx)))
            .child(
                div()
                    .flex_none()
                    .w_3()
                    .text_xs()
                    .text_color(theme::subtle())
                    .child(if is_collapsed { "▸" } else { "▾" }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_color(theme::muted())
                    .child(node.name.clone()),
            )
            .child(render_line_counts(counts, columns))
            .child(div().flex_none().w(px(FILE_STATUS_BADGE_SIZE)))
            .child(
                div()
                    .id(("folder-viewed", index))
                    .flex_none()
                    .on_click(cx.listener(move |detail, _, _, cx| {
                        cx.stop_propagation();
                        detail.toggle_folder_viewed(folder, cx);
                    }))
                    .child(Checkbox::new(folder_check_state(state))),
            )
            .into_any_element()
    }

    /// A file: name, its added and removed lines, a badge when it was
    /// added, removed or moved, and its viewed box.
    fn render_file_row(
        &self,
        base: Stateful<Div>,
        index: usize,
        file: usize,
        row: TreeRow,
        frame: TreeFrame,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(loaded) = self.loaded_files().get(file) else {
            return base.into_any_element();
        };
        let is_current = frame.current_file == Some(file);
        let columns = frame.columns;
        let name = self.file_tree().tree().node(row.node).name.clone();
        let is_viewed = self.file_review().is_viewed(loaded.filename());
        let counts = LineCounts {
            additions: loaded.file.additions,
            deletions: loaded.file.deletions,
        };
        // A plain modification needs no badge; the counts say enough.
        let badge = match loaded.file.status.as_str() {
            "modified" | "changed" => None,
            status => Some(render_file_status_badge(status)),
        };
        base.when(is_current, |row| row.bg(theme::selected()))
            .on_click(cx.listener(move |detail, _, _, cx| detail.reveal_file(file, cx)))
            .child(div().flex_none().w_3())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .when(is_viewed, |name| name.text_color(theme::subtle()))
                    .child(name),
            )
            .child(render_line_counts(counts, columns))
            .child(
                div()
                    .flex_none()
                    .w(px(FILE_STATUS_BADGE_SIZE))
                    .children(badge),
            )
            .child(
                div()
                    .id(("file-viewed", index))
                    .flex_none()
                    .on_click(cx.listener(move |detail, _, _, cx| {
                        cx.stop_propagation();
                        detail.toggle_file_viewed(file, cx);
                    }))
                    .child(Checkbox::new(CheckState::from_bool(is_viewed))),
            )
            .into_any_element()
    }
}

/// The checkbox state for a folder's viewed state.
fn folder_check_state(state: FolderViewedState) -> CheckState {
    match state {
        FolderViewedState::None => CheckState::Unchecked,
        FolderViewedState::Some => CheckState::Mixed,
        FolderViewedState::All => CheckState::Checked,
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

/// `+12` and `−3` in their columns, right-aligned in a monospace font so
/// digits line up from row to row; a zero leaves its column empty.
fn render_line_counts(counts: LineCounts, columns: CountColumns) -> impl IntoElement {
    let column = |width: f32, text: Option<String>, color: Rgba| {
        div()
            .flex_none()
            .w(px(width))
            .text_right()
            .text_color(color)
            .children(text)
    };
    div()
        .flex_none()
        .flex()
        .gap_1()
        .font_family(theme::MONO_FONT)
        .text_size(px(COUNT_FONT_SIZE))
        .child(column(
            columns.additions_width,
            (counts.additions > 0).then(|| format!("+{}", counts.additions)),
            theme::added(),
        ))
        .child(column(
            columns.deletions_width,
            (counts.deletions > 0).then(|| format!("−{}", counts.deletions)),
            theme::removed(),
        ))
}

/// Width of a count column holding `largest` with its sign.
fn count_width(largest: u64) -> f32 {
    let characters = largest.max(1).to_string().len() + 1;
    f32::from(u16::try_from(characters).unwrap_or(u16::MAX)) * COUNT_CHARACTER_WIDTH
}
