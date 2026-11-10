use gpui::{App, Context};
use qd_cache::{DetailScrollPosition, ReviewQueueViewState};
use qd_github::PullRequest;

use super::ReviewQueue;
use super::restore_phase::RestorePhase;
use super::selection_origin::SelectionOrigin;
use super::selection_restore::SelectionRestore;
use crate::ui_state::UiStateRecorder;

/// A selection saved before the last relaunch, not yet matched to the lists.
pub struct PendingRestore {
    pub pull_request_id: u64,
    pub detail_scroll: DetailScrollPosition,
}

impl ReviewQueue {
    /// Selects the pull request of the shown list at `index` for the user:
    /// shows it from the top and records the new selection.
    pub(super) fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(pull_request) = self.shown_list().get(index).cloned() else {
            return;
        };
        self.select_pull_request(pull_request, SelectionOrigin::Listed, cx);
    }

    /// Selects `pull_request` for the user, unless it is already selected.
    pub(super) fn select_pull_request(
        &mut self,
        pull_request: PullRequest,
        origin: SelectionOrigin,
        cx: &mut Context<Self>,
    ) {
        if self.selected.as_ref().map(|selected| selected.id) == Some(pull_request.id) {
            return;
        }
        // A choice by the user replaces whatever was being restored.
        self.pending_restore = None;
        self.opening_layer = None;
        self.layer_error = None;
        self.set_selected(Some(pull_request), origin, None, cx);
        self.scroll_to_selection();
        self.record_view_state(cx);
    }

    /// Shows `pull_request` (or nothing) in the detail pane, at `restore`
    /// once its files load when given.
    pub(super) fn set_selected(
        &mut self,
        pull_request: Option<PullRequest>,
        origin: SelectionOrigin,
        restore: Option<DetailScrollPosition>,
        cx: &mut Context<Self>,
    ) {
        self.selected.clone_from(&pull_request);
        self.selection_origin = origin;
        self.detail()
            .update(cx, |detail, cx| detail.show(pull_request, restore, cx));
        cx.notify();
    }

    /// Matches the selection saved before the relaunch: in the lists, or
    /// at launch also among cached pull requests in no list (a stack layer
    /// opened from the map). Selects it with its scroll position, keeps
    /// waiting for the refreshed lists, or drops it.
    pub(super) fn restore_selection(&mut self, phase: RestorePhase, cx: &mut Context<Self>) {
        let Some(pending) = &self.pending_restore else {
            return;
        };
        let listed = self.lists.find_by_id(pending.pull_request_id).cloned();
        let (found, origin) = match (listed, phase) {
            (Some(listed), _) => (Some(listed), SelectionOrigin::Listed),
            (None, RestorePhase::CachedQueue) => (
                self.services
                    .cache
                    .lock()
                    .pull_request(pending.pull_request_id)
                    .ok()
                    .flatten(),
                SelectionOrigin::StackLayer,
            ),
            (None, RestorePhase::RefreshedQueue) => (None, SelectionOrigin::Listed),
        };
        match SelectionRestore::decide(found.as_ref(), phase) {
            SelectionRestore::Select(pull_request) => {
                // The detail scroll waits for the files; the list scrolls to the row now.
                let pull_request = pull_request.clone();
                let restore = Some(pending.detail_scroll);
                self.pending_restore = None;
                self.set_selected(Some(pull_request), origin, restore, cx);
                self.scroll_to_selection();
            }
            SelectionRestore::WaitForRefresh => {}
            SelectionRestore::Abandon => {
                self.pending_restore = None;
                self.record_view_state(cx);
            }
        }
    }

    /// Records the tab, selection and detail scroll as UI state. A
    /// selection or scroll still waiting to be restored is recorded as saved,
    /// so a relaunch before it resolves does not lose it.
    pub fn record_view_state(&self, cx: &mut App) {
        let selected_pull_request_id = self
            .selected
            .as_ref()
            .map(|pull_request| pull_request.id)
            .or(self
                .pending_restore
                .as_ref()
                .map(|pending| pending.pull_request_id));
        let detail_scroll = self.pending_restore.as_ref().map_or_else(
            || self.detail().read(cx).scroll_position(),
            |pending| pending.detail_scroll,
        );
        UiStateRecorder::record_review_queue_view_state(
            cx,
            ReviewQueueViewState {
                selected_pull_request_id,
                detail_scroll,
                tab: self.tab(),
                sidebar: self.sidebar_visibility(),
            },
        );
    }

    /// Scrolls the sidebar to the selected pull request's row, if the shown tab has it.
    pub(super) fn scroll_to_selection(&self) {
        let Some(selected_id) = self.selected.as_ref().map(|selected| selected.id) else {
            return;
        };
        let list = self.shown_list();
        let row = self.rows().iter().position(|row| {
            row.pull_request_index().is_some_and(|index| {
                list.get(index)
                    .is_some_and(|pull_request| pull_request.id == selected_id)
            })
        });
        if let Some(row) = row {
            self.queue_list().scroll_to_reveal_item(row);
        }
    }
}
