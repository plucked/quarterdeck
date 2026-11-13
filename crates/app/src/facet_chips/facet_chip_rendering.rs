use gpui::{AnyElement, Div, FontWeight, SharedString, Stateful, div, prelude::*, px};

use crate::controls::{NumberFlow, SectionLabel};
use crate::theme;

/// What a chip row says while no chip is set: how the chips work.
pub const CHIP_HINT: &str = "Click to show only · ⌥-click to hide";
/// Width of a chip row's label column, so the rows' chips line up.
const ROW_LABEL_WIDTH: f32 = 50.;
/// Nudges the label down to the chips' text line.
const ROW_LABEL_TOP: f32 = 3.;

/// A row of chips after its small uppercase label.
pub fn render_chip_row(label: &'static str, chips: Vec<AnyElement>) -> Div {
    div()
        .flex()
        .items_start()
        .gap_1()
        .child(
            div()
                .flex_none()
                .w(px(ROW_LABEL_WIDTH))
                .pt(px(ROW_LABEL_TOP))
                .child(
                    SectionLabel::new(label)
                        .compact()
                        .weight(FontWeight::MEDIUM),
                ),
        )
        // Lines start at the top and keep their own height, so a chip never stretches.
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_wrap()
                .items_start()
                .content_start()
                .gap_1()
                .children(chips),
        )
}

/// The `+3 more` / `less` link closing a chip row; the caller adds the click.
pub fn render_more_chip(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
) -> Stateful<Div> {
    div()
        .id(id)
        .px_1()
        .py_0p5()
        .text_xs()
        .text_color(theme::accent())
        .cursor_pointer()
        .child(NumberFlow::label(label))
}
