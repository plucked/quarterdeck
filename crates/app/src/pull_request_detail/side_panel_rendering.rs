use gpui::{AnyElement, Context, IntoElement, div, prelude::*, px};
use qd_cache::NoteDestination;

use super::action_panel::PanelAction;
use super::pull_request_detail::PullRequestDetail;
use super::side_panel_tab::SidePanelTab;
use crate::controls::{AnimatedReveal, ResizeEdge, ResizeHandle, SegmentedControl, view_option};
use crate::theme;

/// How narrow and how wide the side panel can be dragged.
const MIN_PANEL_WIDTH: f32 = 220.;
const MAX_PANEL_WIDTH: f32 = 640.;
/// The secondary selector and its bottom inset, matching the detail bubble.
const PANEL_OPTIONS_HEIGHT: f32 = 32.;
/// Wider panels have room for the detail header's larger tab spacing.
const LARGE_TABS_WIDTH: f32 = 360.;
/// Keep every tab readable at the narrowest panel width.
const REVIEW_COUNT_WIDTH: f32 = 300.;

impl PullRequestDetail {
    /// The panel beside the diff: the Files, Review, Actions and Checks tabs; Files
    /// holds progress, filter chips, and the files as a folder tree in diff
    /// order (a quiet line while they load, so the panel stays in place when
    /// another pull request is picked); Actions merges or closes; Checks
    /// shows the CI checks (`ChecksPanel`).
    pub(super) fn render_side_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_files = !self.loaded_files().is_empty();
        div()
            .flex_none()
            .w(px(self.side_panel_width()))
            .h_full()
            .flex()
            .flex_col()
            .bg(theme::background())
            .text_sm()
            .child(self.render_side_panel_tabs(cx))
            .map(|panel| match self.review().tab {
                SidePanelTab::Files if has_files => panel.child(self.render_focus_drawer(cx)),
                SidePanelTab::Files => panel.child(self.render_files_pending()),
                SidePanelTab::Review => panel.child(self.render_review_panel(cx)),
                SidePanelTab::Actions => panel.child(self.render_actions_tab(cx)),
                SidePanelTab::Checks => panel.child(self.checks().clone()),
            })
    }

    /// The Files tab before there are files: loading, failed (the diff
    /// column says why), or a pull request without changed files.
    fn render_files_pending(&self) -> AnyElement {
        let text = match self.load_progress() {
            Some(_) => "Loading files…",
            None if self.load_failure().is_some() => "The files could not be loaded.",
            None => "No changed files.",
        };
        div()
            .flex_1()
            .p_3()
            .text_xs()
            .text_color(theme::muted())
            .child(text)
            .into_any_element()
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

    /// The tabs atop the side panel: Files, Review (with its comment count),
    /// Actions and Checks.
    fn render_side_panel_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = self.review().tab;
        let count = self
            .review()
            .comments
            .notes_to(self.active_destination())
            .count();
        let review_label = if count == 0 || self.side_panel_width() < REVIEW_COUNT_WIDTH {
            "Review".to_owned()
        } else {
            format!("Review  {count}")
        };
        let detail = cx.entity().downgrade();
        let show = |tab| view_option(&detail, tab, PullRequestDetail::show_side_panel_tab);
        let tabs = SegmentedControl::new("side-panel-tabs")
            .quiet()
            .when(
                self.side_panel_width() >= LARGE_TABS_WIDTH,
                SegmentedControl::large,
            )
            .option(
                if self.focus_drawer.mode == qd_cache::FilesMode::Plan {
                    "Guide"
                } else {
                    "Files"
                },
                tab == SidePanelTab::Files,
                show(SidePanelTab::Files),
            )
            .option(
                review_label,
                tab == SidePanelTab::Review,
                show(SidePanelTab::Review),
            )
            .option(
                "Actions",
                tab == SidePanelTab::Actions,
                show(SidePanelTab::Actions),
            )
            .option(
                "Checks",
                tab == SidePanelTab::Checks,
                show(SidePanelTab::Checks),
            );
        let expanded = match tab {
            SidePanelTab::Review => self.can_review_on_github(),
            SidePanelTab::Actions => self.services().github.is_ok(),
            SidePanelTab::Files | SidePanelTab::Checks => false,
        };
        div()
            .flex_none()
            .flex()
            .justify_center()
            .px_2()
            .pt_3()
            .pb_2()
            .child(
                div()
                    .id("side-panel-bubble")
                    .flex_none()
                    .flex()
                    .flex_col()
                    .rounded(px(18.))
                    .bg(theme::panel())
                    .overflow_hidden()
                    .child(tabs)
                    .child(AnimatedReveal::new(
                        "side-panel-options-reveal",
                        expanded,
                        px(PANEL_OPTIONS_HEIGHT),
                        self.render_side_panel_options(cx),
                    )),
            )
    }

    /// Keeps the last secondary row mounted through its closing animation.
    /// Handlers ignore clicks as soon as the owning tab is no longer active.
    fn render_side_panel_options(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let options = match self.review().options_tab {
            SidePanelTab::Actions => self.render_action_options(cx),
            SidePanelTab::Files | SidePanelTab::Review | SidePanelTab::Checks => {
                self.render_review_options(cx)
            }
        };
        div().h_full().px_2().pb_1p5().child(options)
    }

    /// The review's destination, inside the same bubble as the primary tabs.
    fn render_review_options(&self, cx: &mut Context<Self>) -> SegmentedControl {
        let detail = cx.entity().downgrade();
        let destination = self.active_destination();
        let pick = |destination| {
            let detail = detail.clone();
            move |_: &mut gpui::Window, cx: &mut gpui::App| {
                detail
                    .update(cx, |detail, cx| {
                        if detail.review().tab == SidePanelTab::Review
                            && detail.can_review_on_github()
                        {
                            detail.set_comment_destination(destination, cx);
                        }
                    })
                    .ok();
            }
        };
        SegmentedControl::new("review-destination-tabs")
            .quiet()
            .fill_width()
            .option(
                "GitHub",
                destination == NoteDestination::GitHub,
                pick(NoteDestination::GitHub),
            )
            .option(
                "Local",
                destination == NoteDestination::Local,
                pick(NoteDestination::Local),
            )
    }

    /// Choosing an action only prepares it; execution still needs confirmation.
    fn render_action_options(&self, cx: &mut Context<Self>) -> SegmentedControl {
        let detail = cx.entity().downgrade();
        let selected = self.action_panel().map(|panel| panel.action);
        let pick = |action| {
            let detail = detail.clone();
            move |_: &mut gpui::Window, cx: &mut gpui::App| {
                detail
                    .update(cx, |detail, cx| {
                        if detail.review().tab == SidePanelTab::Actions
                            && !detail
                                .action_panel()
                                .is_some_and(|panel| panel.action == action)
                        {
                            detail.toggle_action_panel(action, cx);
                        }
                    })
                    .ok();
            }
        };
        SegmentedControl::new("action-tabs")
            .quiet()
            .fill_width()
            .option(
                "Merge",
                selected == Some(PanelAction::Merge),
                pick(PanelAction::Merge),
            )
            .option(
                "Close",
                selected == Some(PanelAction::Close),
                pick(PanelAction::Close),
            )
    }
}
