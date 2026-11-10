use gpui::{AnyElement, IntoElement, div, prelude::*};
use qd_studio::ReaderFamiliarity;

use super::brief_handlers::BriefHandlers;
use crate::section_label::SectionLabel;
use crate::segmented_control::SegmentedControl;
use crate::theme;

/// The question how well the reader knows the area, its three answers as
/// a segmented control (`current` selected), and what the selected one
/// does to the brief. `id` keeps two pickers on one page apart.
pub fn render_familiarity_picker(
    id: &'static str,
    current: ReaderFamiliarity,
    handlers: &BriefHandlers,
) -> AnyElement {
    let control = ReaderFamiliarity::ALL.into_iter().fold(
        SegmentedControl::new(id),
        |control, familiarity| {
            let choose_reader = handlers.choose_reader.clone();
            control.option(
                familiarity.label(),
                familiarity == current,
                move |window, cx| choose_reader(familiarity, window, cx),
            )
        },
    );
    div()
        .flex()
        .flex_col()
        .gap_1p5()
        .child(SectionLabel::new("How well do you know this area?"))
        .child(div().flex().child(control))
        .child(
            div()
                .text_xs()
                .text_color(theme::muted())
                .child(current.effect()),
        )
        .into_any_element()
}
