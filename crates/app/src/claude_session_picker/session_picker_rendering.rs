use gpui::{
    AnyElement, Context, FontWeight, IntoElement, Render, Window, div, list, prelude::*, px,
};

use super::claude_session_picker::ClaudeSessionPicker;
use super::session_picker_phase::SessionPickerPhase;
use super::session_picker_row::SessionPickerRow;
use crate::controls::{Button, NumberFlow, Pill, PillStyle, SectionLabel, TextTooltip};
use crate::scrollbar::{ScrollbarTarget, VerticalScrollbar};
use crate::text_area::{TextAreaCursorDown, TextAreaCursorUp};
use crate::theme;

const SESSION_NAME_SIZE: f32 = 13.;
const ACTIVITY_DOT_SIZE: f32 = 5.;

impl Render for ClaudeSessionPicker {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Sending and success become a compact receipt with the destination retained.
        let choosing = !matches!(
            self.phase,
            SessionPickerPhase::Sending | SessionPickerPhase::Submitted(_)
        );
        div()
            .id("claude-picker")
            .track_focus(&self.focus)
            // Mapped text-field actions dispatch before raw key events in gpui.
            .capture_action(cx.listener(|picker, _: &TextAreaCursorDown, _, cx| {
                picker.select_next_session(cx);
                cx.stop_propagation();
            }))
            .capture_action(cx.listener(|picker, _: &TextAreaCursorUp, _, cx| {
                picker.select_previous_session(cx);
                cx.stop_propagation();
            }))
            .capture_key_down(cx.listener(Self::on_navigation_key))
            .on_key_down(cx.listener(Self::on_list_key))
            .w_full().min_w_0().min_h_0().flex_1().flex().flex_col().gap_3()
            .pt_3().border_t_1().border_color(theme::border())
            .child(
                div().flex_none().flex().flex_col().gap_3()
                    .child(self.render_header(cx))
                    .when(choosing, |header| header
                        .child(self.search.clone())
                        .child(self.render_result_count(cx))),
            )
            .when(choosing, |panel| panel.child(self.render_sessions(cx)))
            .child(
                div().flex_none().flex().flex_col().gap_3()
                    .child(self.render_destination())
                    .child(self.render_feedback())
                    .child(self.render_send_actions(cx))
                    .when(choosing, |footer| footer.child(div().text_xs().text_color(theme::subtle())
                        .child("Claude will know these notes came from you in Quarterdeck. Replies stay in Claude."))),
            )
    }
}

