use std::time::Instant;

use gpui::{
    AnyElement, App, AvailableSpace, Context, Div, Entity, FontWeight, IntoElement, Styled, Window,
    div, prelude::*, px, size,
};
use qd_github::PullRequest;

use super::pull_request_detail::PullRequestDetail;
use crate::avatar_images::Avatar;
use crate::controls::{HoverExpand, RevealMotion};
use crate::markdown_view::is_openable;
use crate::theme;

/// Room for the tabs and metadata in the same row.
const HEADER_HEIGHT: f32 = 48.;
/// The author's avatar in the header.
const HEADER_AVATAR_SIZE: f32 = 16.;
/// The title, a step above the interface's body text.
const HEADER_TITLE_SIZE: f32 = 15.;
/// Keep both hover targets usable when the header has to scroll.
const HEADER_FIELD_MIN_WIDTH: f32 = 80.;

/// The header fields that can expand into the tabs' space.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HeaderField {
    Title,
    Branch,
}

/// Keeps the center tabs in sync with the active hover target.
struct HeaderHover {
    field: HeaderField,
    is_hovered: bool,
    tabs: RevealMotion,
}

impl HeaderHover {
    fn new() -> Self {
        Self {
            field: HeaderField::Title,
            is_hovered: false,
            tabs: RevealMotion::new(true),
        }
    }

    /// Ignore a late leave event from the field the pointer just left.
    fn on_hover(state: &Entity<Self>, field: HeaderField) -> impl Fn(bool, &mut App) + 'static {
        let state = state.clone();
        move |is_hovered, cx| {
            state.update(cx, |state, cx| {
                if is_hovered {
                    state.field = field;
                    state.is_hovered = true;
                } else if state.field == field {
                    state.is_hovered = false;
                }
                state.tabs.set_expanded(!state.is_hovered, Instant::now());
                cx.notify();
            });
        }
    }

    /// Shares room evenly at rest and gives the last hovered field the released space.
    fn weight(&self, field: HeaderField, tabs_visibility: f32) -> f32 {
        if self.field == field {
            2. - tabs_visibility
        } else {
            tabs_visibility
        }
    }

    /// Lets the active field grow beyond the pane when its full text needs a scroll.
    fn is_expanding(&self, field: HeaderField) -> bool {
        self.is_hovered && self.field == field
    }
}

impl PullRequestDetail {
    /// The pull request's identity, view tabs and metadata share one row
    /// above the progress line. `None` without a pull request.
    pub(super) fn render_sticky_header(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let pull_request = self.pull_request()?;
        let hover = window.use_keyed_state(("header-hover", pull_request.id), cx, |_, _| {
            HeaderHover::new()
        });
        let now = Instant::now();
        let state = hover.read(cx);
        let tabs_visibility = state.tabs.value(now);
        if state.tabs.is_active(now) {
            window.request_animation_frame();
        }
        let title_expands = state.is_expanding(HeaderField::Title);
        let branch_expands = state.is_expanding(HeaderField::Branch);
        let title = header_field_space(state.weight(HeaderField::Title, tabs_visibility))
            .debug_selector(|| "review-header-title-space".into())
            .when(!title_expands, |field| {
                field.min_w(px(HEADER_FIELD_MIN_WIDTH))
            })
            .child(Self::render_title_link(pull_request, &hover, title_expands));
        let branches = Self::render_branch_preview(pull_request, &hover, branch_expands);
        let branch = header_field_space(state.weight(HeaderField::Branch, tabs_visibility))
            .justify_end()
            .when_some(branches, |field, preview| {
                field
                    .when(!branch_expands, |field| {
                        field.min_w(px(HEADER_FIELD_MIN_WIDTH))
                    })
                    .child(preview)
            });
        // Keep the full controls reachable when both side panels leave a
        // narrow detail pane, including when the title or branches expand.
        let row = div()
            .id("detail-header-row")
            .flex()
            .w_full()
            .min_w_0()
            .overflow_x_scroll()
            .items_center()
            .gap_2()
            .h(px(HEADER_HEIGHT))
            .px_4()
            .text_xs()
            .text_color(theme::muted())
            .child(title)
            .child(self.render_header_tabs(tabs_visibility, window, cx))
            .child(branch);

        Some(
            div()
                .flex_none()
                .flex()
                .flex_col()
                .bg(theme::background())
                .border_b_1()
                .border_color(theme::border())
                .child(row)
                .children(self.render_progress_line())
                .into_any_element(),
        )
    }

