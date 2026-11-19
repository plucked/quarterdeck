use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui::Context;
use qd_cache::{NoteAnchor, NoteDestination, NoteId, ReviewNote, ReviewScope, ReviewSummary};
use qd_github::{
    GitHubError, NewReviewThread, NewThreadReply, ReviewEvent, ReviewSubmission, SubmittedPart,
};

use super::detail_event::DetailEvent;
use super::operation_status::OperationStatus;
use super::pull_request_detail::PullRequestDetail;
use super::review_target::ReviewTarget;
use super::thread_resolution::ThreadResolution;
use crate::avatar_images::AvatarImages;
use crate::ui_state::{ReviewChange, UiStateRecorder};

/// A GitHub review to submit, with the ids of the notes its new
/// conversations and replies come from, in order, so each draft GitHub
/// takes can be dropped as it is taken.
struct CollectedSubmission {
    submission: ReviewSubmission,
    thread_notes: Vec<NoteId>,
    reply_notes: Vec<NoteId>,
}

impl PullRequestDetail {
    /// Whether the shown files can take GitHub comments (a single pull request).
    pub(super) fn can_review_on_github(&self) -> bool {
        *self.target() == ReviewTarget::Layer
    }

    /// Reads GitHub's conversations and the viewer's pending review for the
    /// shown pull request, following any additional thread and comment pages; combined reviews have none.
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
                            comments.github_error = None;
                            // Only thread changes affect the inline cards, their authors and row heights.
                            let threads_changed = comments
                                .github
                                .as_ref()
                                .map_or(&[][..], |shown| &shown.threads)
                                != state.threads;
                            if threads_changed {
                                AvatarImages::load(
                                    cx,
                                    state
                                        .threads
                                        .iter()
                                        .flat_map(|thread| &thread.comments)
                                        .map(|comment| comment.author_avatar_url.clone()),
                                );
                            }
                            comments.github = Some(state);
                            if threads_changed {
                                detail.rebuild_rows_in_place();
                            }
                        }
                        Err(error) => comments.github_error = Some(format!("{error:#}").into()),
                    }
                    cx.notify();
                })
                .ok();
        });
        self.review_mut().load_task = Some(task);
    }

    /// Runs one GitHub call off the main thread, saying what it does, then
    /// reads the conversations again so the diff shows the result.
    pub(super) fn run_github_action(
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

        if self
            .review()
            .comments
            .notes_to(NoteDestination::GitHub)
            .any(|note| {
                !note.matches_revision(self.shown_head_sha(), self.shown_base_sha.as_deref())
            })
        {
            self.review_mut().status = OperationStatus::Failed("An outdated draft refers to earlier code. Recreate it on the current diff and remove the old draft before submitting.".into());
            cx.notify();
            return;
        }
        let original_summary = self.review().comments.summary.clone();
        let Some(head_sha) = self.shown_head_sha() else {
            return;
        };
        let CollectedSubmission {
            mut submission,
            thread_notes,
            reply_notes,
        } = self.collect_submission(head_sha.to_owned());
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
                    detail.finish_submission(pull_request_id, submitted, &original_summary, cx);
                })
                .ok();
        });
        self.review_mut().submission = Some(task);
        cx.notify();
    }

    /// Takes in how the submission of pull request `pull_request_id`'s
    /// review ended: its saved summary goes once submitted, and while that
    /// pull request still shows, the review tab says how it went and the
    /// conversations are read again.
    fn finish_submission(
        &mut self,
        pull_request_id: u64,
        submitted: Result<(), GitHubError>,
        original_summary: &ReviewSummary,
        cx: &mut Context<Self>,
    ) {
        // This task ends here: let go of its handle without cancelling it.
        if let Some(task) = self.review_mut().submission.take() {
            task.detach();
        }

        // The saved summary of a submitted review goes, whatever shows now.
        if submitted.is_ok() {
            cx.emit(DetailEvent::PullRequestChanged(pull_request_id));
            UiStateRecorder::record_review_change(
                cx,
                ReviewChange::DeleteSummary(pull_request_id, original_summary.clone()),
            );
        }

        // Another pull request or range shows now: its review stays as it is.
        if self.review_scope() != Some(&ReviewScope::PullRequest(pull_request_id)) {
            cx.notify();
            return;
        }

        // Say how it went and read the conversations again.
        self.review_mut().status = match submitted {
            Ok(()) if self.review().comments.summary != *original_summary => {
                OperationStatus::Done("Review submitted. Your newer draft was kept.".into())
            }
            Ok(()) => {
                self.review_mut().comments.summary = ReviewSummary::default();
                if let Some(area) = self.review().summary_editor.clone() {
                    area.update(cx, |area, cx| area.set_text("", cx));
                }
                OperationStatus::Done("Review submitted.".into())
            }
            Err(error) => OperationStatus::Failed(format!("{error:#}").into()),
        };
        self.load_github_review(cx);
        self.rebuild_rows_in_place();
        cx.notify();
    }

    /// The GitHub review to submit from the drafts and the summary, with
    /// the note ids of its threads and replies in order.
    fn collect_submission(&self, head_sha: String) -> CollectedSubmission {
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
            head_sha,
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
}
