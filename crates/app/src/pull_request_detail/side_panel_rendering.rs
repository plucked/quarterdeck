use gpui::{AnyElement, Context, IntoElement, div, prelude::*, px};

use super::pull_request_detail::PullRequestDetail;
use super::side_panel_tab::SidePanelTab;
use crate::controls::{ResizeEdge, ResizeHandle, SegmentedControl, view_option};
use crate::theme;

/// How narrow and how wide the side panel can be dragged.
const MIN_PANEL_WIDTH: f32 = 220.;
const MAX_PANEL_WIDTH: f32 = 640.;

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
            .bg(theme::panel())
            .text_sm()
            .child(self.render_side_panel_tabs(cx))
            .map(|panel| match self.review().tab {
                SidePanelTab::Files if has_files => panel
                    .child(self.render_tree_header(cx))
                    .child(self.render_tree_body(cx)),
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
        let review_label = if count == 0 {
            "Review".to_owned()
        } else {
            format!("Review  {count}")
        };
        let detail = cx.entity().downgrade();
        let show = |tab| view_option(&detail, tab, PullRequestDetail::show_side_panel_tab);
        div().flex_none().px_3().pt_3().pb_1().child(
            SegmentedControl::new("side-panel-tabs")
                .option(
                    "Files",
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
                ),
        )
    }
}