impl ClaudeSessionPicker {
    /// The same quiet section heading and ghost refresh used in neighboring panels.
    fn render_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let title = match self.phase {
            SessionPickerPhase::Sending => "Sending to Claude",
            SessionPickerPhase::Submitted(_) => "Review submitted",
            _ => "Send to Claude",
        };
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .child(SectionLabel::new(title))
            .when(
                !matches!(
                    self.phase,
                    SessionPickerPhase::Sending | SessionPickerPhase::Submitted(_)
                ),
                |header| {
                    header.child(
                        Button::new("refresh-sessions", "Refresh")
                            .ghost()
                            .disabled(matches!(self.phase, SessionPickerPhase::Loading))
                            .on_click(cx.listener(|picker, _, _, cx| picker.refresh_sessions(cx))),
                    )
                },
            )
            .into_any_element()
    }

    /// Counts make a narrowed or empty list distinguishable from missing sessions.
    fn render_result_count(&self, cx: &mut Context<Self>) -> AnyElement {
        let shown = self.selection.visible().len();
        let total = self.sessions.len();
        let label = match (&self.phase, self.query.trim()) {
            (SessionPickerPhase::Loading, _) => "Finding sessions…".to_owned(),
            (_, "") => format!(
                "{total} {}",
                if total == 1 { "session" } else { "sessions" }
            ),
            _ => format!("{shown} of {total} sessions"),
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .text_xs()
            .text_color(theme::muted())
            .child(NumberFlow::label(label))
            .when(!self.query.is_empty(), |row| {
                row.child(
                    Button::new("clear-session-search", "Clear search")
                        .ghost()
                        .on_click(cx.listener(|picker, _, window, cx| {
                            picker.clear_search(cx);
                            picker.search.read(cx).focus(window);
                        })),
                )
            })
            .into_any_element()
    }

    /// The virtualized list fills the space between the search controls and the send footer.
    fn render_sessions(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.selection.visible().is_empty() {
            let message = match self.phase {
                SessionPickerPhase::Loading => {
                    "Looking for active Claude Code sessions on this Mac…"
                }
                SessionPickerPhase::Failed(_) => return div().into_any_element(),
                _ if !self.sessions.is_empty() => {
                    "No matching sessions. Try another name, repo or branch, or clear the search."
                }
                _ => {
                    "No active sessions with an inbox. Open Claude Code on this Mac, then refresh."
                }
            };
            return div()
                .w_full()
                .flex_1()
                .min_h_0()
                .px_3()
                .py_4()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .text_xs()
                .text_color(theme::muted())
                .child(message)
                .into_any_element();
        }
        div()
            .relative()
            .w_full()
            .flex_1()
            .min_h_0()
            .rounded_md()
            .overflow_hidden()
            .border_1()
            .border_color(theme::border())
            .bg(theme::background())
            .child(
                list(
                    self.scroll.clone(),
                    cx.processor(|picker, row, _, cx| match picker.rows[row] {
                        SessionPickerRow::Repository { group, count } => {
                            picker.render_repository_header(group, count)
                        }
                        SessionPickerRow::Session(index) => picker.render_session(index, cx),
                    }),
                )
                .size_full(),
            )
            .child(VerticalScrollbar::new(
                "claude-sessions-scroll",
                ScrollbarTarget::EstimatedList(self.scroll.clone(), self.row_tops.clone()),
            ))
            .into_any_element()
    }

    /// Repository names lead each group, with matching counts and full paths on hover.
    fn render_repository_header(&self, index: usize, count: usize) -> AnyElement {
        let group = &self.groups[index];
        let details = group.repository.as_ref().map_or_else(
            || "Sessions without an available Git repository".to_owned(),
            |path| path.display().to_string(),
        );
        div()
            .id(("claude-repository", index))
            .w_full()
            .h(px(SessionPickerRow::Repository {
                group: index,
                count,
            }
            .height()))
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .bg(theme::surface())
            .text_xs()
            .text_color(theme::muted())
            .tooltip(TextTooltip::builder(details))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(group.name.clone()),
            )
            .child(Pill::new(count.to_string(), PillStyle::Filled).compact())
            .into_any_element()
    }

    /// Session name leads; the worktree's leaf and activity share a quieter second line.
    fn render_session(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let session = &self.sessions[index];
        let presentation = &self.presentation[index];
        let selected = self.selection.selected() == Some(index);
        let selectable = matches!(self.phase, SessionPickerPhase::Ready);
        div()
            .id(("claude-session", index))
            .w_full()
            .h(px(SessionPickerRow::Session(index).height()))
            .px_3()
            .py_2()
            .flex()
            .flex_col()
            .justify_center()
            .gap_1()
            .min_w_0()
            .text_color(theme::text())
            .when(selected, |row| row.bg(theme::selected()))
            .when(!selectable, |row| row.opacity(0.5))
            .when(selectable, |row| {
                row.cursor_pointer()
                    .when(!selected, |row| row.hover(|style| style.bg(theme::hover())))
                    .on_click(cx.listener(move |picker, _, window, cx| {
                        window.focus(&picker.focus);
                        picker.select_session(index, cx);
                    }))
            })
            .tooltip(TextTooltip::builder(presentation.details.clone()))
            .child(
                div()
                    .w_full()
                    .min_w_0()
                    .truncate()
                    .text_size(px(SESSION_NAME_SIZE))
                    .font_weight(FontWeight::MEDIUM)
                    .child(presentation.name.clone()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .min_w_0()
                    .w_full()
                    .text_xs()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_color(theme::muted())
                            .child(presentation.subtitle.clone()),
                    )
                    .child(render_activity(&session.status)),
            )
            .into_any_element()
    }

    /// A selected destination stays explicit even after search, submission or an error.
    fn render_destination(&self) -> AnyElement {
        let Some(index) = self.selection.selected() else {
            return div()
                .text_xs()
                .text_color(theme::muted())
                .child("Choose a session to receive your notes.")
                .into_any_element();
        };
        let presentation = &self.presentation[index];
        div()
            .id("claude-selected-destination")
            .min_w_0()
            .w_full()
            .flex()
            .flex_col()
            .gap_1()
            .tooltip(TextTooltip::builder(presentation.details.clone()))
            .child(
                div()
                    .min_w_0()
                    .text_sm()
                    .line_clamp(2)
                    .font_weight(FontWeight::MEDIUM)
                    .child(format!("To {}", presentation.name)),
            )
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_xs()
                    .text_color(theme::muted())
                    .child(self.sessions[index].working_directory.display().to_string()),
            )
            .into_any_element()
    }

    /// The note count belongs to the send action; successful sends require a new choice.
    fn render_send_actions(&self, cx: &mut Context<Self>) -> AnyElement {
        if matches!(self.phase, SessionPickerPhase::Submitted(_)) {
            return div()
                .flex()
                .justify_end()
                .child(
                    Button::new("choose-another-session", "Send to another session")
                        .ghost()
                        .on_click(cx.listener(|picker, _, window, cx| {
                            picker.choose_another_session(window, cx);
                        })),
                )
                .into_any_element();
        }
        let label = match self.phase {
            SessionPickerPhase::Sending => "Sending…".to_owned(),
            _ => format!(
                "Send {} {}",
                self.note_count,
                if self.note_count == 1 {
                    "note"
                } else {
                    "notes"
                }
            ),
        };
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .justify_between()
            .gap_2()
            .when(matches!(self.phase, SessionPickerPhase::Ready), |actions| {
                actions.child(
                    div()
                        .text_xs()
                        .text_color(theme::subtle())
                        .child("↑↓ select · Enter sends"),
                )
            })
            .child(
                Button::new("send-review-to-claude", label)
                    .primary()
                    .disabled(!self.can_send())
                    .on_click(cx.listener(|picker, _, _, cx| picker.send_review(cx))),
            )
            .into_any_element()
    }

    /// Submission deliberately stops short of claiming model delivery or completion.
    fn render_feedback(&self) -> AnyElement {
        let (message, color) = match &self.phase {
            SessionPickerPhase::Failed(error) => (error.as_str(), theme::danger()),
            SessionPickerPhase::Submitted(message) => (message.as_str(), theme::success()),
            SessionPickerPhase::Sending => ("Sending your notes…", theme::accent()),
            SessionPickerPhase::Ready | SessionPickerPhase::Loading => {
                return div().into_any_element();
            }
        };
        div()
            .text_xs()
            .text_color(color)
            .child(message.to_owned())
            .into_any_element()
    }
}

/// Activity is secondary metadata, without a badge competing with the session name.
fn render_activity(status: &str) -> AnyElement {
    let (label, color) = match status {
        "busy" => ("Working", theme::accent()),
        "waiting" => ("Needs input", theme::text()),
        "idle" => ("Idle", theme::subtle()),
        _ => ("Active", theme::muted()),
    };
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1p5()
        .text_color(color)
        .child(div().size(px(ACTIVITY_DOT_SIZE)).rounded_full().bg(color))
        .child(label)
        .into_any_element()
}
