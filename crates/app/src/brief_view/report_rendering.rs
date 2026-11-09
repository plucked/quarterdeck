use gpui::{AnyElement, Entity, FontWeight, IntoElement, div, prelude::*};

use super::brief_handlers::{BriefHandlers, on_click};
use super::report_state::ReportState;
use crate::button::Button;
use crate::text_area::TextArea;
use crate::theme;

/// Reporting the brief, under the version line once its Report button is
/// clicked: the question, a text field and Save / Cancel while written; the
/// saved report's path with Copy path and Done once saved; `None` while closed.
pub fn render_report_panel(state: &ReportState, handlers: &BriefHandlers) -> Option<AnyElement> {
    let panel = match state {
        ReportState::Closed => return None,
        ReportState::Writing(editor) => report_form(editor, None, handlers),
        ReportState::Failed { message, editor } => {
            report_form(editor, Some(message.clone().into()), handlers)
        }
        ReportState::Saving => panel()
            .child(
                div()
                    .text_sm()
                    .text_color(theme::muted())
                    .child("Saving the report…"),
            )
            .into_any_element(),
        ReportState::Saved(path) => panel()
            .child(label("REPORT SAVED"))
            .child(
                div()
                    .text_sm()
                    .text_color(theme::muted())
                    .child("Give this path to a Claude Code session in the Quarterdeck repository and ask it to suggest improvements to the brief tool:"),
            )
            .child(
                div()
                    .font_family(theme::MONO_FONT)
                    .text_xs()
                    .child(path.clone()),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("copy-report-path", "Copy path")
                            .primary()
                            .on_click(on_click(&handlers.copy_report_path)),
                    )
                    .child(
                        Button::new("close-report", "Done")
                            .ghost()
                            .on_click(on_click(&handlers.close_report)),
                    ),
            )
            .into_any_element(),
    };
    Some(panel)
}

/// The question, what a report holds, the text field, a failure's reason
/// when there is one, and Save report / Cancel.
fn report_form(
    editor: &Entity<TextArea>,
    failure: Option<String>,
    handlers: &BriefHandlers,
) -> AnyElement {
    panel()
        .child(label("WHAT IS WRONG OR WEIRD IN THIS BRIEF?"))
        .child(
            div()
                .text_xs()
                .text_color(theme::muted())
                .child("Saved to a local folder with everything about how the brief was made: the prompt, the guide and schema, Claude's raw answer, what the checks left out and what it read. Nothing is sent anywhere."),
        )
        .child(editor.clone())
        .children(failure.map(|failure| {
            div()
                .text_xs()
                .text_color(theme::error())
                .child(failure)
        }))
        .child(
            div()
                .flex()
                .gap_2()
                .child(
                    Button::new("save-report", "Save report")
                        .primary()
                        .on_click(on_click(&handlers.save_report)),
                )
                .child(
                    Button::new("cancel-report", "Cancel")
                        .ghost()
                        .on_click(on_click(&handlers.close_report)),
                ),
        )
        .into_any_element()
}

/// The panel's card.
fn panel() -> gpui::Div {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .p_3()
        .rounded_md()
        .bg(theme::surface())
        .border_1()
        .border_color(theme::border_strong())
}

/// A small uppercase label heading the panel.
fn label(text: &'static str) -> gpui::Div {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::subtle())
        .child(text)
}
