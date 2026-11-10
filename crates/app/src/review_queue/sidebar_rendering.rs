use gpui::{AnyElement, Context, FontWeight, SharedString, div, list, prelude::*, px};
use qd_cache::QueueTab;

use super::ReviewQueue;
use super::others_filter_rendering::{render_filter_button, render_others_empty_state};
use super::pull_request_row_rendering::render_pull_request_row;
use super::queue_row::QueueRow;
use super::refresh_status::RefreshStatus;
use super::repo_group::RepoGroup;
use super::stack_row_rendering::{on_layer_click, render_layer_row, render_stack_header};
use crate::controls::{
    FoldChevron, FoldState, PulsingStatus, RepoMonogram, ResizeEdge, ResizeHandle,
    SegmentedControl, view_option,
};
use crate::display_text::unix_now;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::theme;

/// How narrow and how wide the sidebar can be dragged.
const MIN_SIDEBAR_WIDTH: f32 = 240.;
const MAX_SIDEBAR_WIDTH: f32 = 720.;

impl ReviewQueue {
    /// The left column: the tabs, refresh button and state, then the rows.
    pub(super) fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w(px(self.sidebar_width()))
            .h_full()
            .flex_shrink_0()
            .bg(theme::sidebar())
            .child(self.render_sidebar_header(cx))
            .when(self.shows_others_empty_state(), |sidebar| {
                sidebar.child(render_others_empty_state())
            })
            .when(!self.shows_others_empty_state(), |sidebar| {
                sidebar.child(
                    // The list fills the rest of the column, with a scrollbar over its right edge.
                    div()
                        .relative()
                        .flex_1()
                        .min_h_0()
                        .child(
                            list(
                                self.queue_list().clone(),
                                cx.processor(|queue, index, _window, cx| {
                                    queue.render_queue_row(index, cx)
                                }),
                            )
                            .size_full(),
                        )
                        .child(VerticalScrollbar::new(
                            "pull-requests-scrollbar",
                            ScrollbarTarget::List(self.queue_list().clone()),
                        )),
                )
            })
    }

    /// Whether the sidebar shows the Others tab's invitation to choose
    /// repositories instead of a list.
    fn shows_others_empty_state(&self) -> bool {
        self.tab() == QueueTab::Others && !self.others_tab().has_watched()
    }

    /// The divider right of the sidebar; dragging it resizes the sidebar.
    pub(super) fn render_sidebar_handle(&self, cx: &mut Context<Self>) -> ResizeHandle {
        let queue = cx.entity().downgrade();
        ResizeHandle::new(
            "sidebar-resize",
            ResizeEdge::Right,
            self.sidebar_width(),
            MIN_SIDEBAR_WIDTH,
            MAX_SIDEBAR_WIDTH,
            move |width, _, cx| {
                queue
                    .update(cx, |queue, cx| queue.set_sidebar_width(width, cx))
                    .ok();
            },
        )
    }

    /// The Mine / Review / Others tabs atop the list, and what is going on
    /// (refreshing, opening a layer, a failure) while it lasts.
    fn render_sidebar_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let status = self.render_status_line();
        div()
            .flex()
            .flex_col()
            .gap_2()
            .px_3()
            .pt_3()
            .pb_2()
            .child(self.render_tabs(cx))
            .children(status)
    }

    /// The Mine / Review / Others tabs with their counts (Others: those its
    /// filters show).
    fn render_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = self.tab();
        let authored = self.lists().authored.len();
        let review_requested = self.review_tab().len();
        let others = self.others_tab().shown().len();
        let queue = cx.entity().downgrade();
        let show_tab = |tab| view_option(&queue, tab, ReviewQueue::show_tab);
        SegmentedControl::new("queue-tabs")
            .fill_width()
            .option(
                format!("Mine  {authored}"),
                tab == QueueTab::Authored,
                show_tab(QueueTab::Authored),
            )
            .option(
                format!("Review  {review_requested}"),
                tab == QueueTab::ReviewRequested,
                show_tab(QueueTab::ReviewRequested),
            )
            .option(
                format!("Others  {others}"),
                tab == QueueTab::Others,
                show_tab(QueueTab::Others),
            )
    }

    /// The refresh state, an opening stack layer, or why one did not open.
    fn render_status_line(&self) -> Option<AnyElement> {
        if let Some(number) = self.opening_layer() {
            return Some(
                PulsingStatus::new("opening-layer", format!("Opening #{number} from GitHub…"))
                    .into_any_element(),
            );
        }
        if let Some(error) = self.layer_error() {
            return Some(render_error_status(error.clone()));
        }
        // Others' pull requests can fail alone (a watched repository gone or renamed).
        if self.tab() == QueueTab::Others
            && matches!(self.refresh_status(), RefreshStatus::Idle)
            && let Some(error) = self.others_tab().error()
        {
            return Some(render_error_status(format!("Others: {error}").into()));
        }
        match self.refresh_status() {
            RefreshStatus::Idle => None,
            RefreshStatus::Refreshing => {
                Some(PulsingStatus::new("refreshing", "Refreshing from GitHub…").into_any_element())
            }
            RefreshStatus::Failed(message) => Some(render_error_status(message.clone())),
        }
    }

    /// Row `index` of the list; an empty element when it is gone.
    fn render_queue_row(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        self.render_known_queue_row(index, cx)
            .unwrap_or_else(|| div().into_any_element())
    }

    /// Row `index`, inset from the sidebar's edges; `None` when the row or
    /// what it shows is gone.
    fn render_known_queue_row(&self, index: usize, cx: &mut Context<Self>) -> Option<AnyElement> {
        let selected_id = self.selected_pull_request().map(|selected| selected.id);
        let now = unix_now();
        let row = self.rows().get(index)?;
        let element = match row {
            QueueRow::RepoHeader(group) => render_repo_header(index, group, cx),
            QueueRow::OthersFilter(repo) => self.render_others_filter(repo, cx),
            QueueRow::NothingShown(repo) => self.render_nothing_shown(repo, cx),
            QueueRow::PullRequest(list_index) => {
                let pull_request = self.shown_list().get(*list_index)?;
                render_pull_request_row(
                    pull_request,
                    *list_index,
                    selected_id == Some(pull_request.id),
                    self.has_brief(&pull_request.repo, pull_request.number),
                    now,
                    cx,
                )
            }
            QueueRow::StackLayer(list_index) => {
                let list_index = *list_index;
                let pull_request = self.shown_list().get(list_index)?;
                let row = self.listed_layer_row(pull_request, list_index, selected_id);
                let on_click = on_layer_click(
                    pull_request.repo.clone(),
                    pull_request.number,
                    move |queue, cx| queue.select(list_index, cx),
                    cx,
                );
                render_layer_row(row, on_click)
            }
            QueueRow::OtherLayer { key, layer } => {
                let stack_layer = self.stack(key)?.layers.get(*layer)?;
                let row = self.other_layer_row(index, &key.repo, *layer, stack_layer);
                let (open_key, layer) = (key.clone(), *layer);
                let on_click = on_layer_click(
                    key.repo.clone(),
                    stack_layer.number,
                    move |queue, cx| queue.open_other_layer(&open_key, layer, cx),
                    cx,
                );
                render_layer_row(row, on_click)
            }
            QueueRow::StackHeader(group) => {
                let is_reviewed = self.is_stack_reviewed(&group.key, cx);
                render_stack_header(index, group, self.tab(), is_reviewed, cx)
            }
        };
        // Inset from the sidebar's edges; the row keeps the full width inside
        // (`w_full`: the list's width bounds it, so wrapping content such as
        // filter chips wraps instead of widening the row).
        // A repository after the first starts below a divider.
        let starts_later_group = index > 0 && matches!(row, QueueRow::RepoHeader(_));
        Some(
            div()
                .w_full()
                .px_2()
                .pb_0p5()
                .when(starts_later_group, |row| {
                    row.pt_2()
                        .child(div().mx_2().mb_2().h(px(1.)).bg(theme::border()))
                })
                .child(element)
                .into_any_element(),
        )
    }
}

