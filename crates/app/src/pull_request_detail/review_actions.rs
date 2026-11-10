use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui::{AppContext as _, ClipboardItem, Context, Window};
use qd_cache::{NoteAnchor, NoteDestination, NoteId, ReviewNote, ReviewScope, ReviewSummary};
use qd_github::{NewReviewThread, NewThreadReply, ReviewEvent, ReviewSubmission, SubmittedPart};

use super::comment_editor::{CommentEditor, EditorTarget};
use super::comment_placement::CommentPlacement;
use super::comment_range::CommentRange;
use super::comment_scope::CommentScope;
use super::composer_opening::ComposerOpening;
use super::detail_row::DetailRow;
use super::diff_line_ref::DiffLineRef;
use super::local_review_export::{ReviewExportHeader, local_review_markdown};
use super::operation_status::OperationStatus;
use super::pull_request_detail::PullRequestDetail;
use super::review_comments::{CommentCard, ReviewComments};
use super::review_target::ReviewTarget;
use super::side_panel_tab::SidePanelTab;
use super::thread_resolution::ThreadResolution;
use crate::avatar_images::AvatarImages;
use crate::display_date::unix_now;
use crate::short_sha::short_sha;
use crate::text_area::{TextArea, TextAreaEvent};
use crate::ui_state::{ReviewChange, UiStateRecorder};

/// The placeholder of a new comment's editor.
const COMMENT_PLACEHOLDER: &str = "Write a comment… (⌘↩ to save, esc to cancel)";

