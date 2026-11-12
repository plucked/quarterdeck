//! `TextTooltip`: a short text shown while the pointer rests on an element.

use gpui::{
    AnyView, App, AppContext, Context, IntoElement, Render, SharedString, Window, div, prelude::*,
};

use crate::theme;

/// The tooltip view gpui shows over an element (`.tooltip(…)` takes a
/// view): one line of text on a card.
pub struct TextTooltip {
    text: SharedString,
}

impl TextTooltip {
    /// A builder for `.tooltip(…)` showing `text`.
    pub fn builder(
        text: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        let text = text.into();
        move |_, cx| {
            let text = text.clone();
            cx.new(|_| Self { text }).into()
        }
    }
}

impl Render for TextTooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .max_w(gpui::px(360.))
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(theme::border_strong())
            .bg(theme::surface())
            .text_xs()
            .text_color(theme::text())
            .child(self.text.clone())
    }
}
