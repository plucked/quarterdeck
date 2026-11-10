use gpui::{AnyElement, IntoElement, div, prelude::*};
use qd_studio::{AgentProgress, ReaderFamiliarity};

use super::brief_handlers::{BriefHandlers, on_click};
use super::brief_state::BriefState;
use super::familiarity_picker::render_familiarity_picker;
use crate::button::Button;
use crate::counted::counted;
use crate::section_label::SectionLabel;
use crate::theme;

/// Characters per thousand, for the writing progress.
const CHARACTERS_PER_THOUSAND: f64 = 1000.;

/// No brief yet (what it is, and a button), one being written (how far,
/// and Cancel) or a failed run (why, and Try again). `can_write` is false
/// while there are no files to send.
pub fn render_brief_status(
    state: Option<&BriefState>,
    can_write: bool,
    reader: ReaderFamiliarity,
    handlers: &BriefHandlers,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .rounded_md()
        .bg(theme::surface())
        .border_1()
        .border_color(theme::border_strong());
    match state {
        None => card
            .child(SectionLabel::new("Brief"))
            .child(div().text_sm().text_color(theme::muted()).child(
                "Claude Code reads this change's diffs and a shallow checkout of its head \
                 commit, and walks you through it: why it exists, the context you \
                 need, what changes, the order to read it in and how it works, with every \
                 claim tied to lines you can open in the diff. It can only read files in that checkout, which \
                 is deleted when it is done; it cannot run or change anything. The code \
                 it reads is sent to Anthropic.",
            ))
            .child(render_familiarity_picker("brief-reader", reader, handlers))
            .child(
                div().flex().child(
                    Button::new("write-brief", "Write brief")
                        .primary()
                        .disabled(!can_write)
                        .on_click(on_click(&handlers.write)),
                ),
            )
            .into_any_element(),
        Some(BriefState::Writing(progress)) => card
            .child(SectionLabel::new("Writing the brief"))
            .child(
                div()
                    .text_sm()
                    .text_color(theme::muted())
                    .child(progress_text(*progress)),
            )
            .child(
                div().flex().child(
                    Button::new("cancel-brief", "Cancel")
                        .ghost()
                        .on_click(on_click(&handlers.cancel)),
                ),
            )
            .into_any_element(),
        Some(BriefState::Failed(message)) => card
            .child(SectionLabel::new("No brief"))
            .child(
                div()
                    .text_sm()
                    .text_color(theme::danger())
                    .child(message.clone()),
            )
            .child(render_familiarity_picker("brief-reader", reader, handlers))
            .child(
                div().flex().child(
                    Button::new("retry-brief", "Try again")
                        .disabled(!can_write)
                        .on_click(on_click(&handlers.write)),
                ),
            )
            .into_any_element(),
        Some(BriefState::Written(_)) => div().into_any_element(),
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
