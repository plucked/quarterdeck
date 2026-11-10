//! `Button`: a small text button (ghost, plain, primary or danger) that can
//! be disabled.

use gpui::{
    App, ClickEvent, ElementId, FontWeight, IntoElement, RenderOnce, SharedString, Styled, Window,
    div, prelude::*,
};

use crate::theme;

/// Opacity of a primary button under the pointer (it has no hover color).
const PRIMARY_HOVER_OPACITY: f32 = 0.88;
/// Opacity of a disabled button (or one that does nothing).
const DISABLED_OPACITY: f32 = 0.4;

/// What a button does when clicked.
type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// How a button looks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ButtonKind {
    /// Outlined on a raised surface: most actions.
    Plain,
    /// Text only until hovered: toolbar actions.
    Ghost,
    /// The main action of its group.
    Primary,
    /// Removes something.
    Danger,
}

/// A small text button. `id` must be unique among its siblings.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    kind: ButtonKind,
    is_disabled: bool,
    is_selected: bool,
    on_click: Option<ClickHandler>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: ButtonKind::Plain,
            is_disabled: false,
            is_selected: false,
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

    /// Shows it as text only until hovered, for toolbars.
    #[must_use]
    pub fn ghost(mut self) -> Self {
        self.kind = ButtonKind::Ghost;
        self
    }

    /// Shows it pressed (its panel is open).
    #[must_use]
    pub fn selected(mut self, is_selected: bool) -> Self {
        self.is_selected = is_selected;
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
        let (background, text, border) = match (self.kind, self.is_selected) {
            (ButtonKind::Primary, _) => {
                (Some(theme::accent()), theme::on_accent(), theme::accent())
            }
            (ButtonKind::Danger, _) => (
                Some(theme::surface()),
                theme::danger(),
                theme::border_strong(),
            ),
            (ButtonKind::Plain | ButtonKind::Ghost, true) => {
                (Some(theme::selected()), theme::text(), theme::selected())
            }
            (ButtonKind::Plain, false) => (
                Some(theme::surface()),
                theme::text(),
                theme::border_strong(),
            ),
            (ButtonKind::Ghost, false) => (None, theme::muted(), gpui::transparent_black().into()),
        };
        let kind = self.kind;
        let button = div()
            .id(self.id)
            .flex_none()
            .flex()
            .items_center()
            .px_2()
            .py_0p5()
            .rounded_md()
            .border_1()
            .border_color(border)
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .text_color(text)
            .when_some(background, Styled::bg)
            .child(self.label);
        match (self.is_disabled, self.on_click) {
            (false, Some(handler)) => button
                .cursor_pointer()
                .hover(move |style| match kind {
                    ButtonKind::Ghost => style.bg(theme::hover()).text_color(theme::text()),
                    ButtonKind::Plain | ButtonKind::Danger => style.bg(theme::hover()),
                    ButtonKind::Primary => style.opacity(PRIMARY_HOVER_OPACITY),
                })
                .on_click(move |event, window, cx| {
                    cx.stop_propagation();
                    handler(event, window, cx);
                }),
            (true, _) | (false, None) => button.opacity(DISABLED_OPACITY),
        }
    }
}
