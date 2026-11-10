use gpui::{AppContext as _, Context, Window};
use qd_cache::{NoteAnchor, NoteDestination};

use super::comment_card::CommentCard;
use super::comment_editor::{CommentEditor, EditorTarget};
use super::comment_range::CommentRange;
use super::composer_opening::ComposerOpening;
use super::detail_row::DetailRow;
use super::diff_line_ref::DiffLineRef;
use super::pull_request_detail::PullRequestDetail;
use crate::display_date::unix_now;
use crate::text_area::{TextArea, TextAreaEvent};
use crate::ui_state::{ReviewChange, UiStateRecorder};

/// The placeholder of a new comment's editor.
const COMMENT_PLACEHOLDER: &str = "Write a comment… (⌘↩ to save, esc to cancel)";

impl PullRequestDetail {
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
    pub(super) fn file_of_path(&self, path: &str) -> Option<usize> {
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
    pub(super) fn refresh_comment_files(&mut self, files: &[Option<usize>]) {
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
}
