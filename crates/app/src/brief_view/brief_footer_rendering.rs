use gpui::{AnyElement, IntoElement, div, prelude::*};
use qd_studio::{CodeAccess, ReaderFamiliarity, RejectedWidget};

use super::brief_handlers::{BriefHandlers, on_click};
use super::familiarity_picker::render_familiarity_picker;
use super::prepared_brief::PreparedBrief;
use crate::controls::Button;
use crate::counted::counted;
use crate::theme;

/// Who wrote the brief, for which reader and what it cost, and writing it
/// again for the reader now picked (`reader`).
pub fn render_brief_footer(
    brief: &PreparedBrief,
    reader: ReaderFamiliarity,
    handlers: &BriefHandlers,
) -> AnyElement {
    let cost = brief
        .record
        .cost_usd
        .map(|cost| format!(" · ${cost:.2}"))
        .unwrap_or_default();
    div()
        .flex()
        .flex_col()
        .gap_3()
        .pt_4()
        .border_t_1()
        .border_color(theme::border())
        .child(
            div()
                .text_xs()
                .text_color(theme::subtle())
                .child(written_from(&brief.record.code, brief.record.reader, &cost)),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_end()
                .justify_between()
                .gap_3()
                .child(render_familiarity_picker(
                    "brief-rewrite-reader",
                    reader,
                    handlers,
                ))
                .child(
                    Button::new("rewrite-brief", "Write again")
                        .ghost()
                        .on_click(on_click(&handlers.write)),
                ),
        )
        .into_any_element()
}

/// What the footer says the brief was written from, for whom, and what it cost.
fn written_from(code: &CodeAccess, reader: ReaderFamiliarity, cost: &str) -> String {
    let for_whom = format!("for a reader who {}", reader.reader_description());
    match code {
        CodeAccess::Workspace => format!(
            "Written by Claude Code {for_whom}, from the diffs and a checkout of the head commit{cost}. It read code but ran nothing; check claims against the code."
        ),
        CodeAccess::DiffOnly(reason) => format!(
            "Written by Claude Code {for_whom}, from the diffs alone ({reason}){cost}. It ran nothing; check claims against the code."
        ),
    }
}

/// The widgets left out, each with why, so a thin brief is explained.
pub fn render_rejected(rejected: &[RejectedWidget]) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .pt_3()
        .text_xs()
        .text_color(theme::subtle())
        .child(format!(
            "Left out: {} of the brief pointed at lines outside the diff or was malformed.",
            counted(rejected.len(), "part", "parts")
        ))
        .children(
            rejected
                .iter()
                .map(|rejected| div().child(format!("{}: {}", rejected.widget, rejected.reason))),
        )
        .into_any_element()
}
