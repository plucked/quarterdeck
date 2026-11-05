//! `Button`: a small text button, plain or primary, that can be disabled.

use gpui::{
    App, ClickEvent, ElementId, IntoElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::*,
};

use crate::theme;

/// What a button does when clicked.
type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// How a button looks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonKind {
    Plain,
    /// The main action of its group.
    Primary,
    /// Removes something.
    Danger,
}

/// A small bordered text button. `id` must be unique among its siblings.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    kind: ButtonKind,
    is_disabled: bool,
    on_click: Option<ClickHandler>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: ButtonKind::Plain,
            is_disabled: false,
            on_click: None,
        }
    }

    /// Shows it as the main action.
    #[must_use]
    pub fn primary(mut self) -> Self {
        self.kind = ButtonKind::Primary;
        self
    }

    /// Shows it as removing something.
    #[must_use]
    pub fn danger(mut self) -> Self {
        self.kind = ButtonKind::Danger;
        self
    }

    /// Greys it out and ignores clicks when `is_disabled`.
    #[must_use]
    pub fn disabled(mut self, is_disabled: bool) -> Self {
        self.is_disabled = is_disabled;
        self
    }

    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let (background, text, border) = match self.kind {
            ButtonKind::Primary => (Some(theme::accent()), theme::background(), theme::accent()),
            ButtonKind::Danger => (None, theme::removed(), theme::border()),
            ButtonKind::Plain => (None, theme::text(), theme::border()),
        };
        let button = div()
            .id(self.id)
            .flex_none()
            .px_2()
            .py_0p5()
            .rounded_md()
            .border_1()
            .border_color(border)
            .text_xs()
            .text_color(text)
            .when_some(background, Styled::bg)
            .child(self.label);
        match (self.is_disabled, self.on_click) {
            (false, Some(handler)) => button
                .cursor_pointer()
                .hover(|style| style.opacity(0.85))
                .on_click(move |event, window, cx| {
                    cx.stop_propagation();
                    handler(event, window, cx);
                }),
            (true, _) | (false, None) => button.opacity(0.45),
        }
    }
}
