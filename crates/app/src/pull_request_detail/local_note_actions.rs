use gpui::{ClipboardItem, Context};
use qd_cache::{NoteAnchor, NoteDestination, NoteId, ReviewNote};

use super::comment_card::CommentCard;
use super::detail_row::DetailRow;
use super::local_review_export::{ReviewExportHeader, local_review_markdown};
use super::operation_status::OperationStatus;
use super::pull_request_detail::PullRequestDetail;
use super::review_target::ReviewTarget;
use crate::display_date::unix_now;
use crate::short_sha::short_sha;
use crate::ui_state::{ReviewChange, UiStateRecorder};

impl PullRequestDetail {
    /// Adds a note of the shown scope and saves it.
    pub(super) fn add_note(
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
    pub(super) fn forget_note(&mut self, id: NoteId, cx: &mut Context<Self>) {
        self.review_mut()
            .comments
            .notes
            .retain(|note| note.id != id);
        UiStateRecorder::record_review_change(cx, ReviewChange::DeleteNote(id));
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
