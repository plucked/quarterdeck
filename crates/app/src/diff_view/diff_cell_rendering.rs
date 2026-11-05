use gpui::{Div, Rgba, StyledText, div, prelude::*, px};
use qc_cache::LineWrapping;
use qc_diff::{DiffLine, DiffLineKind};
use qc_syntax::SyntaxSpan;

use super::DiffRowStyle;
use super::code_highlights::code_highlights;
use super::file_card_rendering::frame_file_row;
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

/// A line's text in its syntax colors, with its changed ranges on a
/// stronger background. Wraps within the row, or stays on one line clipped
/// to the row and shifted left by the file's horizontal offset.
pub fn render_code(line: &DiffLine, syntax: &[SyntaxSpan], style: DiffRowStyle) -> Div {
    let highlights = code_highlights(
        line.text.len(),
        syntax,
        &line.changed_ranges,
        emphasis_background(line.kind),
    );
    let text = StyledText::new(line.text.clone()).with_highlights(highlights);
    match style.wrapping {
        LineWrapping::Wrap => div().flex_1().min_w_0().pr_2().child(text),
        LineWrapping::Scroll => div().flex_1().min_w_0().overflow_hidden().child(
            div()
                .ml(px(-style.horizontal_offset))
                .whitespace_nowrap()
                .child(text),
        ),
    }
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
            .pl(px(indent))
            .text_color(theme::muted())
            .whitespace_nowrap()
            .overflow_hidden()
            .child(header.to_owned()),
    )
}
