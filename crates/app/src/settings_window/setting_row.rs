use gpui::{AnyElement, App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*};

use crate::controls::NumberFlow;
use crate::theme;

/// Width of the text field a setting's control holds (a token, a
/// repository, a branch pattern), and of what lines up under it.
pub const SETTING_FIELD_WIDTH: f32 = 280.;

/// One setting on a settings page: its title and a short explanation on the
/// left, the control that changes it on the right.
#[derive(IntoElement)]
pub struct SettingRow {
    id: gpui::ElementId,
    title: SharedString,
    description: SharedString,
    control: AnyElement,
}

impl SettingRow {
    #[track_caller]
    pub fn new(
        title: impl Into<SharedString>,
        description: impl Into<SharedString>,
        control: impl IntoElement,
    ) -> Self {
        Self {
            id: gpui::ElementId::CodeLocation(*std::panic::Location::caller()),
            title: title.into(),
            description: description.into(),
            control: control.into_any_element(),
        }
    }
}

impl RenderOnce for SettingRow {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id(self.id)
            .flex()
            .items_center()
            .gap_4()
            .py_3()
            .border_b_1()
            .border_color(theme::border())
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap_0p5()
                    .child(NumberFlow::new("setting-title", self.title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme::muted())
                            .child(NumberFlow::new("setting-description", self.description)),
                    ),
            )
            .child(div().flex_none().child(self.control))
    }
}
