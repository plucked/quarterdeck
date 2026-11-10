use gpui::{AnyElement, App, ElementId, IntoElement, RenderOnce, Window, div, prelude::*};

use crate::controls::Button;
use crate::theme;

/// What a row's Remove button does.
type RemoveHandler = Box<dyn Fn(&mut App)>;

/// One row of a list on a settings page: what the row shows, then a ghost
/// Remove button, over a divider (a watched repository, a hidden branch rule).
#[derive(IntoElement)]
pub struct RemovableRow {
    /// The Remove button's id, unique among the list's rows.
    remove_id: ElementId,
    content: Vec<AnyElement>,
    on_remove: RemoveHandler,
}

impl RemovableRow {
    pub fn new(remove_id: impl Into<ElementId>, on_remove: impl Fn(&mut App) + 'static) -> Self {
        Self {
            remove_id: remove_id.into(),
            content: Vec::new(),
            on_remove: Box::new(on_remove),
        }
    }

    /// Adds `element` to what the row shows before its Remove button.
    #[must_use]
    pub fn child(mut self, element: impl IntoElement) -> Self {
        self.content.push(element.into_any_element());
        self
    }
}

impl RenderOnce for RemovableRow {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let on_remove = self.on_remove;
        div()
            .flex()
            .items_center()
            .gap_2()
            .py_1p5()
            .border_b_1()
            .border_color(theme::border())
            .children(self.content)
            .child(
                Button::new(self.remove_id, "Remove")
                    .ghost()
                    .on_click(move |_, _, cx| on_remove(cx)),
            )
    }
}
