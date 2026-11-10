use gpui::{AnyElement, Context, FontWeight, SharedString, div, list, prelude::*, px};
use qd_cache::QueueTab;
use qd_github::{PullRequest, PullRequestBranches, StackLayer, ViewerReview};

use super::ReviewQueue;
use super::others_filter_rendering::{render_filter_button, render_others_empty_state};
use super::queue_row::{QueueRow, RepoGroup, StackGroup};
use super::refresh_status::RefreshStatus;
use crate::avatar_images::Avatar;
use crate::display_date::{relative_age, unix_now};
use crate::fold_chevron::{FoldChevron, FoldState};
use crate::pulsing_status::PulsingStatus;
use crate::repo_monogram::RepoMonogram;
use crate::resize_handle::{ResizeEdge, ResizeHandle};
use crate::review_verdict_badge::ReviewVerdictBadge;
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::segmented_control::SegmentedControl;
use crate::theme;
use crate::view_option::view_option;

/// How narrow and how wide the sidebar can be dragged.
const MIN_SIDEBAR_WIDTH: f32 = 240.;
const MAX_SIDEBAR_WIDTH: f32 = 720.;
/// Width of the column holding a stack layer's position and rail.
const LAYER_GUTTER_WIDTH: f32 = 28.;
/// The open / draft ring before a pull request's title.
const STATE_RING_SIZE: f32 = 10.;
/// Text size of the mark on a pull request with a stored brief.
const BRIEF_MARK_SIZE: f32 = 11.;
/// The stack glyph beside a stack's header.
const STACK_GLYPH_SIZE: f32 = 16.;
/// The author's avatar in a pull request row's second line.
const ROW_AVATAR_SIZE: f32 = 14.;
/// Element id of a row's review verdict badge (unique within its row).
const VERDICT_BADGE_ID: &str = "review-verdict";
/// Height of the rail mark beside a layer's position.
const LAYER_RAIL_HEIGHT: f32 = 16.;

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
                let pull_request = self.shown_list().get(*list_index)?;
                let list_index = *list_index;
                let (repo, number) = (pull_request.repo.clone(), pull_request.number);
                let draft_marker = if pull_request.draft { " · draft" } else { "" };
                render_layer_row(
                    LayerRow {
                        id: ("stack-layer", list_index).into(),
                        position: pull_request
                            .stack
                            .as_ref()
                            .map_or(0, |membership| membership.position),
                        title: pull_request.title.clone().into(),
                        meta: format!("#{}{draft_marker}", pull_request.number).into(),
                        is_listed: true,
                        marks: LayerMarks {
                            needs_review: self
                                .needs_review(&pull_request.repo, pull_request.number),
                            has_brief: self.has_brief(&pull_request.repo, pull_request.number),
                            viewer_review: pull_request.viewer_review,
                        },
                        is_selected: selected_id == Some(pull_request.id),
                    },
                    cx.listener(move |queue, event: &gpui::ClickEvent, _, cx| {
                        if !(event.modifiers().shift && queue.start_combined_at(&repo, number, cx))
                        {
                            queue.select(list_index, cx);
                        }
                    }),
                )
            }
            QueueRow::OtherLayer { key, layer } => {
                let stack_layer = self.stack(key)?.layers.get(*layer)?;
                let (key, layer) = (key.clone(), *layer);
                let number = stack_layer.number;
                render_layer_row(
                    LayerRow {
                        id: ("other-layer", index).into(),
                        position: u64::try_from(layer + 1).unwrap_or(u64::MAX),
                        title: stack_layer.title.clone().into(),
                        meta: format!("#{}{}", stack_layer.number, layer_state_marker(stack_layer))
                            .into(),
                        is_listed: false,
                        marks: LayerMarks {
                            needs_review: self.needs_review(&key.repo, stack_layer.number),
                            has_brief: self.has_brief(&key.repo, stack_layer.number),
                            viewer_review: self
                                .lists()
                                .find_by_number(&key.repo, stack_layer.number)
                                .and_then(|listed| listed.viewer_review),
                        },
                        is_selected: self.is_selected(&key.repo, stack_layer.number),
                    },
                    cx.listener(move |queue, event: &gpui::ClickEvent, _, cx| {
                        if !(event.modifiers().shift
                            && queue.start_combined_at(&key.repo, number, cx))
                        {
                            queue.open_other_layer(&key, layer, cx);
                        }
                    }),
                )
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

/// Whether a pull request is ready for review or still a draft, as its
/// sidebar row shows it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum PullRequestState {
    Open,
    Draft,
}

impl PullRequestState {
    fn of(pull_request: &PullRequest) -> Self {
        if pull_request.draft {
            Self::Draft
        } else {
            Self::Open
        }
    }

