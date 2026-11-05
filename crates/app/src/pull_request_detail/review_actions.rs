use std::time::{SystemTime, UNIX_EPOCH};

use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui::{AppContext as _, ClipboardItem, Context, Window};
use qc_cache::{NoteAnchor, NoteDestination, NoteId, ReviewNote, ReviewScope, ReviewSummary};
use qc_github::{
    LineSide, NewReviewThread, NewThreadReply, ReviewEvent, ReviewSubmission, SubmittedPart,
};

use super::comment_editor::{CommentEditor, EditorTarget};
use super::comment_placement::CommentPlacement;
use super::detail_row::DetailRow;
use super::local_review_export::{ReviewExportHeader, local_review_markdown};
use super::pull_request_detail::PullRequestDetail;
use super::review_comments::{CommentCard, ReviewComments};
use super::review_status::ReviewStatus;
use super::review_target::ReviewTarget;
use super::side_panel_tab::SidePanelTab;
use crate::text_area::{TextArea, TextAreaEvent};
use crate::ui_state::{ReviewChange, UiStateRecorder};

/// The placeholder of a new comment's editor.
const COMMENT_PLACEHOLDER: &str = "Write a comment… (⌘↩ to save, esc to cancel)";

impl PullRequestDetail {
    /// Where new comments go now: the chosen destination for a single pull
    /// request; always local for a combined review (GitHub has no place for it).
    pub(super) fn active_destination(&self) -> NoteDestination {
        match self.target() {
            ReviewTarget::Layer => self.review().destination,
            ReviewTarget::Combined(_) | ReviewTarget::AwaitingStack => NoteDestination::Local,
        }
    }

    /// Whether the shown files can take GitHub comments (a single pull request).
    pub(super) fn can_review_on_github(&self) -> bool {
        *self.target() == ReviewTarget::Layer
    }

    /// Recomputes the cards for the active destination and where they and
    /// the composer go; the row rebuild that follows places them.
    pub(super) fn refresh_comment_layout(&mut self) {
        let destination = self.active_destination();
        let composer = match self.review().editor.as_ref().map(|editor| &editor.target) {
            Some(EditorTarget::NewComment {
                file, side, line, ..
            }) => Some((*file, *side, *line)),
            Some(_) | None => None,
        };
        let files: Vec<String> = self
            .loaded_files()
            .iter()
            .map(|loaded| loaded.filename().to_owned())
            .collect();
        let review = self.review_mut();
        review.comments.refresh_documents();
        review.cards = review.comments.cards(destination);
        review.placement = CommentPlacement::build(
            &review.cards,
            |path| files.iter().position(|file| file == path),
            composer,
        );
    }

    /// Starts over for newly loaded files: no comments, no editor.
    pub(super) fn reset_review(&mut self) {
        let review = self.review_mut();
        review.comments = ReviewComments::default();
        review.editor = None;
        review.load_task = None;
        review.action_task = None;
        review.status = ReviewStatus::Idle;
        review.summary_editor = None;
        review.summary_subscription = None;
    }

    /// Reads GitHub's conversations and the viewer's pending review for the
    /// shown pull request (one GraphQL request); combined reviews have none.
    pub(super) fn load_github_review(&mut self, cx: &mut Context<Self>) {
        let (Some(pull_request), true) =
            (self.pull_request().cloned(), self.can_review_on_github())
        else {
            return;
        };
        let Ok(client) = self.services().github.clone() else {
            return;
        };
        self.review_mut().comments.is_loading_github = true;
        cx.notify();
        let task = cx.spawn(async move |detail, cx| {
            let state = cx
                .background_executor()
                .spawn(async move { client.review_state(&pull_request.repo, pull_request.number) })
                .await;
            detail
                .update(cx, |detail, cx| {
                    let comments = &mut detail.review_mut().comments;
                    comments.is_loading_github = false;
                    match state {
                        Ok(state) => {
                            comments.github = Some(state);
                            comments.github_error = None;
                        }
                        Err(error) => comments.github_error = Some(format!("{error:#}").into()),
                    }
                    detail.rebuild_rows_in_place();
                    cx.notify();
                })
                .ok();
        });
        self.review_mut().load_task = Some(task);
    }

