use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*};

use crate::theme;

/// The note shown in place of an empty list on a settings page (`None yet.
/// …`), saying what having none means.
#[derive(IntoElement)]
pub struct EmptyListNote {
    text: SharedString,
}

impl EmptyListNote {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl RenderOnce for EmptyListNote {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .py_3()
            .text_xs()
            .text_color(theme::muted())
            .child(self.text)
    }
}