/// A failure in the header.
fn render_error_status(text: SharedString) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme::danger())
        .child(text)
        .into_any_element()
}

/// A row's frame (placed inset by the list): rounded corners, a hover tint
/// and the selection.
pub(super) fn row_frame(
    id: impl Into<gpui::ElementId>,
    is_selected: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .px_2()
        .py_2()
        .rounded_lg()
        .cursor_pointer()
        .when(is_selected, |row| row.bg(theme::selected()))
        .when(!is_selected, |row| {
            row.hover(|style| style.bg(theme::hover()))
        })
}

/// A row's second line: small and quiet.
pub(super) fn render_meta(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .min_w_0()
        .text_xs()
        .text_color(theme::subtle())
        .truncate()
        .child(text.into())
}

/// A repository's header: its monogram, name and owner, a pill with how
/// many pull requests it has here, and a chevron. Clicking it folds or
/// unfolds the repository's group.
fn render_repo_header(
    row_index: usize,
    group: &RepoGroup,
    cx: &mut Context<ReviewQueue>,
) -> AnyElement {
    let repo = group.repo.clone();
    div()
        .id(("repo-header", row_index))
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1p5()
        .rounded_lg()
        .cursor_pointer()
        .hover(|style| style.bg(theme::hover()))
        .on_click(cx.listener(move |queue, _, _, cx| queue.toggle_repository_group(&repo, cx)))
        .child(RepoMonogram::new(group.repo.name.clone()))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_baseline()
                .gap_1p5()
                .child(
                    div()
                        .flex_none()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(group.repo.name.clone()),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_xs()
                        .text_color(theme::subtle())
                        .child(group.repo.owner.clone()),
                ),
        )
        .child(
            div()
                .flex_none()
                .px_1p5()
                .rounded_full()
                .bg(theme::surface())
                .text_xs()
                .text_color(theme::muted())
                .child(repo_count_text(group)),
        )
        .children(group.others.as_ref().map(|others| {
            render_filter_button(
                row_index,
                &group.repo,
                others.is_filter_active,
                others.is_filter_open,
                cx,
            )
        }))
        .child(FoldChevron::new(
            ("repo-fold", row_index),
            FoldState::of(group.is_collapsed),
        ))
        .into_any_element()
}

/// A repository header's count: in the Others tab, `4 / 12` while its
/// filter hides some; otherwise how many it shows.
fn repo_count_text(group: &RepoGroup) -> String {
    match &group.others {
        Some(others) if others.total != group.pull_requests => {
            format!("{} / {}", group.pull_requests, others.total)
        }
        Some(_) | None => group.pull_requests.to_string(),
    }
}
