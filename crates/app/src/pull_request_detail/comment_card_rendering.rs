use gpui::{AnyElement, Context, Div, FontWeight, IntoElement, SharedString, div, prelude::*};
use qc_cache::{NoteAnchor, NoteDestination, NoteId, ReviewNote};
use qc_github::{LineSide, ReviewThread, ThreadComment};

use super::comment_editor::EditorTarget;
use super::pull_request_detail::PullRequestDetail;
use super::review_comments::{CommentCard, note_key};
use crate::button::Button;
use crate::diff_view::frame_file_row;
use crate::markdown_view::MarkdownView;
use crate::segmented_control::SegmentedControl;
use crate::theme;

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
        frame_file_row(div().px_3().py_1().children(content)).into_any_element()
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
        let destination = self.render_composer_destination(cx);
        let card = card_frame()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_xs()
                    .child(
                        div()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format!("New comment · {lines}")),
                    )
                    .child(destination)
                    .child(
                        div()
                            .flex_1()
                            .text_color(theme::muted())
                            .text_right()
                            .child("⇧-click another line's + to span lines"),
                    ),
            )
            .child(editor.text.clone())
            .child(Self::render_editor_buttons("composer", "Add comment", cx));
        frame_file_row(div().px_3().py_1().child(card)).into_any_element()
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
        let destination = self.review().destination;
        let detail = cx.entity().downgrade();
        let pick = move |destination: NoteDestination| {
            let detail = detail.clone();
            move |_: &mut gpui::Window, cx: &mut gpui::App| {
                detail
                    .update(cx, |detail, cx| {
                        detail.set_comment_destination(destination, cx);
                    })
                    .ok();
            }
        };
        SegmentedControl::new("composer-destination")
            .option(
                "GitHub draft",
                destination == NoteDestination::GitHub,
                pick(NoteDestination::GitHub),
            )
            .option(
                "Local note",
                destination == NoteDestination::Local,
                pick(NoteDestination::Local),
            )
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
            .items_center()
            .gap_2()
            .text_xs()
            .text_color(theme::muted())
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Conversation"),
            );
        if thread.is_resolved {
            header = header.child(render_badge("Resolved", theme::added()));
        }
        if thread.is_outdated {
            let original = thread
                .original_line
                .map_or_else(String::new, |line| format!(" (was line {line})"));
            header = header.child(render_badge(format!("Outdated{original}"), theme::muted()));
        }
        header = header.child(div().flex_1());
        let resolve = match (thread.is_resolved, thread.permissions) {
            (false, permissions) if permissions.can_resolve => Some(("Resolve", true)),
            (true, permissions) if permissions.can_unresolve => Some(("Reopen", false)),
            (true | false, _) => None,
        };
        if let Some((label, is_resolved)) = resolve {
            let resolving_id = thread_id.clone();
            header = header.child(
                Button::new(element_id("resolve", &thread_id), label).on_click(cx.listener(
                    move |detail, _, _, cx| {
                        detail.set_thread_resolved(resolving_id.clone(), is_resolved, cx);
                    },
                )),
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
                .child(editor.text.clone())
                .child(Self::render_editor_buttons(
                    element_id("reply", &thread_id),
                    "Add reply",
                    cx,
                ));
        } else if thread.permissions.can_reply {
            let path = thread.path.clone();
            card = card.child(div().child(
                Button::new(element_id("reply-open", &thread_id), "Reply").on_click(cx.listener(
                    move |detail, _, window, cx| {
                        detail.open_editor(
                            EditorTarget::Reply {
                                thread_id: thread_id.clone(),
                                path: path.clone(),
                            },
                            "",
                            window,
                            cx,
                        );
                    },
                )),
            ));
        }
        card
    }

    /// One GitHub comment: author, date, pending mark, body, and edit and
    /// delete when the viewer may; the editor while it is being edited.
    fn render_thread_comment(&self, comment: &ThreadComment, cx: &mut Context<Self>) -> Div {
        let is_editing = matches!(
            self.review().editor.as_ref().map(|editor| &editor.target),
            Some(EditorTarget::EditGitHubComment { comment_id }) if *comment_id == comment.id
        );
        let mut header = div()
            .flex()
            .items_center()
            .gap_2()
            .text_xs()
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(comment.author_login.clone()),
            )
            .child(
                div().text_color(theme::muted()).child(
                    comment
                        .created_at
                        .get(..10)
                        .unwrap_or(&comment.created_at)
                        .to_owned(),
                ),
            );
        if comment.is_pending {
            header = header.child(render_badge("Pending on GitHub", theme::accent()));
        }
        header = header.child(div().flex_1());
        if comment.can_update && !is_editing {
            let (id, body) = (comment.id.clone(), comment.body.clone());
            header = header.child(
                Button::new(element_id("edit-comment", &comment.id), "Edit").on_click(cx.listener(
                    move |detail, _, window, cx| {
                        detail.open_editor(
                            EditorTarget::EditGitHubComment {
                                comment_id: id.clone(),
                            },
                            &body,
                            window,
                            cx,
                        );
                    },
                )),
            );
        }
        if comment.can_delete {
            let id = comment.id.clone();
            header = header.child(
                Button::new(element_id("delete-comment", &comment.id), "Delete")
                    .danger()
                    .on_click(cx.listener(move |detail, _, _, cx| {
                        detail.delete_github_comment(id.clone(), cx);
                    })),
            );
        }
        let body = match (is_editing, &self.review().editor) {
            (true, Some(editor)) => div()
                .flex()
                .flex_col()
                .gap_1()
                .child(editor.text.clone())
                .child(Self::render_editor_buttons(
                    element_id("edit", &comment.id),
                    "Save on GitHub",
                    cx,
                ))
                .into_any_element(),
            (true | false, _) => self.render_markdown(&comment.id),
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .pt_1()
            .border_t_1()
            .border_color(theme::border())
            .child(header)
            .child(body)
    }

    /// One of the user's notes as a card of its own.
    fn render_note_card(&self, note: &ReviewNote, cx: &mut Context<Self>) -> impl IntoElement {
        let label = match note.destination {
            NoteDestination::GitHub => "Draft · posted with your review",
            NoteDestination::Local => "Local note",
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
    ) -> Div {
        let id = note.id;
        let is_editing = matches!(
            self.review().editor.as_ref().map(|editor| &editor.target),
            Some(EditorTarget::EditNote(edited)) if *edited == id
        );
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
            .items_center()
            .gap_2()
            .text_xs()
            .child(render_badge(label, color))
            .child(div().text_color(theme::muted()).child(lines))
            .child(div().flex_1());
        if !is_editing {
            let body = note.body.clone();
            header = header.child(Button::new(element_id("edit-note", &key), "Edit").on_click(
                cx.listener(move |detail, _, window, cx| {
                    detail.open_editor(EditorTarget::EditNote(id), &body, window, cx);
                }),
            ));
        }
        header = header.child(
            Button::new(element_id("delete-note", &key), "Delete")
                .danger()
                .on_click(cx.listener(move |detail, _, _, cx| detail.delete_note(id, cx))),
        );
        let body = match (is_editing, &self.review().editor) {
            (true, Some(editor)) => div()
                .flex()
                .flex_col()
                .gap_1()
                .child(editor.text.clone())
                .child(Self::render_editor_buttons(
                    element_id("edit-note-save", &key),
                    "Save",
                    cx,
                ))
                .into_any_element(),
            (true | false, _) => self.render_markdown(&note_key(id)),
        };
        div().flex().flex_col().gap_1().child(header).child(body)
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
        id: impl Into<gpui::ElementId>,
        save_label: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id: gpui::ElementId = id.into();
        div()
            .flex()
            .justify_end()
            .gap_2()
            .child(
                Button::new(
                    gpui::ElementId::NamedChild(Box::new(id.clone()), "cancel".into()),
                    "Cancel",
                )
                .on_click(cx.listener(|detail, _, window, cx| detail.close_editor(window, cx))),
            )
            .child(
                Button::new(
                    gpui::ElementId::NamedChild(Box::new(id), "save".into()),
                    format!("{save_label}  ⌘↩"),
                )
                .primary()
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
        .gap_1p5()
        .p_2()
        .rounded_md()
        .border_1()
        .border_color(theme::border())
        .bg(theme::panel())
}

/// A small colored label.
fn render_badge(text: impl Into<SharedString>, color: gpui::Rgba) -> impl IntoElement {
    div()
        .flex_none()
        .px_1()
        .rounded_sm()
        .border_1()
        .border_color(color)
        .text_xs()
        .text_color(color)
        .child(text.into())
}

/// `Line 42`, `Lines 40–42`, with ` (old)` on the old side.
pub(super) fn lines_label(side: LineSide, line: u32, start_line: Option<u32>) -> String {
    let side = match side {
        LineSide::New => "",
        LineSide::Old => " (old)",
    };
    match start_line {
        Some(start) if start != line => format!("Lines {start}–{line}{side}"),
        Some(_) | None => format!("Line {line}{side}"),
    }
}
