//! `PanelToggle`: an icon button that shows or hides a side panel, drawn as
//! a window outline with the panel's edge marked (filled while it shows),
//! like macOS's sidebar buttons.

use gpui::{App, ClickEvent, ElementId, IntoElement, RenderOnce, Window, div, prelude::*, px};

use crate::theme;

/// Size of the clickable square.
const BUTTON_SIZE: f32 = 26.;
/// Width and height of the drawn window outline.
const ICON_WIDTH: f32 = 16.;
const ICON_HEIGHT: f32 = 13.;
/// Width of the panel's strip inside the outline.
const PANEL_STRIP_WIDTH: f32 = 5.;

/// What a click on the toggle runs.
type ClickHandler = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// Which edge of the window the panel sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelEdge {
    Left,
    Right,
}

/// Toggles one side panel; the icon fills the panel's strip while it shows.
#[derive(IntoElement)]
pub struct PanelToggle {
    id: ElementId,
    edge: PanelEdge,
    is_shown: bool,
    is_disabled: bool,
    on_click: Option<ClickHandler>,
}

impl PanelToggle {
    pub fn new(id: impl Into<ElementId>, edge: PanelEdge, is_shown: bool) -> Self {
        Self {
            id: id.into(),
            edge,
            is_shown,
            is_disabled: false,
            on_click: None,
        }
    }

    /// Greys the button out and ignores clicks (no panel to toggle).
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

impl RenderOnce for PanelToggle {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        // The panel's strip: filled while the panel shows, else only its divider line.
        let strip = div()
            .flex_none()
            .w(px(PANEL_STRIP_WIDTH))
            .h_full()
            .border_color(theme::muted())
            .when(self.is_shown, |strip| strip.bg(theme::muted()));
        let strip = match self.edge {
            PanelEdge::Left => strip.border_r_1(),
            PanelEdge::Right => strip.border_l_1(),
        };
        let outline = div()
            .w(px(ICON_WIDTH))
            .h(px(ICON_HEIGHT))
            .flex()
            .when(self.edge == PanelEdge::Right, |outline| {
                outline.justify_end()
            })
            .rounded(px(3.))
            .border_1()
            .border_color(theme::muted())
            .overflow_hidden()
            .child(strip);

        // The clickable square around the icon.
        let button = div()
            .id(self.id)
            .flex_none()
            .size(px(BUTTON_SIZE))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .child(outline);
        match (self.is_disabled, self.on_click) {
            (false, Some(handler)) => button
                .cursor_pointer()
                .hover(|style| style.bg(theme::hover()))
                .on_click(move |event, window, cx| {
                    cx.stop_propagation();
                    handler(event, window, cx);
                }),
            (true, _) | (false, None) => button.opacity(0.35),
        }
    }
}