    /// The ring before the title: green when open, grey for a draft.
    fn ring_color(self) -> gpui::Rgba {
        match self {
            Self::Open => theme::success(),
            Self::Draft => theme::subtle(),
        }
    }

    /// A draft's title reads quieter.
    fn title_color(self) -> gpui::Rgba {
        match self {
            Self::Open => theme::text(),
            Self::Draft => theme::muted(),
        }
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
fn row_frame(id: impl Into<gpui::ElementId>, is_selected: bool) -> gpui::Stateful<gpui::Div> {
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
fn render_meta(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .min_w_0()
        .text_xs()
        .text_color(theme::subtle())
        .truncate()
        .child(text.into())
}

/// One standalone pull request: a state ring (open or draft), its title on
/// up to two lines, then `#number · author · age`, the viewer's review
/// verdict, a Draft pill, and a mark when it has a stored brief, then `base ← head` once its branches are
/// known; clicking selects it.
fn render_pull_request_row(
    pull_request: &PullRequest,
    list_index: usize,
    is_selected: bool,
    has_brief: bool,
    now: i64,
    cx: &mut Context<ReviewQueue>,
) -> AnyElement {
    let state = PullRequestState::of(pull_request);
    row_frame(("pull-request", list_index), is_selected)
        .flex()
        .gap_2p5()
        .on_click(cx.listener(move |queue, _, _, cx| queue.select(list_index, cx)))
        .child(
            div().flex_none().pt(px(4.)).child(
                div()
                    .size(px(STATE_RING_SIZE))
                    .rounded_full()
                    .border_2()
                    .border_color(state.ring_color()),
            ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(state.title_color())
                        .line_clamp(2)
                        .child(pull_request.title.clone()),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1p5()
                        .child(
                            div()
                                .flex_none()
                                .child(render_meta(format!("#{} ·", pull_request.number))),
                        )
                        .child(Avatar::new(
                            pull_request.author.avatar_url.clone(),
                            px(ROW_AVATAR_SIZE),
                        ))
                        .child(render_meta(format!(
                            "{} · {}",
                            pull_request.author.login,
                            relative_age(&pull_request.updated_at, now)
                        )))
                        .children(
                            pull_request
                                .viewer_review
                                .map(|review| ReviewVerdictBadge::new(VERDICT_BADGE_ID, review)),
                        )
                        .when(state == PullRequestState::Draft, |meta| {
                            meta.child(render_draft_pill())
                        })
                        .when(has_brief, |meta| meta.child(render_brief_mark())),
                )
                .children(pull_request.branches.as_ref().map(render_branches)),
        )
        .into_any_element()
}

/// A pull request's branches as `base ← head`, in the code font: the
/// branch it goes into, then the one it comes from. Each name truncates on
/// its own, so a long head branch leaves the base readable.
fn render_branches(branches: &PullRequestBranches) -> impl IntoElement {
    let branch_name = |name: &str| {
        div()
            .min_w_0()
            .truncate()
            .child(SharedString::from(name.to_owned()))
    };
    div()
        .flex()
        .items_center()
        .gap_1()
        .min_w_0()
        .font_family(theme::MONO_FONT)
        .text_xs()
        .text_color(theme::subtle())
        .child(branch_name(&branches.base_ref))
        .child(div().flex_none().child("←"))
        .child(branch_name(&branches.head_ref))
}

/// The mark of a pull request with a stored brief: a small star in the
/// accent color.
fn render_brief_mark() -> impl IntoElement {
    div()
        .flex_none()
        .text_size(px(BRIEF_MARK_SIZE))
        .text_color(theme::accent())
        .child("✦")
}

/// The pill marking a draft.
fn render_draft_pill() -> impl IntoElement {
    div()
        .flex_none()
        .px_1p5()
        .rounded_full()
        .border_1()
        .border_color(theme::border_strong())
        .text_size(px(10.))
        .text_color(theme::muted())
        .child("Draft")
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

/// The header of a stack's group: a stack glyph, its number and trunk,
/// how many layers it has and how many of them this list has, and a
/// chevron. Clicking opens the stack's combined review (highlighted while
/// shown); the chevron folds or unfolds the group.
fn render_stack_header(
    row_index: usize,
    group: &StackGroup,
    tab: QueueTab,
    is_reviewed: bool,
    cx: &mut Context<ReviewQueue>,
) -> AnyElement {
    let open_key = group.key.clone();
    let toggle_key = group.key.clone();
    let is_all = u64::try_from(group.listed).is_ok_and(|listed| listed == group.size);
    let listed = match (tab, is_all) {
        (QueueTab::Authored, true) => "all yours".to_owned(),
        (QueueTab::Authored, false) => format!("{} yours", group.listed),
        (QueueTab::ReviewRequested, true) => "all to review".to_owned(),
        (QueueTab::ReviewRequested, false) => format!("{} to review", group.listed),
        (QueueTab::Others, true) => "all listed".to_owned(),
        (QueueTab::Others, false) => format!("{} listed", group.listed),
    };
    row_frame(("stack-header", row_index), is_reviewed)
        .flex()
        .items_center()
        .gap_2p5()
        .on_click(cx.listener(move |queue, _, _, cx| queue.open_stack(&open_key, cx)))
        .child(render_stack_glyph())
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_0p5()
                .child(
                    div()
                        .flex()
                        .items_baseline()
                        .gap_1p5()
                        .text_sm()
                        .child(
                            div()
                                .flex_none()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!("Stack #{}", group.key.number)),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_xs()
                                .text_color(theme::muted())
                                .child(format!("→ {}", group.base_ref)),
                        ),
                )
                .child(render_meta(format!("{} layers · {listed}", group.size))),
        )
        .child(
            FoldChevron::new(("stack-fold", row_index), FoldState::of(group.is_collapsed))
                .on_click(cx.listener(move |queue, _, _, cx| {
                    queue.toggle_stack_group(&toggle_key, cx);
                })),
        )
        .into_any_element()
}

/// Two overlapping rounded squares: the mark of a stack.
fn render_stack_glyph() -> impl IntoElement {
    div()
        .flex_none()
        .relative()
        .size(px(STACK_GLYPH_SIZE))
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size(px(STACK_GLYPH_SIZE - 5.))
                .rounded(px(3.))
                .border_1()
                .border_color(theme::accent()),
        )
        .child(
            div()
                .absolute()
                .bottom_0()
                .right_0()
                .size(px(STACK_GLYPH_SIZE - 5.))
                .rounded(px(3.))
                .bg(theme::accent()),
        )
}

/// What one layer row under a stack's header shows.
struct LayerRow {
    id: gpui::ElementId,
    /// 1 for the layer on the trunk.
    position: u64,
    title: SharedString,
    meta: SharedString,
    /// Whether the shown list has it; other layers show muted.
    is_listed: bool,
    marks: LayerMarks,
    is_selected: bool,
}

/// What a layer row marks beside its title.
struct LayerMarks {
    /// It asks for the viewer's review (a badge says so).
    needs_review: bool,
    /// It has a stored brief (a star says so).
    has_brief: bool,
    /// The viewer's latest review (a badge says which), when known.
    viewer_review: Option<ViewerReview>,
}

/// One layer of a stack, under its header, on a single line so long stacks
/// stay compact: its position on a rail, the title, a Review badge when it
/// asks for the viewer's review, a star with a stored brief, the viewer's
/// review verdict, and `#number` (with its state) on the
/// right. Layers the list lacks are muted; clicking any opens it.
fn render_layer_row(
    row: LayerRow,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> AnyElement {
    let rail_color = if row.is_listed {
        theme::accent()
    } else {
        theme::border_strong()
    };
    row_frame(row.id, row.is_selected)
        .pl_1()
        .py_1()
        .flex()
        .items_center()
        .gap_2()
        .on_click(on_click)
        .child(
            div()
                .flex_none()
                .w(px(LAYER_GUTTER_WIDTH))
                .flex()
                .items_center()
                .justify_end()
                .gap_1p5()
                .text_xs()
                .text_color(theme::subtle())
                .child(row.position.to_string())
                .child(
                    div()
                        .w(px(2.))
                        .h(px(LAYER_RAIL_HEIGHT))
                        .rounded_full()
                        .bg(rail_color),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_sm()
                .truncate()
                .when(!row.is_listed, |title| title.text_color(theme::muted()))
                .child(row.title),
        )
        .when(row.marks.needs_review, |line| {
            line.child(render_review_badge())
        })
        .when(row.marks.has_brief, |line| line.child(render_brief_mark()))
        .children(
            row.marks
                .viewer_review
                .map(|review| ReviewVerdictBadge::new(VERDICT_BADGE_ID, review)),
        )
        .child(
            div()
                .flex_none()
                .text_xs()
                .text_color(theme::subtle())
                .child(row.meta),
        )
        .into_any_element()
}

/// The badge on a layer that asks for the viewer's review.
fn render_review_badge() -> impl IntoElement {
    div()
        .flex_none()
        .px_1p5()
        .rounded_full()
        .bg(theme::selected())
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme::accent())
        .child("Review")
}

/// ` · merged`, ` · closed` or ` · draft` for a layer not simply open.
fn layer_state_marker(layer: &StackLayer) -> &'static str {
    match (layer.is_merged, layer.is_open, layer.is_draft) {
        (true, _, _) => " · merged",
        (false, false, _) => " · closed",
        (false, true, true) => " · draft",
        (false, true, false) => "",
    }
}
