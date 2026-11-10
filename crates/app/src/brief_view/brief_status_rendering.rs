use gpui::{AnyElement, IntoElement, Rgba, SharedString, div, prelude::*};
use qd_studio::{AgentProgress, ReaderFamiliarity};

use super::brief_card::brief_card;
use super::brief_handlers::{BriefHandlers, on_click};
use super::brief_state::BriefState;
use super::familiarity_picker::render_familiarity_picker;
use crate::controls::{Button, SectionLabel};
use crate::counted::counted;
use crate::theme;

/// Characters per thousand, for the writing progress.
const CHARACTERS_PER_THOUSAND: f64 = 1000.;
/// What the card says before a brief is written: what writing one does.
const INTRO_TEXT: &str = "Claude Code reads this change's diffs and a shallow checkout of its head \
     commit, and walks you through it: why it exists, the context you \
     need, what changes, the order to read it in and how it works, with every \
     claim tied to lines you can open in the diff. It can only read files in that checkout, which \
     is deleted when it is done; it cannot run or change anything. The code \
     it reads is sent to Anthropic.";

/// What a status card shows: its label, its text, whether the reader picks
/// how well they know the area, and its button.
struct StatusCardContent {
    label: &'static str,
    text: SharedString,
    text_color: Rgba,
    reader_picker: ReaderPicker,
    button: StatusButton,
}

/// Whether a status card asks how well the reader knows the area.
enum ReaderPicker {
    Shown,
    Hidden,
}

/// A status card's button.
struct StatusButton {
    id: &'static str,
    label: &'static str,
    look: StatusButtonLook,
    action: StatusAction,
    is_disabled: bool,
}

/// How a status card's button looks.
enum StatusButtonLook {
    /// The main action: writing a first brief.
    Primary,
    /// Quiet, beside a run in progress.
    Ghost,
    /// Outlined: trying again after a failure.
    Plain,
}

/// What a status card's button does.
enum StatusAction {
    Write,
    Cancel,
}

/// No brief yet (what it is, and a button), one being written (how far,
/// and Cancel) or a failed run (why, and Try again). `can_write` is false
/// while there are no files to send.
pub fn render_brief_status(
    state: Option<&BriefState>,
    can_write: bool,
    reader: ReaderFamiliarity,
    handlers: &BriefHandlers,
) -> AnyElement {
    let Some(content) = status_card_content(state, can_write) else {
        return div().into_any_element();
    };
    // The button, styled and wired as the state asks.
    let button = Button::new(content.button.id, content.button.label)
        .disabled(content.button.is_disabled)
        .on_click(on_click(match content.button.action {
            StatusAction::Write => &handlers.write,
            StatusAction::Cancel => &handlers.cancel,
        }));
    let button = match content.button.look {
        StatusButtonLook::Primary => button.primary(),
        StatusButtonLook::Ghost => button.ghost(),
        StatusButtonLook::Plain => button,
    };
    // The card: label, text, the reader picker when asked, then the button.
    brief_card()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .child(SectionLabel::new(content.label))
        .child(
            div()
                .text_sm()
                .text_color(content.text_color)
                .child(content.text),
        )
        .when(
            matches!(content.reader_picker, ReaderPicker::Shown),
            |card| card.child(render_familiarity_picker("brief-reader", reader, handlers)),
        )
        .child(div().flex().child(button))
        .into_any_element()
}

/// What the status card shows for `state`; `None` once a brief is written,
/// which shows in its place.
fn status_card_content(state: Option<&BriefState>, can_write: bool) -> Option<StatusCardContent> {
    match state {
        None => Some(StatusCardContent {
            label: "Brief",
            text: INTRO_TEXT.into(),
            text_color: theme::muted(),
            reader_picker: ReaderPicker::Shown,
            button: StatusButton {
                id: "write-brief",
                label: "Write brief",
                look: StatusButtonLook::Primary,
                action: StatusAction::Write,
                is_disabled: !can_write,
            },
        }),
        Some(BriefState::Writing(progress)) => Some(StatusCardContent {
            label: "Writing the brief",
            text: progress_text(*progress).into(),
            text_color: theme::muted(),
            reader_picker: ReaderPicker::Hidden,
            button: StatusButton {
                id: "cancel-brief",
                label: "Cancel",
                look: StatusButtonLook::Ghost,
                action: StatusAction::Cancel,
                is_disabled: false,
            },
        }),
        Some(BriefState::Failed(message)) => Some(StatusCardContent {
            label: "No brief",
            text: message.clone(),
            text_color: theme::danger(),
            reader_picker: ReaderPicker::Shown,
            button: StatusButton {
                id: "retry-brief",
                label: "Try again",
                look: StatusButtonLook::Plain,
                action: StatusAction::Write,
                is_disabled: !can_write,
            },
        }),
        Some(BriefState::Written(_)) => None,
    }
}

/// What the status row says while the agent works.
fn progress_text(progress: Option<AgentProgress>) -> String {
    match progress {
        None => "Starting Claude Code…".to_owned(),
        Some(AgentProgress::FetchingCode { percent: None }) => {
            "Checking out the head commit…".to_owned()
        }
        Some(AgentProgress::FetchingCode {
            percent: Some(percent),
        }) => format!("Checking out the head commit… {percent}%"),
        Some(AgentProgress::UnpackingCode) => "Unpacking the code…".to_owned(),
        Some(AgentProgress::Started) => "Claude is reading the change…".to_owned(),
        Some(AgentProgress::Thinking) => "Claude is working through the change…".to_owned(),
        Some(AgentProgress::Exploring { tool_calls }) => format!(
            "Claude is reading the code… {} so far",
            counted(tool_calls, "look", "looks")
        ),
        Some(AgentProgress::Writing { characters }) => {
            let thousands =
                f64::from(u32::try_from(characters).unwrap_or(u32::MAX)) / CHARACTERS_PER_THOUSAND;
            format!("Writing the brief… {thousands:.1}k characters")
        }
    }
}
