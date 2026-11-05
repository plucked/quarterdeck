use gpui::{Div, IntoElement, div, prelude::*, px};

use crate::theme;

/// Height of the strip that closes a file's card below its last row.
const FILE_END_HEIGHT: f32 = 4.;

/// Frames a row inside a file's card: the card's side borders. Each row of
/// the virtualized detail list draws its own piece of the card.
pub fn frame_file_row(row: Div) -> Div {
    row.border_x_1().border_color(theme::border())
}

/// The strip closing a file's card, with the gap below it to the next file.
pub fn render_file_end() -> impl IntoElement {
    div().pb_3().child(
        div()
            .h(px(FILE_END_HEIGHT))
            .border_x_1()
            .border_b_1()
            .rounded_b_md()
            .border_color(theme::border()),
    )
}
