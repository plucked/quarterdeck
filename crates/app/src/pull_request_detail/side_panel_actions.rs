use gpui::{AppContext as _, Context};
use qd_cache::{NoteDestination, ReviewScope};
use qd_github::ReviewEvent;

use super::comment_editor::EditorTarget;
use super::pull_request_detail::PullRequestDetail;
use super::side_panel_tab::SidePanelTab;
use crate::text_area::{TextArea, TextAreaEvent};
use crate::ui_state::{ReviewChange, UiStateRecorder};

impl PullRequestDetail {
    /// Shows the files or the review in the side panel; the review tab of a
    /// single pull request gets its summary editor.
    pub(super) fn show_side_panel_tab(&mut self, tab: SidePanelTab, cx: &mut Context<Self>) {
        self.review_mut().tab = tab;
        if matches!(tab, SidePanelTab::Review | SidePanelTab::Actions) {
            self.review_mut().options_tab = tab;
        }
        self.sync_checks_visibility(cx);
        if tab == SidePanelTab::Review && self.review().summary_editor.is_none() {
            let summary = self.review().comments.summary.body.clone();
            let area = cx.new(|cx| {
                let mut area = TextArea::comment("Write a review summary…", cx);
                area.set_text(summary, cx);
                area.set_github_markdown(true, cx);
                area
            });
            let subscription = cx.subscribe(&area, |detail, area, event: &TextAreaEvent, cx| {
                if *event == TextAreaEvent::Changed {
                    let body = area.read(cx).text().to_owned();
                    detail.review_mut().comments.summary.body = body;
                    detail.record_summary(cx);
                }
                if matches!(
                    event,
                    TextAreaEvent::Changed
                        | TextAreaEvent::LayoutChanged
                        | TextAreaEvent::FormattingChanged
                ) {
                    cx.notify();
                }
            });
            self.review_mut().summary_editor = Some(area);
            self.review_mut().summary_subscription = Some(subscription);
        }
        cx.notify();
    }

    /// Picks the verdict of the GitHub review being written.
    pub(super) fn set_review_event(&mut self, event: ReviewEvent, cx: &mut Context<Self>) {
        self.review_mut().comments.summary.event = event;
        self.record_summary(cx);
        cx.notify();
    }

    /// Saves the summary and verdict of the shown pull request's review.
    fn record_summary(&mut self, cx: &mut Context<Self>) {
        if let Some(ReviewScope::PullRequest(pull_request_id)) = self.review_scope().cloned() {
            let summary = self.review().comments.summary.clone();
            UiStateRecorder::record_review_change(
                cx,
                ReviewChange::PutSummary(pull_request_id, summary),
            );
        }
    }

    /// Sends new comments to GitHub drafts or local notes (from the comment
    /// being written or the review tab), and keeps the choice for the next
    /// comments and launches. An open comment stays open.
    pub(super) fn set_comment_destination(
        &mut self,
        destination: NoteDestination,
        cx: &mut Context<Self>,
    ) {
        if self.review().destination == destination {
            return;
        }
        self.review_mut().destination = destination;
        if let Some(editor) = &self.review().editor
            && matches!(editor.target, EditorTarget::NewComment { .. })
        {
            let github = self.editor_destination(&editor.target) == NoteDestination::GitHub;
            editor
                .text
                .update(cx, |area, cx| area.set_github_markdown(github, cx));
        }
        self.remeasure_editor_row();
        UiStateRecorder::record_review_layout(cx, self.review_layout());
        cx.notify();
    }
}
