use gpui::{AnyElement, Context, Div, FontWeight, IntoElement, SharedString, div, prelude::*, px};
use qd_cache::{NoteAnchor, NoteDestination, NoteId, ReviewNote};
use qd_github::{ReviewThread, ThreadComment};

use super::comment_card::CommentCard;
use super::comment_editor::{CommentEditor, EditorTarget};
use super::lines_label::lines_label;
use super::pull_request_detail::PullRequestDetail;
use super::review_comments::note_key;
use super::thread_resolution::ThreadResolution;
use crate::avatar_images::Avatar;
use crate::controls::{Button, NumberFlow, Pill, PillStyle, TextTooltip};
use crate::diff_view::frame_file_row;
use crate::display_text::short_date;
use crate::markdown_view::MarkdownView;
use crate::text_area::MarkdownEditor;
use crate::theme;

/// A comment author's avatar in the comment's header line.
const COMMENT_AVATAR_SIZE: f32 = 18.;

impl PullRequestDetail {
    /// Card `card` of the diff, inside its file's frame: a GitHub
    /// conversation with its draft replies, or one of the user's notes.
    pub(super) fn render_comment_card_row(
        &self,
        card: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(placed) = self.review().cards.get(card) else {
            return div().into_any_element();
        };
        let content = match &placed.card {
            CommentCard::Thread { thread, replies } => self
                .review()
                .comments
                .github
                .as_ref()
                .and_then(|state| state.threads.get(*thread))
                .map(|thread| {
                    self.render_thread_card(thread, replies, cx)
                        .into_any_element()
                }),
            CommentCard::Note(id) => self
                .review()
                .comments
                .note(*id)
                .map(|note| self.render_note_card(note, cx).into_any_element()),
        };
        frame_file_row(div().px_4().py_1p5().children(content)).into_any_element()
    }

