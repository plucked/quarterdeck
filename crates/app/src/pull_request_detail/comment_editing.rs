use gpui::{AppContext as _, Context, Window};
use qd_cache::{NoteAnchor, NoteDestination};

use super::comment_card::CommentCard;
use super::comment_editor::{CommentEditor, EditorTarget};
use super::comment_range::CommentRange;
use super::composer_opening::ComposerOpening;
use super::detail_row::DetailRow;
use super::diff_line_ref::DiffLineRef;
use super::pull_request_detail::PullRequestDetail;
use crate::display_text::unix_now;
use crate::text_area::{TextArea, TextAreaEvent};
use crate::ui_state::{ReviewChange, UiStateRecorder};

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
        if opening == ComposerOpening::ExtendOpen && self.review.editor.is_some() {
            self.extend_open_comment(at, cx);
            return;
        }
        self.open_range_composer(
            CommentRange::ending_at(at.file, at.side, at.line, None),
            window,
            cx,
        );
    }

    /// An invalid Shift-click leaves the draft in place so it can still be saved.
    fn extend_open_comment(&mut self, at: DiffLineRef, cx: &mut Context<Self>) {
        let Some(editor) = self.review.editor.as_ref() else {
            return;
        };
        if !matches!(editor.target, EditorTarget::NewComment { .. })
            || editor.text.read(cx).suggestion_anchors_range()
        {
            return;
        }
        let Some(extended) = self
            .comment_range()
            .and_then(|range| range.extended_to(at, self.loaded_files().get(at.file)?))
        else {
            return;
        };
        if let Some(EditorTarget::NewComment {
            side,
            line,
            start_line,
            ..
        }) = self.review.editor.as_mut().map(|editor| &mut editor.target)
        {
            *side = extended.side;
            *line = extended.last;
            *start_line = (extended.first < extended.last).then_some(extended.first);
        }
        self.refresh_suggestion_source();
        self.refresh_comment_files(&[Some(at.file)]);
        cx.notify();
    }

    /// Opens a comment on the supplied single-line or multiline anchor.
    pub(super) fn open_range_composer(
        &mut self,
        range: CommentRange,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(file) = self.loaded_files().get(range.file) else {
            return;
        };
        self.open_editor(
            EditorTarget::NewComment {
                file: range.file,
                path: file.filename().to_owned(),
                side: range.side,
                line: range.last,
                start_line: (range.first < range.last).then_some(range.first),
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
        let placeholder = match &target {
            EditorTarget::Reply { .. } => "Write a reply…",
            _ => "Write a comment…",
        };
        let github = self.editor_destination(&target) == NoteDestination::GitHub;
        let area = cx.new(|cx| {
            let mut area = TextArea::comment(placeholder, cx);
            area.set_text(text, cx);
            area.set_github_markdown(github, cx);
            area
        });
        let subscription = cx.subscribe_in(
            &area,
            window,
            |detail, _, event: &TextAreaEvent, window, cx| match event {
                TextAreaEvent::Submit => detail.save_editor(window, cx),
                TextAreaEvent::Cancel => detail.close_editor(window, cx),
                TextAreaEvent::FormattingChanged => cx.notify(),
                TextAreaEvent::Changed | TextAreaEvent::LayoutChanged => {
                    detail.remeasure_editor_row();
                    cx.notify();
                }
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
        let suggestion_source = target.suggestion_source(self.loaded_files());
        self.review_mut().editor = Some(CommentEditor {
            target,
            text: area,
            suggestion_source,
            _subscription: subscription,
        });
        self.refresh_comment_files(&[closed_file, opened_file]);
        if let Some(file) = opened_file {
            self.select_diff_file(file);
        }
        // The virtual list knows the editor's height once the inserted row has been laid out.
        cx.on_next_frame(window, |detail, _, cx| {
            if let Some(row) = detail.editor_row() {
                detail.list().scroll_to_reveal_item(row);
                cx.notify();
            }
        });
        cx.notify();
    }

    /// Updates the code offered by Suggest change after the composer range is extended.
    pub(super) fn refresh_suggestion_source(&mut self) {
        let source = self
            .review()
            .editor
            .as_ref()
            .and_then(|editor| editor.target.suggestion_source(self.loaded_files()));
        if let Some(editor) = &mut self.review_mut().editor {
            editor.suggestion_source = source;
        }
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
                if !note.matches_revision(self.shown_head_sha(), self.shown_base_sha.as_deref()) {
                    return None;
                }
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

    /// Lets the list measure the editor again after its text or toolbar changes.
    pub(super) fn remeasure_editor_row(&mut self) {
        if let Some(row) = self.editor_row() {
            self.remeasure_row(row);
        }
    }

    /// Finds the open editor in the active file's virtualized rows.
    fn editor_row(&self) -> Option<usize> {
        let target = &self.review().editor.as_ref()?.target;
        let rows = self.rows().file_range(self.editor_file(target)?)?;
        rows.into_iter()
            .find(|row| match (self.rows().get(*row), target) {
                (Some(DetailRow::Composer { .. }), EditorTarget::NewComment { .. }) => true,
                (Some(DetailRow::CommentCard { card, .. }), _) => {
                    self.card_holds_editor(card, target)
                }
                _ => false,
            })
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
            self.resume_deferred_reload(cx);
            cx.notify();
        }
    }

    /// Saves what the editor holds: a new note or draft, an edited one, or
    /// an edit of a GitHub comment (sent at once). An empty text saves nothing.
    pub(super) fn save_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self
            .review_mut()
            .editor
            .take_if(|editor| !editor.text.read(cx).text().trim().is_empty())
        else {
            return;
        };
        window.focus(self.focus());
        let body = editor.text.read(cx).text().trim().to_owned();
        let file = self.editor_file(&editor.target);
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
                    self.review_mut().claude_picker = None;
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
        self.resume_deferred_reload(cx);
        cx.notify();
    }
}
