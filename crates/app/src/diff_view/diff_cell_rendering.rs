use std::ops::Range;

use gpui::{
    AnyElement, Div, ElementId, HighlightStyle, IntoElement, Rgba, StyledText, div, prelude::*, px,
};
use qc_cache::LineWrapping;
use qc_diff::{DiffLine, DiffLineKind};

use crate::theme;

/// Height of one diff row; rows of a split view in scroll mode line up because all share it.
pub const DIFF_ROW_HEIGHT: f32 = 18.;
/// Approximate width of one digit in the diff's monospaced font.
const DIGIT_WIDTH: f32 = 7.5;
/// Space around the digits of a gutter.
const GUTTER_PADDING: f32 = 16.;
/// Width of the `+`/`-` marker column.
const MARKER_WIDTH: f32 = 14.;

/// The width of a line-number gutter for numbers up to `max_line_number`.
pub fn gutter_width(max_line_number: u32) -> f32 {
    let digits = max_line_number.max(1).ilog10() + 1;
    f32::from(u16::try_from(digits).unwrap_or(u16::MAX)) * DIGIT_WIDTH + GUTTER_PADDING
}

/// A right-aligned line number, or an empty gutter.
pub fn render_gutter(number: Option<u32>, width: f32) -> Div {
    div()
        .flex_none()
        .w(px(width))
        .pr_2()
        .text_right()
        .text_color(theme::muted())
        .children(number.map(|number| number.to_string()))
}

/// The `+`, `-` or blank marker column.
pub fn render_marker(kind: DiffLineKind) -> Div {
    let marker = match kind {
        DiffLineKind::Added => "+",
        DiffLineKind::Removed => "-",
        DiffLineKind::Context => "",
    };
    div()
        .flex_none()
        .w(px(MARKER_WIDTH))
        .text_color(theme::muted())
        .child(marker)
}

/// A line's text with its changed ranges on a stronger background. Wraps
/// within the row, or stays on one line for horizontal scrolling.
pub fn render_code(line: &DiffLine, wrapping: LineWrapping) -> Div {
    let emphasis = emphasis_background(line.kind);
    let highlights = line
        .changed_ranges
        .iter()
        .map(|range: &Range<usize>| {
            (
                range.clone(),
                HighlightStyle {
                    background_color: Some(emphasis.into()),
                    ..HighlightStyle::default()
                },
            )
        })
        .collect::<Vec<_>>();
    let text = StyledText::new(line.text.clone()).with_highlights(highlights);
    match wrapping {
        LineWrapping::Wrap => div().flex_1().min_w_0().pr_2().child(text),
        LineWrapping::Scroll => div().flex_none().pr_4().whitespace_nowrap().child(text),
    }
}

/// The row background for a line of `kind`.
pub fn row_background(kind: DiffLineKind) -> Option<Rgba> {
    match kind {
        DiffLineKind::Added => Some(theme::added_background()),
        DiffLineKind::Removed => Some(theme::removed_background()),
        DiffLineKind::Context => None,
    }
}

/// The background of the changed ranges inside a line of `kind`.
fn emphasis_background(kind: DiffLineKind) -> Rgba {
    match kind {
        DiffLineKind::Added => theme::added_emphasis(),
        DiffLineKind::Removed => theme::removed_emphasis(),
        DiffLineKind::Context => theme::code_background(),
    }
}

/// A row with a line's background; empty rows keep the row height.
pub fn render_row(kind: Option<DiffLineKind>) -> Div {
    let row = div().flex().min_h(px(DIFF_ROW_HEIGHT));
    match kind.map(row_background) {
        Some(Some(background)) => row.bg(background),
        Some(None) => row,
        // No line on this side of a split row.
        None => row.bg(theme::diff_filler()),
    }
}

/// The `@@ … @@` header that starts a hunk; empty text keeps the row for alignment.
pub fn render_hunk_header(header: &str, indent: f32) -> Div {
    render_row(None)
        .pl(px(indent))
        .text_color(theme::muted())
        .whitespace_nowrap()
        .overflow_hidden()
        .child(header.to_owned())
}

/// Stacks `rows` as a body: wrapped rows fill the width; scrolling rows sit
/// in a horizontal scroll area whose content is as wide as its longest row,
/// so line backgrounds reach the end of every line.
pub fn render_body(rows: Vec<Div>, wrapping: LineWrapping, id: ElementId) -> AnyElement {
    match wrapping {
        LineWrapping::Wrap => div().flex().flex_col().children(rows).into_any_element(),
        LineWrapping::Scroll => div()
            .id(id)
            .overflow_x_scroll()
            .flex()
            .child(
                div()
                    .flex_none()
                    .min_w_full()
                    .flex()
                    .flex_col()
                    .children(rows),
            )
            .into_any_element(),
    }
}
