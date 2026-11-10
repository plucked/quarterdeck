use gpui::{AnyElement, Context, FontWeight, IntoElement, SharedString, div, prelude::*};
use qd_cache::{NoteAnchor, NoteDestination, ReviewNote};
use qd_github::ReviewEvent;

use super::comment_card_rendering::{element_id, lines_label};
use super::pull_request_detail::PullRequestDetail;
use super::review_status::ReviewStatus;
use super::side_panel_tab::SidePanelTab;
use crate::button::Button;
use crate::section_label::SectionLabel;
use crate::segmented_control::SegmentedControl;
use crate::theme;
use crate::view_option::view_option;

impl PullRequestDetail {
    /// The tabs atop the side panel: Files, Review (with its comment count),
    /// Actions and Checks.
    pub(super) fn render_side_panel_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = self.review().tab;
        let count = self
            .review()
            .comments
            .notes_to(self.active_destination())
            .count();
        let review_label = if count == 0 {
            "Review".to_owned()
        } else {
            format!("Review  {count}")
        };
        let detail = cx.entity().downgrade();
        let show = |tab| view_option(&detail, tab, PullRequestDetail::show_side_panel_tab);
        div().flex_none().px_3().pt_3().pb_1().child(
            SegmentedControl::new("side-panel-tabs")
                .option(
                    "Files",
                    tab == SidePanelTab::Files,
                    show(SidePanelTab::Files),
                )
                .option(
                    review_label,
                    tab == SidePanelTab::Review,
                    show(SidePanelTab::Review),
                )
                .option(
                    "Actions",
                    tab == SidePanelTab::Actions,
                    show(SidePanelTab::Actions),
                )
                .option(
                    "Checks",
                    tab == SidePanelTab::Checks,
                    show(SidePanelTab::Checks),
                ),
        )
    }

    /// The review tab: where comments go, what the user wrote, and
    /// submitting it to GitHub or copying it for an agent.
    pub(super) fn render_review_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let destination = self.active_destination();
        let body = match destination {
            NoteDestination::GitHub => self.render_github_review(cx),
            NoteDestination::Local => self.render_local_review(cx),
        };
        div()
            .id("review-panel")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_4()
            .px_3()
            .pt_2()
            .pb_4()
            .child(self.render_destination_switch(cx))
            .children(self.render_review_status())
            .child(body)
    }

    /// GitHub or Local: where new comments go. A combined review is local only.
    fn render_destination_switch(&self, cx: &mut Context<Self>) -> AnyElement {
        if !self.can_review_on_github() {
            return div()
                .text_xs()
                .text_color(theme::muted())
                .child(
                    "Combined reviews are local: GitHub has no place for comments across layers.",
                )
                .into_any_element();
        }
        let destination = self.review().destination;
        let detail = cx.entity().downgrade();
        let pick = |destination| {
            view_option(
                &detail,
                destination,
                PullRequestDetail::set_comment_destination,
            )
        };
        div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(theme::subtle())
                    .child("Comments go to"),
            )
            .child(
                SegmentedControl::new("comment-destination")
                    .option(
                        "GitHub",
                        destination == NoteDestination::GitHub,
                        pick(NoteDestination::GitHub),
                    )
                    .option(
                        "Local",
                        destination == NoteDestination::Local,
                        pick(NoteDestination::Local),
                    ),
            )
            .into_any_element()
    }

    /// The last review action's outcome, while there is one.
    fn render_review_status(&self) -> Option<AnyElement> {
        let (text, color) = match &self.review().status {
            ReviewStatus::Idle => return None,
            ReviewStatus::Working(text) => (text.clone(), theme::accent()),
            ReviewStatus::Done(text) => (text.clone(), theme::success()),
            ReviewStatus::Failed(text) => (text.clone(), theme::error()),
        };
        Some(
            div()
                .text_xs()
                .text_color(color)
                .child(text)
                .into_any_element(),
        )
    }

    /// The GitHub review: drafts, a pending review begun in the browser,
    /// the verdict, the summary and the submit button.
    fn render_github_review(&self, cx: &mut Context<Self>) -> AnyElement {
        let comments = &self.review().comments;
        if comments.is_loading_github && comments.github.is_none() {
            return muted_line("Reading the conversations from GitHub…");
        }
        if let Some(error) = &comments.github_error {
            return div()
                .text_xs()
                .text_color(theme::error())
                .child(format!("GitHub's conversations could not be read: {error}"))
                .into_any_element();
        }
        let Some(state) = &comments.github else {
            return muted_line("GitHub review needs a GitHub token.");
        };
        let drafts: Vec<&ReviewNote> = comments.notes_to(NoteDestination::GitHub).collect();
        let has_pending = state.pending_review_id.is_some();
        let event = comments.summary.event;
        let can_submit = !drafts.is_empty()
            || has_pending
            || event != ReviewEvent::Comment
            || !comments.summary.body.trim().is_empty();

        // The verdict: approving and requesting changes are not for one's own pull request.
        let verdict: AnyElement = if state.viewer_is_author {
            muted_line("Your own pull request: your review can only comment.")
        } else {
            let detail = cx.entity().downgrade();
            let pick = |event| view_option(&detail, event, PullRequestDetail::set_review_event);
            SegmentedControl::new("review-event")
                .option(
                    "Comment",
                    event == ReviewEvent::Comment,
                    pick(ReviewEvent::Comment),
                )
                .option(
                    "Approve",
                    event == ReviewEvent::Approve,
                    pick(ReviewEvent::Approve),
                )
                .option(
                    "Request changes",
                    event == ReviewEvent::RequestChanges,
                    pick(ReviewEvent::RequestChanges),
                )
                .into_any_element()
        };

        let count = drafts.len();
        // While a submission runs the button says so and takes no clicks.
        let is_submitting = self.review().submission.is_some();
        let submit_label = match (&self.review().submission, count) {
            (Some(_), _) => "Submitting…".to_owned(),
            (None, 0) => "Submit review".to_owned(),
            (None, 1) => "Submit review (1 comment)".to_owned(),
            (None, _) => format!("Submit review ({count} comments)"),
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(SectionLabel::new(format!("Your drafts ({count})")).compact())
            .child(Self::render_note_list(&drafts, "Click + beside a line in the diff to comment on it. Drafts are posted when you submit.", cx))
            .when(has_pending, |panel| {
                panel.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(muted_line("You have a pending review on GitHub (begun in the browser). Its comments are submitted with this review."))
                        .child(div().child(
                            Button::new("discard-pending", "Discard pending review")
                                .danger()
                                .on_click(cx.listener(|detail, _, _, cx| detail.discard_pending_review(cx))),
                        )),
                )
            })
            .child(SectionLabel::new("Verdict").compact())
            .child(verdict)
            .children(self.review().summary_editor.clone())
            .child(
                div().flex().justify_end().child(
                    Button::new("submit-review", submit_label)
                        .primary()
                        .disabled(!can_submit || is_submitting)
                        .on_click(cx.listener(|detail, _, _, cx| detail.submit_review(cx))),
                ),
            )
            .into_any_element()
    }

    /// The local review: the notes, and copying them for an agent.
    fn render_local_review(&self, cx: &mut Context<Self>) -> AnyElement {
        let notes: Vec<&ReviewNote> = self
            .review()
            .comments
            .notes_to(NoteDestination::Local)
            .collect();
        let count = notes.len();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(SectionLabel::new(format!("Local notes ({count})")).compact())
            .child(Self::render_note_list(&notes, "Click + beside a line in the diff to note something. Notes stay on this Mac.", cx))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new("clear-local", "Clear all")
                            .danger()
                            .disabled(count == 0)
                            .on_click(cx.listener(|detail, _, _, cx| detail.clear_local_notes(cx))),
                    )
                    .child(
                        Button::new("copy-local", "Copy for agent")
                            .primary()
                            .disabled(count == 0)
                            .on_click(cx.listener(|detail, _, _, cx| detail.copy_local_review(cx))),
                    ),
            )
            .child(muted_line("Copies the notes as Markdown: file paths, line numbers, the code and your note, ready to paste to an agent."))
            .into_any_element()
    }

    /// The notes as a list by file: lines and the note's first line;
    /// clicking jumps to it in the diff, × deletes it.
    fn render_note_list(
        notes: &[&ReviewNote],
        empty: &'static str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if notes.is_empty() {
            return muted_line(empty);
        }
        let mut list = div().flex().flex_col().gap_1();
        let mut current_path: Option<&str> = None;
        for note in notes {
            if current_path != Some(note.path.as_str()) {
                list = list.child(
                    div()
                        .pt_1p5()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme::muted())
                        .truncate()
                        .child(note.path.clone()),
                );
                current_path = Some(&note.path);
            }
            let id = note.id;
            let lines = match &note.anchor {
                NoteAnchor::Line {
                    side,
                    line,
                    start_line,
                } => lines_label(*side, *line, *start_line),
                NoteAnchor::Reply { .. } => "Reply".to_owned(),
            };
            let first_line = note.body.lines().next().unwrap_or_default().to_owned();
            let key = SharedString::from(format!("{id:?}"));
            list = list.child(
                div()
                    .id(element_id("note-row", &key))
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .border_1()
                    .border_color(theme::border())
                    .bg(theme::surface())
                    .text_xs()
                    .cursor_pointer()
                    .hover(|style| style.border_color(theme::border_strong()))
                    .on_click(cx.listener(move |detail, _, _, cx| {
                        detail.reveal_note(id);
                        cx.notify();
                    }))
                    .child(div().flex_none().text_color(theme::subtle()).child(lines))
                    .child(div().flex_1().min_w_0().truncate().child(first_line))
                    .child(
                        div()
                            .id(element_id("note-delete", &key))
                            .flex_none()
                            .text_color(theme::subtle())
                            .hover(|style| style.text_color(theme::danger()))
                            .on_click(cx.listener(move |detail, _, _, cx| {
                                cx.stop_propagation();
                                detail.delete_note(id, cx);
                            }))
                            .child("×"),
                    ),
            );
        }
        list.into_any_element()
    }
}

/// A quiet explanatory line.
fn muted_line(text: &'static str) -> AnyElement {
    div()
        .text_xs()
        .text_color(theme::subtle())
        .child(text)
        .into_any_element()
}
