use gpui::{AnyElement, App, IntoElement, Styled, canvas};

use super::brief_rendering::BRIEF_SIDE_PADDING;

/// Narrower columns are not reported: the list before its first layout.
const MIN_REPORTED_WIDTH: f32 = 120.;

/// An invisible layer over the brief's list that measures the column (the
/// list's width less the side padding) when it is laid out and calls
/// `on_change` with it, after the frame, when it differs from `known` by a
/// pixel or more. The showing view then lays the diagrams out for it, so
/// they follow the width the reader gives the pane.
pub fn render_column_width_probe(
    known: f32,
    on_change: impl Fn(f32, &mut App) + 'static,
) -> AnyElement {
    canvas(
        move |bounds, _, cx| {
            let width = f32::from(bounds.size.width) - 2. * BRIEF_SIDE_PADDING;
            if width >= MIN_REPORTED_WIDTH && (width - known).abs() >= 1. {
                cx.defer(move |cx| on_change(width, cx));
            }
        },
        |_, (), _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
    .into_any_element()
}