/// A GitHub review to submit, with the ids of the notes its new
/// conversations and replies come from, in order, so each draft GitHub
/// takes can be dropped as it is taken.
struct CollectedSubmission {
    submission: ReviewSubmission,
    thread_notes: Vec<NoteId>,
    reply_notes: Vec<NoteId>,
}

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

    /// Which comments the diff shows for the shown target.
    fn comment_scope(&self) -> CommentScope {
        match self.target() {
            ReviewTarget::Layer => CommentScope::GitHubAndLocal,
            ReviewTarget::Combined(_) | ReviewTarget::AwaitingStack => CommentScope::LocalOnly,
        }
    }

    /// Recomputes the cards for the active destination and where they and
    /// the composer go; the row rebuild that follows places them.
    pub(super) fn refresh_comment_layout(&mut self) {
        let composer = match self.review().editor.as_ref().map(|editor| &editor.target) {
            Some(EditorTarget::NewComment {
                file, side, line, ..
            }) => Some(DiffLineRef {
                file: *file,
                side: *side,
                line: *line,
            }),
            Some(_) | None => None,
        };
        let files: Vec<String> = self
            .loaded_files()
            .iter()
            .map(|loaded| loaded.filename().to_owned())
            .collect();
        let scope = self.comment_scope();
        let review = self.review_mut();
        review.comments.refresh_documents();
        review.cards = review.comments.cards(scope);
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
        review.status = OperationStatus::Idle;
        review.drop_summary_editor();
    }

    /// Reads GitHub's conversations and the viewer's pending review for the
    /// shown pull request (one GraphQL request); combined reviews have none.
    pub(super) fn load_github_review(&mut self, cx: &mut Context<Self>) {
        if !self.can_review_on_github() {
            return;
        }
        let Some(pull_request) = self.pull_request().cloned() else {
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
                            // The comment cards show their authors' avatars.
                            let avatars: Vec<String> = state
                                .threads
                                .iter()
                                .flat_map(|thread| &thread.comments)
                                .map(|comment| comment.author_avatar_url.clone())
                                .collect();
                            comments.github = Some(state);
                            comments.github_error = None;
                            AvatarImages::load(cx, avatars);
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
        UiStateRecorder::record_review_layout(cx, self.review_layout());
        cx.notify();
    }

    /// Opens the editor of a new comment under `at`; opened to extend
    /// (shift-click) with an editor open on the same file and side, stretches
    /// that comment over the lines in between instead.
    pub(super) fn open_composer(
        &mut self,
        at: DiffLineRef,
        opening: ComposerOpening,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let DiffLineRef { file, side, line } = at;
        let Some(path) = self
            .loaded_files()
            .get(file)
            .map(|loaded| loaded.filename().to_owned())
        else {
            return;
        };

        // A shift-click on the same file and side makes the open comment span the lines.
        if opening == ComposerOpening::ExtendOpen
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
            self.refresh_comment_files(&[Some(file)]);
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

        // Only the files of the editor closed and the one opened change rows.
        let closed_file = self
            .review()
            .editor
            .as_ref()
            .and_then(|editor| self.editor_file(&editor.target));
        let opened_file = self.editor_file(&target);
        self.review_mut().editor = Some(CommentEditor {
            target,
            text: area,
            _subscription: subscription,
        });
        self.refresh_comment_files(&[closed_file, opened_file]);
        cx.notify();
    }

    /// The loaded file at `path`.
    fn file_of_path(&self, path: &str) -> Option<usize> {
        self.loaded_files()
            .iter()
            .position(|loaded| loaded.filename() == path)
    }

    /// The file whose rows show the editor for `target`.
    fn editor_file(&self, target: &EditorTarget) -> Option<usize> {
        match target {
            EditorTarget::NewComment { file, .. } => Some(*file),
            EditorTarget::Reply { path, .. } => self.file_of_path(path),
            EditorTarget::EditNote(id) => self
                .review()
                .comments
                .note(*id)
                .and_then(|note| self.file_of_path(&note.path)),
            EditorTarget::EditGitHubComment { comment_id } => {
                let thread = self
                    .review()
                    .comments
                    .github
                    .as_ref()?
                    .threads
                    .iter()
                    .find(|thread| {
                        thread
                            .comments
                            .iter()
                            .any(|comment| comment.id == *comment_id)
                    })?;
                self.file_of_path(&thread.path)
            }
        }
    }

    /// Places the comments again and rebuilds the rows of `files` alone,
    /// keeping every other row and its measured height.
    fn refresh_comment_files(&mut self, files: &[Option<usize>]) {
        self.refresh_comment_layout();
        let mut rebuilt: Vec<usize> = Vec::new();
        for file in files.iter().flatten() {
            if !rebuilt.contains(file) {
                self.rebuild_file_rows(*file);
                rebuilt.push(*file);
            }
        }
    }

    /// The lines the comment being written or edited is about; `None`
    /// without one (or for a reply).
    pub(super) fn comment_range(&self) -> Option<CommentRange> {
        let editor = self.review().editor.as_ref()?;
        match &editor.target {
            EditorTarget::NewComment {
                file,
                side,
                line,
                start_line,
                ..
            } => Some(CommentRange::ending_at(*file, *side, *line, *start_line)),
            EditorTarget::EditNote(id) => {
                let note = self.review().comments.note(*id)?;
                let NoteAnchor::Line {
                    side,
                    line,
                    start_line,
                } = &note.anchor
                else {
                    return None;
                };
                let file = self.file_of_path(&note.path)?;
                Some(CommentRange::ending_at(file, *side, *line, *start_line))
            }
            EditorTarget::Reply { .. } | EditorTarget::EditGitHubComment { .. } => None,
        }
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
        let Some(rows) = self
            .editor_file(&target)
            .and_then(|file| self.rows().file_range(file))
        else {
            return;
        };
        let row = rows
            .into_iter()
            .find(|row| match (self.rows().get(*row), &target) {
                (Some(DetailRow::Composer { .. }), EditorTarget::NewComment { .. }) => true,
                (Some(DetailRow::CommentCard { card, .. }), _) => {
                    self.card_holds_editor(card, &target)
                }
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
        if let Some(editor) = self.review_mut().editor.take() {
            let file = self.editor_file(&editor.target);
            self.refresh_comment_files(&[file]);
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
        let file = self.editor_file(&editor.target);
        let body = editor.text.read(cx).text().trim().to_owned();
        if body.is_empty() {
            self.refresh_comment_files(&[file]);
            cx.notify();
            return;
        }
        let now = unix_now();
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
        self.refresh_comment_files(&[file]);
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
        let now = unix_now();
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
        let file = self
            .review()
            .comments
            .note(id)
            .and_then(|note| self.file_of_path(&note.path));
        self.forget_note(id, cx);
        self.refresh_comment_files(&[file]);
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
            self.forget_note(id, cx);
        }
        self.rebuild_rows_in_place();
        cx.notify();
    }

    /// Removes note `id` from the shown notes and from storage; the caller
    /// lays the rows out again.
    fn forget_note(&mut self, id: NoteId, cx: &mut Context<Self>) {
        self.review_mut()
            .comments
            .notes
            .retain(|note| note.id != id);
        UiStateRecorder::record_review_change(cx, ReviewChange::DeleteNote(id));
    }

    /// Deletes a GitHub comment the viewer may delete (at once).
    pub(super) fn delete_github_comment(&mut self, comment_id: String, cx: &mut Context<Self>) {
        self.run_github_action("Deleting the comment on GitHub…", cx, move |client| {
            client.delete_review_comment(&comment_id)
        });
    }

    /// Resolves a GitHub conversation, or reopens it (at once).
    pub(super) fn set_thread_resolution(
        &mut self,
        thread_id: String,
        resolution: ThreadResolution,
        cx: &mut Context<Self>,
    ) {
        let is_resolved = resolution.is_resolved_after();
        self.run_github_action(resolution.working_message(), cx, move |client| {
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
        action: impl FnOnce(&qd_github::GitHubClient) -> Result<(), qd_github::GitHubError>
        + Send
        + 'static,
    ) {
        let Ok(client) = self.services().github.clone() else {
            self.review_mut().status = OperationStatus::Failed("GitHub is not connected.".into());
            cx.notify();
            return;
        };
        self.review_mut().status = OperationStatus::Working(message.into());
        cx.notify();
        let task = cx.spawn(async move |detail, cx| {
            let done = cx
                .background_executor()
                .spawn(async move { action(&client) })
                .await;
            detail
                .update(cx, |detail, cx| {
                    detail.review_mut().status = match done {
                        Ok(()) => OperationStatus::Idle,
                        Err(error) => OperationStatus::Failed(format!("{error:#}").into()),
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
        self.sync_checks_visibility(cx);
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
    /// after a failure does not post them twice. Does nothing while a
    /// submission runs: its drafts are still there, and a second click would
    /// post them as a second review.
    pub(super) fn submit_review(&mut self, cx: &mut Context<Self>) {
        if self.review().submission.is_some() {
            return;
        }
        let (Some(state), Ok(client)) = (
            self.review().comments.github.clone(),
            self.services().github.clone(),
        ) else {
            return;
        };
        let Some(ReviewScope::PullRequest(pull_request_id)) = self.review_scope().cloned() else {
            return;
        };

        let CollectedSubmission {
            mut submission,
            thread_notes,
            reply_notes,
        } = self.collect_submission();
        // GitHub refuses approving or requesting changes on one's own pull request.
        if state.viewer_is_author {
            submission.event = ReviewEvent::Comment;
        }

        self.review_mut().status = OperationStatus::Working("Submitting the review…".into());
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
                        detail.forget_note(id, cx);
                    }
                });
                if dropped.is_err() {
                    return;
                }
            }
            let submitted = submitted.await;

            detail
                .update(cx, |detail, cx| {
                    // This task ends here: let go of its handle without cancelling it.
                    if let Some(task) = detail.review_mut().submission.take() {
                        task.detach();
                    }

                    // The saved summary of a submitted review goes, whatever shows now.
                    if submitted.is_ok() {
                        UiStateRecorder::record_review_change(
                            cx,
                            ReviewChange::DeleteSummary(pull_request_id),
                        );
                    }

                    // Another pull request or range shows now: its review stays as it is.
                    if detail.review_scope() != Some(&ReviewScope::PullRequest(pull_request_id)) {
                        cx.notify();
                        return;
                    }

                    // Say how it went and read the conversations again.
                    detail.review_mut().status = match submitted {
                        Ok(()) => {
                            detail.review_mut().comments.summary = ReviewSummary::default();
                            if let Some(area) = detail.review().summary_editor.clone() {
                                area.update(cx, |area, cx| area.set_text("", cx));
                            }
                            OperationStatus::Done("Review submitted.".into())
                        }
                        Err(error) => OperationStatus::Failed(format!("{error:#}").into()),
                    };
                    detail.load_github_review(cx);
                    detail.rebuild_rows_in_place();
                    cx.notify();
                })
                .ok();
        });
        self.review_mut().submission = Some(task);
        cx.notify();
    }

    /// The GitHub review to submit from the drafts and the summary, with
    /// the note ids of its threads and replies in order.
    fn collect_submission(&self) -> CollectedSubmission {
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
        CollectedSubmission {
            submission,
            thread_notes,
            reply_notes,
        }
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
            OperationStatus::Done(format!("Copied {count} note{plural} for your agent.").into());
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