    /// Sends new comments to GitHub drafts or local notes, and saves it.
    pub(super) fn set_comment_destination(
        &mut self,
        destination: NoteDestination,
        cx: &mut Context<Self>,
    ) {
        if self.review().destination == destination {
            return;
        }
        self.review_mut().destination = destination;
        self.review_mut().editor = None;
        self.rebuild_rows_in_place();
        cx.notify();
    }

    /// Opens the editor of a new comment under line `line` of `side` in file
    /// `file`; with `extend` (shift-click) and an editor open on the same
    /// file and side, stretches that comment over the lines in between.
    pub(super) fn open_composer(
        &mut self,
        file: usize,
        side: LineSide,
        line: u32,
        extend: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self
            .loaded_files()
            .get(file)
            .map(|loaded| loaded.filename().to_owned())
        else {
            return;
        };

        // A shift-click on the same file and side makes the open comment span the lines.
        if extend
            && let Some(editor) = &mut self.review_mut().editor
            && let EditorTarget::NewComment {
                file: open_file,
                side: open_side,
                line: open_line,
                start_line,
                ..
            } = &mut editor.target
            && *open_file == file
            && *open_side == side
        {
            let first = start_line.unwrap_or(*open_line).min(line);
            let last = (*open_line).max(line);
            *start_line = (first != last).then_some(first);
            *open_line = last;
            self.rebuild_rows_in_place();
            cx.notify();
            return;
        }
        self.open_editor(
            EditorTarget::NewComment {
                file,
                path,
                side,
                line,
                start_line: None,
            },
            "",
            window,
            cx,
        );
    }

    /// Opens the editor for `target` with `text`, focused, replacing any open one.
    pub(super) fn open_editor(
        &mut self,
        target: EditorTarget,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let area = cx.new(|cx| {
            let mut area = TextArea::new(COMMENT_PLACEHOLDER, cx);
            area.set_text(text, cx);
            area
        });
        let subscription = cx.subscribe_in(
            &area,
            window,
            |detail, _, event: &TextAreaEvent, window, cx| match event {
                TextAreaEvent::Submit => detail.save_editor(window, cx),
                TextAreaEvent::Cancel => detail.close_editor(window, cx),
                TextAreaEvent::Changed => detail.remeasure_editor_row(),
            },
        );
        area.read(cx).focus(window);
        self.review_mut().editor = Some(CommentEditor {
            target,
            text: area,
            _subscription: subscription,
        });
        self.rebuild_rows_in_place();
        cx.notify();
    }

    /// Lets the list measure the editor's row again as its text grows.
    fn remeasure_editor_row(&mut self) {
        let Some(target) = self
            .review()
            .editor
            .as_ref()
            .map(|editor| editor.target.clone())
        else {
            return;
        };
        let row = (0..self.rows().len()).find(|row| match (self.rows().get(*row), &target) {
            (Some(DetailRow::Composer { .. }), EditorTarget::NewComment { .. }) => true,
            (Some(DetailRow::CommentCard { card, .. }), _) => self.card_holds_editor(card, &target),
            _ => false,
        });
        if let Some(row) = row {
            self.remeasure_row(row);
        }
    }

    /// Whether card `card` shows the editor for `target` (a reply or an edit in it).
    pub(super) fn card_holds_editor(&self, card: usize, target: &EditorTarget) -> bool {
        let Some(placed) = self.review().cards.get(card) else {
            return false;
        };
        match (&placed.card, target) {
            (CommentCard::Note(id), EditorTarget::EditNote(edited)) => id == edited,
            (CommentCard::Thread { thread, replies }, target) => {
                let Some(thread) = self
                    .review()
                    .comments
                    .github
                    .as_ref()
                    .and_then(|state| state.threads.get(*thread))
                else {
                    return false;
                };
                match target {
                    EditorTarget::Reply { thread_id, .. } => *thread_id == thread.id,
                    EditorTarget::EditGitHubComment { comment_id } => thread
                        .comments
                        .iter()
                        .any(|comment| comment.id == *comment_id),
                    EditorTarget::EditNote(edited) => replies.contains(edited),
                    EditorTarget::NewComment { .. } => false,
                }
            }
            (CommentCard::Note(_), _) => false,
        }
    }