    /// Fades and clips the tabs while releasing their width to the hovered field.
    fn render_header_tabs(
        &self,
        visibility: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let tabs = div()
            .debug_selector(|| "review-header-tabs".into())
            .flex_none();
        if visibility >= 1. {
            return tabs.child(self.render_mode_tabs(cx));
        }
        if visibility <= 0. {
            return tabs.w_0();
        }
        let width = self
            .render_mode_tabs(cx)
            .into_any_element()
            .layout_as_root(
                size(AvailableSpace::MaxContent, AvailableSpace::MaxContent),
                window,
                cx,
            )
            .width;
        tabs.w(width * visibility)
            .flex()
            .overflow_hidden()
            .opacity(visibility)
            .child(self.render_mode_tabs(cx))
    }

    /// Opens the pull request on click and reveals its full title and byline on hover.
    fn render_title_link(
        pull_request: &PullRequest,
        hover: &Entity<HeaderHover>,
        expands: bool,
    ) -> impl IntoElement {
        let url = pull_request.html_url.clone();
        let author = pull_request.author.clone();
        div()
            .id(("pull-request-title-link", pull_request.id))
            .debug_selector(|| "pull-request-title-link".into())
            .flex()
            .when(!expands, Styled::min_w_0)
            .text_size(px(HEADER_TITLE_SIZE))
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme::text())
            .when(is_openable(&url), |link| {
                link.cursor_pointer()
                    .hover(|style| style.text_color(theme::accent()))
                    .on_click(move |_, _, cx| cx.open_url(&url))
            })
            .child(
                HoverExpand::new(
                    ("pull-request-title", pull_request.id),
                    pull_request.title.clone().into(),
                    move || {
                        div()
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap_1()
                            .pl_2()
                            .text_xs()
                            .text_color(theme::muted())
                            .whitespace_nowrap()
                            .child("by")
                            .child(Avatar::new(
                                author.avatar_url.clone(),
                                px(HEADER_AVATAR_SIZE),
                            ))
                            .child(author.login.clone())
                    },
                )
                .on_hover(HeaderHover::on_hover(hover, HeaderField::Title)),
            )
    }

    /// Shows the head branch preview and reveals both branches on hover.
    fn render_branch_preview(
        pull_request: &PullRequest,
        hover: &Entity<HeaderHover>,
        expands: bool,
    ) -> Option<AnyElement> {
        let branches = pull_request.branches.as_ref()?;
        let base = format!("→ {}", branches.base_ref);
        Some(
            div()
                .debug_selector(|| "pull-request-branch-preview".into())
                .flex()
                .when(!expands, Styled::min_w_0)
                .px_2()
                .py_1()
                .rounded_md()
                .hover(|style| style.bg(theme::hover()).text_color(theme::text()))
                .child(
                    HoverExpand::new(
                        ("header-branches", pull_request.id),
                        branches.head_ref.clone().into(),
                        move || div().pl_1().whitespace_nowrap().child(base.clone()),
                    )
                    .on_hover(HeaderHover::on_hover(hover, HeaderField::Branch)),
                )
                .into_any_element(),
        )
    }
}

/// A flexible space beside the tabs, with the field kept at its natural width when it fits.
fn header_field_space(weight: f32) -> Div {
    let mut field = div().flex().flex_1().items_center();
    field.style().flex_grow = Some(weight);
    field
}
