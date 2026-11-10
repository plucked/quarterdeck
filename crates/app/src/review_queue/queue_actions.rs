use gpui::{Context, Window};
use qd_cache::QueueTab;

use super::ReviewQueue;
use super::selection_origin::SelectionOrigin;
use crate::app_commands::{
    MarkViewedAndNext, NextFile, NextLayer, PreviousFile, PreviousLayer, Refresh, ShowAuthored,
    ShowOthers, ShowReviewRequested, ToggleFileTree, ToggleSidebar, ToggleStackReview,
};
use crate::pull_request_detail::{DetailEvent, PullRequestDetail};

impl ReviewQueue {
    /// Handles the detail pane's events: its scrolling (saved), a layer it
    /// opens, a pull request it merged or closed, a changed review target
    /// (the sidebar's stack highlight), a stored brief (the sidebar's marks).
    pub(super) fn on_detail_event(&mut self, event: &DetailEvent, cx: &mut Context<Self>) {
        match event {
            DetailEvent::Scrolled => self.record_view_state(cx),
            DetailEvent::OpenLayer(request) => self.open_layer(request, cx),
            DetailEvent::PullRequestChanged => {
                // Keep it in view after the refresh drops it from the lists.
                self.selection_origin = SelectionOrigin::Finished;
                self.refresh(cx);
            }
            DetailEvent::TargetChanged => cx.notify(),
            DetailEvent::BriefStored => self.load_brief_index(cx),
        }
    }

    /// Handles the Refresh action (cmd-r, View menu).
    pub(super) fn on_refresh(&mut self, _: &Refresh, _: &mut Window, cx: &mut Context<Self>) {
        self.refresh(cx);
    }

    /// Handles the `ShowAuthored` action (cmd-1): shows the viewer's own pull requests.
    pub(super) fn on_show_authored(
        &mut self,
        _: &ShowAuthored,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_tab(QueueTab::Authored, cx);
    }

    /// Handles the `ShowReviewRequested` action (cmd-2): shows the review requests.
    pub(super) fn on_show_review_requested(
        &mut self,
        _: &ShowReviewRequested,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_tab(QueueTab::ReviewRequested, cx);
    }

    /// Handles the `ShowOthers` action (cmd-3): shows others' pull requests.
    pub(super) fn on_show_others(
        &mut self,
        _: &ShowOthers,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.show_tab(QueueTab::Others, cx);
    }

    /// Handles the `NextFile` action (`]`): scrolls the detail to the next shown file.
    pub(super) fn on_next_file(&mut self, _: &NextFile, _: &mut Window, cx: &mut Context<Self>) {
        self.detail()
            .update(cx, PullRequestDetail::reveal_next_file);
    }

    /// Handles the `PreviousFile` action (`[`): scrolls the detail to the previous shown file.
    pub(super) fn on_previous_file(
        &mut self,
        _: &PreviousFile,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.detail()
            .update(cx, PullRequestDetail::reveal_previous_file);
    }

    /// Handles the `MarkViewedAndNext` action (`v`): marks the file at the
    /// top viewed and moves to the next unviewed one.
    pub(super) fn on_mark_viewed_and_next(
        &mut self,
        _: &MarkViewedAndNext,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.detail()
            .update(cx, PullRequestDetail::mark_current_viewed_and_advance);
    }

    /// Handles the `ToggleSidebar` action (ctrl-cmd-s): shows or hides the lists.
    pub(super) fn on_toggle_sidebar(
        &mut self,
        _: &ToggleSidebar,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_sidebar(cx);
    }

    /// Handles the `ToggleFileTree` action (cmd-b): shows or hides the side panel with the file tree.
    pub(super) fn on_toggle_file_tree(
        &mut self,
        _: &ToggleFileTree,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.detail()
            .update(cx, PullRequestDetail::toggle_file_tree);
    }

    /// Handles the `NextLayer` action (`}`): shows the stack layer above.
    pub(super) fn on_next_layer(&mut self, _: &NextLayer, _: &mut Window, cx: &mut Context<Self>) {
        self.detail().update(cx, PullRequestDetail::open_next_layer);
    }

    /// Handles the `PreviousLayer` action (`{`): shows the stack layer below.
    pub(super) fn on_previous_layer(
        &mut self,
        _: &PreviousLayer,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.detail()
            .update(cx, PullRequestDetail::open_previous_layer);
    }

    /// Handles the `ToggleStackReview` action (`c`): one layer or combined layers.
    pub(super) fn on_toggle_stack_review(
        &mut self,
        _: &ToggleStackReview,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.detail()
            .update(cx, PullRequestDetail::toggle_review_mode);
    }
}