    /// Closes the editor without saving; the diff takes the keys again.
    pub(super) fn close_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(self.focus());
        if self.review_mut().editor.take().is_some() {
            self.rebuild_rows_in_place();
            cx.notify();
        }
    }

    /// Saves what the editor holds: a new note or draft, an edited one, or
    /// an edit of a GitHub comment (sent at once). An empty text saves nothing.
    pub(super) fn save_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(self.focus());
        let Some(editor) = self.review_mut().editor.take() else {
            return;
        };
        let body = editor.text.read(cx).text().trim().to_owned();
        if body.is_empty() {
            self.rebuild_rows_in_place();
            cx.notify();
            return;
        }
        let now = unix_seconds();
        match editor.target {
            EditorTarget::NewComment {
                path,
                side,
                line,
                start_line,
                ..
            } => self.add_note(
                path,
                NoteAnchor::Line {
                    side,
                    line,
                    start_line,
                },
                self.active_destination(),
                body,
                cx,
            ),
            EditorTarget::Reply { thread_id, path } => self.add_note(
                path,
                NoteAnchor::Reply { thread_id },
                NoteDestination::GitHub,
                body,
                cx,
            ),
            EditorTarget::EditNote(id) => {
                let edited = self
                    .review_mut()
                    .comments
                    .notes
                    .iter_mut()
                    .find(|note| note.id == id)
                    .map(|note| {
                        note.body = body;
                        note.updated_at = now;
                        note.clone()
                    });
                if let Some(note) = edited {
                    UiStateRecorder::record_review_change(cx, ReviewChange::PutNote(note));
                }
            }
            EditorTarget::EditGitHubComment { comment_id } => {
                self.run_github_action("Saving the comment on GitHub…", cx, move |client| {
                    client.update_review_comment(&comment_id, &body)
                });
            }
        }
        self.rebuild_rows_in_place();
        cx.notify();
    }

    /// Adds a note of the shown scope and saves it.
    fn add_note(
        &mut self,
        path: String,
        anchor: NoteAnchor,
        destination: NoteDestination,
        body: String,
        cx: &mut Context<Self>,
    ) {
        let Some(scope) = self.review_scope().cloned() else {
            return;
        };
        let now = unix_seconds();
        let note = ReviewNote {
            id: NoteId::generate(),
            destination,
            scope,
            path,
            anchor,
            body,
            created_at: now,
            updated_at: now,
        };
        self.review_mut().comments.notes.push(note.clone());
        UiStateRecorder::record_review_change(cx, ReviewChange::PutNote(note));
    }

    /// Deletes one of the user's notes.
    pub(super) fn delete_note(&mut self, id: NoteId, cx: &mut Context<Self>) {
        self.review_mut()
            .comments
            .notes
            .retain(|note| note.id != id);
        UiStateRecorder::record_review_change(cx, ReviewChange::DeleteNote(id));
        self.rebuild_rows_in_place();
        cx.notify();
    }

    /// Deletes every local note of the shown scope.
    pub(super) fn clear_local_notes(&mut self, cx: &mut Context<Self>) {
        let ids: Vec<NoteId> = self
            .review()
            .comments
            .notes_to(NoteDestination::Local)
            .map(|note| note.id)
            .collect();
        for id in ids {
            self.review_mut()
                .comments
                .notes
                .retain(|note| note.id != id);
            UiStateRecorder::record_review_change(cx, ReviewChange::DeleteNote(id));
        }
        self.rebuild_rows_in_place();
        cx.notify();
    }

    /// Deletes a GitHub comment the viewer may delete (at once).
    pub(super) fn delete_github_comment(&mut self, comment_id: String, cx: &mut Context<Self>) {
        self.run_github_action("Deleting the comment on GitHub…", cx, move |client| {
            client.delete_review_comment(&comment_id)
        });
    }

    /// Resolves a GitHub conversation, or unresolves it (at once).
    pub(super) fn set_thread_resolved(
        &mut self,
        thread_id: String,
        is_resolved: bool,
        cx: &mut Context<Self>,
    ) {
        let message = if is_resolved {
            "Resolving the conversation…"
        } else {
            "Reopening the conversation…"
        };
        self.run_github_action(message, cx, move |client| {
            client.set_thread_resolved(&thread_id, is_resolved)
        });
    }

    /// Discards the viewer's pending review on GitHub with its comments.
    pub(super) fn discard_pending_review(&mut self, cx: &mut Context<Self>) {
        let Some(review_id) = self
            .review()
            .comments
            .github
            .as_ref()
            .and_then(|state| state.pending_review_id.clone())
        else {
            return;
        };
        self.run_github_action("Discarding the pending review…", cx, move |client| {
            client.discard_pending_review(&review_id)
        });
    }

    /// Runs one GitHub call off the main thread, saying what it does, then
    /// reads the conversations again so the diff shows the result.
    fn run_github_action(
        &mut self,
        message: &'static str,
        cx: &mut Context<Self>,
        action: impl FnOnce(&qc_github::GitHubClient) -> Result<(), qc_github::GitHubError>
        + Send
        + 'static,
    ) {
        let Ok(client) = self.services().github.clone() else {
            self.review_mut().status = ReviewStatus::Failed("GitHub is not connected.".into());
            cx.notify();
            return;
        };
        self.review_mut().status = ReviewStatus::Working(message.into());
        cx.notify();
        let task = cx.spawn(async move |detail, cx| {
            let done = cx
                .background_executor()
                .spawn(async move { action(&client) })
                .await;
            detail
                .update(cx, |detail, cx| {
                    detail.review_mut().status = match done {
                        Ok(()) => ReviewStatus::Idle,
                        Err(error) => ReviewStatus::Failed(format!("{error:#}").into()),
                    };
                    detail.load_github_review(cx);
                    cx.notify();
                })
                .ok();
        });
        self.review_mut().action_task = Some(task);
    }

    /// Shows the files or the review in the side panel; the review tab of a
    /// single pull request gets its summary editor.
    pub(super) fn show_side_panel_tab(&mut self, tab: SidePanelTab, cx: &mut Context<Self>) {
        self.review_mut().tab = tab;
        if tab == SidePanelTab::Review && self.review().summary_editor.is_none() {
            let summary = self.review().comments.summary.body.clone();
            let area = cx.new(|cx| {
                let mut area = TextArea::new("Summary of your review (optional)", cx);
                area.set_text(summary, cx);
                area
            });
            let subscription = cx.subscribe(&area, |detail, area, event: &TextAreaEvent, cx| {
                if *event == TextAreaEvent::Changed {
                    let body = area.read(cx).text().to_owned();
                    detail.review_mut().comments.summary.body = body;
                    detail.record_summary(cx);
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

    /// Submits the GitHub review: the drafts and replies, the verdict and the
    /// summary. Drafts GitHub took are dropped as it takes them, so a retry
    /// after a failure does not post them twice.
    pub(super) fn submit_review(&mut self, cx: &mut Context<Self>) {
        let (Some(state), Ok(client)) = (
            self.review().comments.github.clone(),
            self.services().github.clone(),
        ) else {
            return;
        };
        let Some(ReviewScope::PullRequest(pull_request_id)) = self.review_scope().cloned() else {
            return;
        };

        let (mut submission, thread_notes, reply_notes) = self.collect_submission();
        // GitHub refuses approving or requesting changes on one's own pull request.
        if state.viewer_is_author {
            submission.event = ReviewEvent::Comment;
        }

        self.review_mut().status = ReviewStatus::Working("Submitting the review…".into());
        cx.notify();
        let task = cx.spawn(async move |detail, cx| {
            // Submit off the main thread, hearing which drafts GitHub took.
            let (taken_sender, mut taken) = mpsc::unbounded();
            let submitted = cx.background_executor().spawn(async move {
                client.submit_review(&state, &submission, &mut |part| {
                    taken_sender.unbounded_send(part).ok();
                })
            });
            while let Some(part) = taken.next().await {
                let id = match part {
                    SubmittedPart::Thread(index) => thread_notes.get(index).copied(),
                    SubmittedPart::Reply(index) => reply_notes.get(index).copied(),
                };
                let dropped = detail.update(cx, |detail, cx| {
                    if let Some(id) = id {
                        detail
                            .review_mut()
                            .comments
                            .notes
                            .retain(|note| note.id != id);
                        UiStateRecorder::record_review_change(cx, ReviewChange::DeleteNote(id));
                    }
                });
                if dropped.is_err() {
                    return;
                }
            }
            let submitted = submitted.await;

            // Say how it went and read the conversations again.
            detail
                .update(cx, |detail, cx| {
                    detail.review_mut().status = match submitted {
                        Ok(()) => {
                            detail.review_mut().comments.summary = ReviewSummary::default();
                            if let Some(area) = detail.review().summary_editor.clone() {
                                area.update(cx, |area, cx| area.set_text("", cx));
                            }
                            UiStateRecorder::record_review_change(
                                cx,
                                ReviewChange::DeleteSummary(pull_request_id),
                            );
                            ReviewStatus::Done("Review submitted.".into())
                        }
                        Err(error) => ReviewStatus::Failed(format!("{error:#}").into()),
                    };
                    detail.load_github_review(cx);
                    detail.rebuild_rows_in_place();
                    cx.notify();
                })
                .ok();
        });
        self.review_mut().action_task = Some(task);
    }

    /// The GitHub review to submit from the drafts and the summary, with
    /// the note ids of its threads and replies in order.
    fn collect_submission(&self) -> (ReviewSubmission, Vec<NoteId>, Vec<NoteId>) {
        // The drafts: new conversations and replies, with their note ids.
        let drafts: Vec<ReviewNote> = self
            .review()
            .comments
            .notes_to(NoteDestination::GitHub)
            .cloned()
            .collect();
        let mut threads = Vec::new();
        let mut thread_notes = Vec::new();
        let mut replies = Vec::new();
        let mut reply_notes = Vec::new();
        for note in drafts {
            match note.anchor {
                NoteAnchor::Line {
                    side,
                    line,
                    start_line,
                } => {
                    threads.push(NewReviewThread {
                        path: note.path,
                        side,
                        line,
                        start_line,
                        body: note.body,
                    });
                    thread_notes.push(note.id);
                }
                NoteAnchor::Reply { thread_id } => {
                    replies.push(NewThreadReply {
                        thread_id,
                        body: note.body,
                    });
                    reply_notes.push(note.id);
                }
            }
        }
        let summary: ReviewSummary = self.review().comments.summary.clone();
        let submission = ReviewSubmission {
            event: summary.event,
            body: summary.body.trim().to_owned(),
            threads,
            replies,
        };
        (submission, thread_notes, reply_notes)
    }

    /// Copies the local notes as Markdown for an agent.
    pub(super) fn copy_local_review(&mut self, cx: &mut Context<Self>) {
        let Some(pull_request) = self.pull_request() else {
            return;
        };
        let header = match self.target() {
            ReviewTarget::Combined(range) => {
                let stack_number = pull_request
                    .stack
                    .as_ref()
                    .map_or(0, |membership| membership.number);
                ReviewExportHeader {
                    repository: range.repo.full_name(),
                    subject: format!(
                        "stack #{stack_number}, layers {}–{} combined (#{} to #{})",
                        range.first_index + 1,
                        range.last_index + 1,
                        range.first_number,
                        range.last_number
                    ),
                    branches: format!(
                        "base {} → head {}",
                        short_sha(&range.base_sha),
                        short_sha(&range.head_sha)
                    ),
                }
            }
            ReviewTarget::Layer | ReviewTarget::AwaitingStack => ReviewExportHeader {
                repository: pull_request.repo.full_name(),
                subject: format!(
                    "pull request #{}: {}",
                    pull_request.number, pull_request.title
                ),
                branches: pull_request.html_url.clone(),
            },
        };
        let notes: Vec<&ReviewNote> = self
            .review()
            .comments
            .notes_to(NoteDestination::Local)
            .collect();
        let count = notes.len();
        let markdown = local_review_markdown(&header, &notes, self.loaded_files());
        cx.write_to_clipboard(ClipboardItem::new_string(markdown));
        let plural = if count == 1 { "" } else { "s" };
        self.review_mut().status =
            ReviewStatus::Done(format!("Copied {count} note{plural} for your agent.").into());
        cx.notify();
    }

    /// Scrolls the diff to the card of note `id`, expanding nothing.
    pub(super) fn reveal_note(&mut self, id: NoteId) {
        let card = self
            .review()
            .cards
            .iter()
            .position(|placed| match &placed.card {
                CommentCard::Note(note) => *note == id,
                CommentCard::Thread { replies, .. } => replies.contains(&id),
            });
        let Some(card) = card else {
            return;
        };
        let row = (0..self.rows().len()).find(|row| {
            matches!(self.rows().get(*row), Some(DetailRow::CommentCard { card: shown, .. }) if shown == card)
        });
        if let Some(row) = row {
            self.scroll_to_row(row.saturating_sub(2));
        }
    }
}

/// The current time as stored on notes.
fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_secs()).unwrap_or(i64::MAX)
        })
}

/// The first seven characters of a commit.
fn short_sha(sha: &str) -> &str {
    sha.get(..7).unwrap_or(sha)
}
