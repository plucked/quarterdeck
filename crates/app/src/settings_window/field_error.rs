use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*};

use crate::theme;

/// Why what was typed into a settings field could not be used, in the
/// danger color under the field.
#[derive(IntoElement)]
pub struct FieldError {
    reason: SharedString,
}

impl FieldError {
    pub fn new(reason: impl Into<SharedString>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl RenderOnce for FieldError {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .text_xs()
            .text_color(theme::danger())
            .child(self.reason)
    }
}
