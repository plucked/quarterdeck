use gpui::{AnyElement, IntoElement, div, prelude::*};
use qd_studio::AgentProgress;

use super::brief_handlers::{BriefHandlers, on_click};
use super::brief_save_status::BriefSaveStatus;
use super::brief_status_rendering::progress_text;
use crate::controls::{Button, PulsingStatus};
use crate::theme;

/// Replacement status above the still-readable saved brief, with a recovery action.
pub fn render_brief_refresh(
    is_writing: bool,
    save_status: BriefSaveStatus,
    progress: Option<AgentProgress>,
    error: Option<&str>,
    handlers: &BriefHandlers,
) -> Option<AnyElement> {
    // Keep cancellation available while generation or local saving is active.
    if is_writing || save_status == BriefSaveStatus::Saving {
        let progress = match save_status {
            BriefSaveStatus::Saving => "Saving brief…".to_owned(),
            BriefSaveStatus::None | BriefSaveStatus::Pending => progress_text(progress),
        };
        return Some(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(PulsingStatus::new("brief-refresh-progress", progress))
                .child(
                    Button::new("cancel-brief-refresh", "Cancel")
                        .ghost()
                        .on_click(on_click(&handlers.cancel)),
                )
                .into_any_element(),
        );
    }

    // A failed save retries the completed brief; generation failures start a new run.
    let error = match (error, save_status) {
        (Some(error), _) => error,
        (None, BriefSaveStatus::Pending) => "The new brief could not be saved.",
        (None, BriefSaveStatus::None | BriefSaveStatus::Saving) => return None,
    };
    let recovery = match save_status {
        BriefSaveStatus::Pending => div()
            .flex()
            .flex_wrap()
            .gap_2()
            .child(
                Button::new("retry-brief-refresh-save", "Retry save")
                    .on_click(on_click(&handlers.retry_save)),
            )
            .child(
                Button::new("discard-brief-refresh-save", "Discard")
                    .ghost()
                    .on_click(on_click(&handlers.cancel)),
            ),
        BriefSaveStatus::None | BriefSaveStatus::Saving => div().child(
            Button::new("retry-brief-refresh", "Try again")
                .ghost()
                .on_click(on_click(&handlers.write)),
        ),
    };
    Some(
        div()
            .flex()
            .flex_col()
            .gap_1()
            .text_sm()
            .child(div().text_color(theme::danger()).child(error.to_owned()))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme::muted())
                            .child("Your saved brief is still available."),
                    )
                    .child(recovery),
            )
            .when(save_status == BriefSaveStatus::Pending, |notice| {
                notice.child(
                    div()
                        .text_xs()
                        .text_color(theme::muted())
                        .child("Retry save uses the brief already written."),
                )
            })
            .into_any_element(),
    )
}