    /// The editor of a new comment under its line, inside its file's frame.
    pub(super) fn render_composer_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let Some(editor) = &self.review().editor else {
            return div().into_any_element();
        };
        let EditorTarget::NewComment {
            side,
            line,
            start_line,
            ..
        } = &editor.target
        else {
            return div().into_any_element();
        };
        let lines = lines_label(*side, *line, *start_line);
        let range_hint = if editor.text.read(cx).suggestion_anchors_range() {
            "This suggestion is anchored here. Start a new comment to suggest a different range."
        } else {
            "Drag across code lines or Shift-click another line’s + to select a range."
        };
        let save_label = match self.active_destination() {
            NoteDestination::GitHub => "Add to review",
            NoteDestination::Local => "Save note",
        };
        let header = div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("composer-line")
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("Comment on {}", lines.to_lowercase()))
                            .tooltip(TextTooltip::builder(range_hint)),
                    )
                    .children(self.render_suggestion_button(editor)),
            )
            .child(self.render_composer_destination(cx));
        let composer = div()
            .flex()
            .flex_col()
            .min_w_0()
            .gap_2()
            .bg(theme::panel())
            .px_4()
            .py_3()
            .child(header)
            .child(self.render_editor_text(editor))
            .child(self.render_editor_buttons("composer", save_label, cx));
        frame_file_row(composer).into_any_element()
    }

    /// Offers the anchored head-side code as an editable replacement in a GitHub draft.
    fn render_suggestion_button(&self, editor: &CommentEditor) -> Option<Button> {
        if self.active_destination() != NoteDestination::GitHub {
            return None;
        }
        let source = editor.suggestion_source.clone()?;
        let area = editor.text.clone();
        Some(
            Button::new("composer-suggest-change", "Suggest change")
                .ghost()
                .on_click(move |_, window, cx| {
                    area.update(cx, |area, cx| {
                        area.insert_suggestion(&source, cx);
                        area.focus(window);
                    });
                }),
        )
    }

    /// Where the new comment goes: a GitHub / Local switch on a single pull
    /// request (kept for the next comments), else a local note.
    fn render_composer_destination(&self, cx: &mut Context<Self>) -> AnyElement {
        if !self.can_review_on_github() {
            return div()
                .text_color(theme::renamed())
                .child("Local note")
                .into_any_element();
        }
        self.render_destination_control("composer-destination", "GitHub draft", "Local note", cx)
            .quiet()
            .into_any_element()
    }

    /// A GitHub conversation: its state and actions, every comment, the
    /// viewer's draft replies, and a reply editor or button.
    fn render_thread_card(
        &self,
        thread: &ReviewThread,
        replies: &[NoteId],
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let thread_id = thread.id.clone();
        let mut header = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x_2()
            .gap_y_1()
            .text_xs()
            .text_color(theme::muted())
            .child(div().font_weight(FontWeight::MEDIUM).child("Conversation"));
        if thread.is_resolved {
            header = header.child(render_badge("Resolved", theme::success()));
        }
        if thread.is_outdated {
            let original = thread
                .original_line
                .map_or_else(String::new, |line| format!(" (was line {line})"));
            header = header.child(render_badge(format!("Outdated{original}"), theme::muted()));
        }
        header = header.child(div().flex_1());
        if let Some(resolution) = ThreadResolution::offered_for(thread) {
            let resolving_id = thread_id.clone();
            header = header.child(
                Button::new(element_id("resolve", &thread_id), resolution.label())
                    .ghost()
                    .on_click(cx.listener(move |detail, _, _, cx| {
                        detail.set_thread_resolution(resolving_id.clone(), resolution, cx);
                    })),
            );
        }

        // Every comment, then the viewer's draft replies.
        let mut card = card_frame().child(header);
        for comment in &thread.comments {
            card = card.child(self.render_thread_comment(comment, cx));
        }
        for reply in replies {
            if let Some(note) = self.review().comments.note(*reply) {
                card = card.child(self.render_note_body(note, "Draft reply", cx));
            }
        }

        // A reply: the editor while writing one, else the button.
        let is_replying = matches!(
            self.review().editor.as_ref().map(|editor| &editor.target),
            Some(EditorTarget::Reply { thread_id: replying, .. }) if *replying == thread.id
        );
        if is_replying && let Some(editor) = &self.review().editor {
            card = card
                .child(self.render_editor_text(editor))
                .child(self.render_editor_buttons(
                    element_id("reply", &thread_id),
                    "Add reply",
                    cx,
                ));
        } else if thread.permissions.can_reply {
            let path = thread.path.clone();
            card = card.child(
                div().child(
                    Button::new(element_id("reply-open", &thread_id), "Reply")
                        .ghost()
                        .on_click(cx.listener(move |detail, _, window, cx| {
                            detail.open_editor(
                                EditorTarget::Reply {
                                    thread_id: thread_id.clone(),
                                    path: path.clone(),
                                },
                                "",
                                window,
                                cx,
                            );
                        })),
                ),
            );
        }
        card
    }

    /// One GitHub comment: author, date, pending mark, body, and edit and
    /// delete when the viewer may; the editor while it is being edited.
    fn render_thread_comment(
        &self,
        comment: &ThreadComment,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let editing = self.review().editor.as_ref().filter(|editor| {
            matches!(
                &editor.target,
                EditorTarget::EditGitHubComment { comment_id } if *comment_id == comment.id
            )
        });
        let is_editing = editing.is_some();
        let mut header = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x_2()
            .gap_y_1()
            .text_xs()
            .when(!comment.author_avatar_url.is_empty(), |header| {
                header.child(Avatar::new(
                    comment.author_avatar_url.clone(),
                    px(COMMENT_AVATAR_SIZE),
                ))
            })
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(comment.author_login.clone()),
            )
            .child(
                div()
                    .text_color(theme::subtle())
                    .child(NumberFlow::label(short_date(&comment.created_at))),
            );
        if comment.is_pending {
            header = header.child(render_badge("Pending on GitHub", theme::accent()));
        }
        let mut actions = div().flex().flex_none().ml_auto().gap_1();
        if comment.can_update && !is_editing {
            let (id, body) = (comment.id.clone(), comment.body.clone());
            actions = actions.child(
                Button::new(element_id("edit-comment", &comment.id), "Edit")
                    .ghost()
                    .on_click(cx.listener(move |detail, _, window, cx| {
                        detail.open_editor(
                            EditorTarget::EditGitHubComment {
                                comment_id: id.clone(),
                            },
                            &body,
                            window,
                            cx,
                        );
                    })),
            );
        }
        if comment.can_delete {
            let id = comment.id.clone();
            actions = actions.child(
                Button::new(element_id("delete-comment", &comment.id), "Delete")
                    .ghost()
                    .on_click(cx.listener(move |detail, _, _, cx| {
                        detail.delete_github_comment(id.clone(), cx);
                    })),
            );
        }
        header = header.child(actions);
        let body = self.render_comment_body(
            editing,
            element_id("edit", &comment.id),
            "Save on GitHub",
            &comment.id,
            cx,
        );
        div()
            .id(element_id("thread-comment", &comment.id))
            .flex()
            .flex_col()
            .gap_1()
            .pt_2()
            .border_t_1()
            .border_color(theme::border())
            .child(header)
            .child(body)
    }

    /// One of the user's notes as a card of its own.
    fn render_note_card(&self, note: &ReviewNote, cx: &mut Context<Self>) -> impl IntoElement {
        let label = if note.matches_revision(self.shown_head_sha(), self.shown_base_sha.as_deref())
        {
            match note.destination {
                NoteDestination::GitHub => "Draft · posted with your review",
                NoteDestination::Local => "Local note",
            }
        } else {
            "Outdated note"
        };
        card_frame().child(self.render_note_body(note, label, cx))
    }

    /// A note's label, lines, body and edit and delete buttons; the editor
    /// while it is being edited.
    fn render_note_body(
        &self,
        note: &ReviewNote,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<Div> {
        let id = note.id;
        let editing = self.review().editor.as_ref().filter(
            |editor| matches!(&editor.target, EditorTarget::EditNote(edited) if *edited == id),
        );
        let is_editing = editing.is_some();
        let lines = match &note.anchor {
            NoteAnchor::Line {
                side,
                line,
                start_line,
            } => lines_label(*side, *line, *start_line),
            NoteAnchor::Reply { .. } => String::new(),
        };
        let color = match note.destination {
            NoteDestination::GitHub => theme::accent(),
            NoteDestination::Local => theme::renamed(),
        };
        let key = SharedString::from(note_key(id));
        let mut header = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap_x_2()
            .gap_y_1()
            .text_xs()
            .child(render_badge(label, color))
            .child(
                div()
                    .text_color(theme::subtle())
                    .child(NumberFlow::label(lines)),
            );
        let mut actions = div().flex().flex_none().ml_auto().gap_1();
        if !is_editing {
            let body = note.body.clone();
            actions = actions.child(
                Button::new(element_id("edit-note", &key), "Edit")
                    .ghost()
                    .on_click(cx.listener(move |detail, _, window, cx| {
                        detail.open_editor(EditorTarget::EditNote(id), &body, window, cx);
                    })),
            );
        }
        actions = actions.child(
            Button::new(element_id("delete-note", &key), "Delete")
                .ghost()
                .on_click(cx.listener(move |detail, _, _, cx| detail.delete_note(id, cx))),
        );
        header = header.child(actions);
        let body = self.render_comment_body(
            editing,
            element_id("edit-note-save", &key),
            "Save",
            &key,
            cx,
        );
        div()
            .id(element_id("note-body", &key))
            .flex()
            .flex_col()
            .gap_1()
            .child(header)
            .child(body)
    }

    /// A comment's body: while `editing`, its editor with cancel and
    /// `save_label` (buttons under `editor_id`); else the body with `key`
    /// as Markdown.
    fn render_comment_body(
        &self,
        editing: Option<&CommentEditor>,
        editor_id: gpui::ElementId,
        save_label: &'static str,
        key: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match editing {
            Some(editor) => div()
                .flex()
                .flex_col()
                .gap_1()
                .child(self.render_editor_text(editor))
                .child(self.render_editor_buttons(editor_id, save_label, cx))
                .into_any_element(),
            None => self.render_markdown(key),
        }
    }

    /// Shows formatting for the destination of this editor, including GitHub
    /// replies and edits opened while Local is selected in the review panel.
    fn render_editor_text(&self, editor: &CommentEditor) -> AnyElement {
        match self.editor_destination(&editor.target) {
            NoteDestination::GitHub => MarkdownEditor::new(editor.text.clone()).into_any_element(),
            NoteDestination::Local => div()
                .min_w_0()
                .rounded_md()
                .border_1()
                .border_color(theme::border_strong())
                .bg(theme::background())
                .child(editor.text.clone())
                .into_any_element(),
        }
    }

    /// A comment body as Markdown (selectable, links open).
    fn render_markdown(&self, key: &str) -> AnyElement {
        match self.review().comments.document(key) {
            Some(document) => div()
                .text_sm()
                .child(MarkdownView::new(
                    SharedString::from(format!("comment-{key}")),
                    document,
                ))
                .into_any_element(),
            None => div().into_any_element(),
        }
    }

    /// Cancel and save under an open editor.
    fn render_editor_buttons(
        &self,
        id: impl Into<gpui::ElementId>,
        save_label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id: gpui::ElementId = id.into();
        let editor = self.review().editor.as_ref();
        let is_empty = editor.is_none_or(|editor| editor.text.read(cx).text().trim().is_empty());
        let is_review_draft = editor.is_some_and(|editor| match editor.target {
            EditorTarget::NewComment { .. } => self.active_destination() == NoteDestination::GitHub,
            EditorTarget::Reply { .. } => true,
            _ => false,
        });
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_end()
            .gap_2()
            .when(is_review_draft, |row| {
                row.child(
                    div()
                        .flex_1()
                        .min_w(gpui::px(96.))
                        .text_xs()
                        .text_color(theme::muted())
                        .child("Posts when you submit your review."),
                )
            })
            .child(
                Button::new(
                    gpui::ElementId::NamedChild(Box::new(id.clone()), "cancel".into()),
                    "Cancel",
                )
                .ghost()
                .on_click(cx.listener(|detail, _, window, cx| detail.close_editor(window, cx))),
            )
            .child(
                Button::new(
                    gpui::ElementId::NamedChild(Box::new(id), "save".into()),
                    format!("{save_label}  ⌘↩"),
                )
                .primary()
                .disabled(is_empty)
                .on_click(cx.listener(|detail, _, window, cx| detail.save_editor(window, cx))),
            )
    }
}

/// An element id made of a name and a key (a comment or note id).
pub(super) fn element_id(name: &str, key: &str) -> gpui::ElementId {
    gpui::ElementId::Name(format!("{name}-{key}").into())
}

/// The frame of a comment card.
fn card_frame() -> Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .px_3()
        .py_2p5()
        .rounded_lg()
        .border_1()
        .border_color(theme::border_strong())
        .bg(theme::surface())
}

/// A small pill in `color` saying a comment's state.
#[track_caller]
fn render_badge(text: impl Into<SharedString>, color: gpui::Rgba) -> Pill {
    Pill::new(text, PillStyle::Tinted(color))
        .compact()
        .weight(FontWeight::MEDIUM)
}
