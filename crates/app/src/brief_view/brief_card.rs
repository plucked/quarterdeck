use gpui::{Div, div, prelude::*};

use crate::theme;

/// The look of a brief's cards (the status card, the report panel): a
/// raised surface with a strong border and rounded corners. Callers add
/// layout and padding.
pub fn brief_card() -> Div {
    div()
        .rounded_md()
        .bg(theme::surface())
        .border_1()
        .border_color(theme::border_strong())
}
