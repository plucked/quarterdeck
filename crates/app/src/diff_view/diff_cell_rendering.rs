use gpui::{CursorStyle, Div, Rgba, StyledText, canvas, div, prelude::*, px};
use qd_cache::LineWrapping;
use qd_diff::{DiffLine, DiffLineKind};
use qd_syntax::SyntaxSpan;

use super::code_highlights::code_highlights;
use super::file_card_rendering::frame_file_row;
use super::{CodeCells, CodeSegment, DiffRowStyle, DiffSide};
use crate::controls::NumberFlow;
use crate::count_as_f32::count_as_f32;
use crate::theme;

/// Height of one unwrapped diff row.
pub const DIFF_ROW_HEIGHT: f32 = 18.;
/// Approximate width of one character in the diff's monospaced font.
pub const DIFF_CHARACTER_WIDTH: f32 = 7.2;
/// Approximate width of one digit, with a little room.
const DIGIT_WIDTH: f32 = 7.5;
/// Space around the digits of a gutter.
const GUTTER_PADDING: f32 = 16.;
/// Width of the `+`/`-` marker column.
const MARKER_WIDTH: f32 = 14.;

/// The width of a line-number gutter for numbers up to `max_line_number`.
pub fn gutter_width(max_line_number: u32) -> f32 {
    let digits = max_line_number.max(1).ilog10() + 1;
    count_as_f32(digits) * DIGIT_WIDTH + GUTTER_PADDING
}

/// A right-aligned line number, or an empty gutter.
pub fn render_gutter(id: &'static str, number: Option<u32>, width: f32) -> Div {
    div()
        .flex_none()
        .w(px(width))
        .pr_2()
        .text_right()
        .text_color(theme::line_number())
        .children(number.map(|number| NumberFlow::new(id, number.to_string())))
}

/// The `+`, `-` or blank marker column.
pub fn render_marker(kind: DiffLineKind) -> Div {
    // The mark carries the change's color too, so added and removed differ
    // by symbol as well as by hue.
    let (marker, color) = match kind {
        DiffLineKind::Added => ("+", theme::added()),
        DiffLineKind::Removed => ("-", theme::removed()),
        DiffLineKind::Context => ("", theme::muted()),
    };
    div()
        .flex_none()
        .w(px(MARKER_WIDTH))
        .text_color(color)
        .child(marker)
}

/// A line's text in its syntax colors, with its changed ranges on a
/// stronger background and its selected part on the selection background.
/// Wraps within the row, or stays on one line clipped to the row and
/// shifted left by the file's horizontal offset. The text registers itself
/// in `cells.segments` when painted, under its row and column `side`, so the
/// mouse can select it.
pub fn render_code(
    line: &DiffLine,
    syntax: &[SyntaxSpan],
    style: DiffRowStyle,
    cells: &CodeCells,
    side: DiffSide,
) -> Div {
    let selected = cells.selected_range(side);
    let highlights = code_highlights(
        line.text.len(),
        syntax,
        &line.changed_ranges,
        emphasis_background(line.kind),
        selected.as_ref(),
    );
    let text = StyledText::new(line.text.clone()).with_highlights(highlights);

    // Paint runs after layout, so the layout registered here has its bounds.
    let segment = CodeSegment {
        row: cells.row,
        side,
        layout: text.layout().clone(),
        text_length: line.text.len(),
    };
    let segments = cells.segments.clone();
    let registration = canvas(|_, _, _| {}, move |_, (), _, _| segments.register(segment))
        .absolute()
        .size_full();

    let cell = match style.wrapping {
        LineWrapping::Wrap => div().flex_1().min_w_0().pr_2().child(text),
        LineWrapping::Scroll => div().flex_1().min_w_0().overflow_hidden().child(
            div()
                .ml(px(-style.horizontal_offset))
                .whitespace_nowrap()
                .child(text),
        ),
    };
    cell.relative()
        .cursor(CursorStyle::IBeam)
        .child(registration)
}

/// The row background for a line of `kind`.
fn row_background(kind: DiffLineKind) -> Option<Rgba> {
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

/// A diff row in the monospaced diff font with a line's background; `None`
/// is the empty side of a split row. Empty rows keep the row height.
pub fn render_row(kind: Option<DiffLineKind>) -> Div {
    let row = div()
        .flex()
        .min_h(px(DIFF_ROW_HEIGHT))
        .font_family(theme::MONO_FONT)
        .text_xs()
        .line_height(px(DIFF_ROW_HEIGHT));
    match kind.map(row_background) {
        Some(Some(background)) => row.bg(background),
        Some(None) => row,
        None => row.bg(theme::diff_filler()),
    }
}

/// The `@@ … @@` row that starts a hunk, inside the file's card.
pub fn render_hunk_header(header: &str, indent: f32) -> Div {
    frame_file_row(
        render_row(None)
            .bg(theme::hunk_background())
            .pl(px(indent))
            .text_color(theme::subtle())
            .whitespace_nowrap()
            .overflow_hidden()
            .child(NumberFlow::label(header.to_owned())),
    )
}
