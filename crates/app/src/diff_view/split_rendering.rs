use gpui::{Div, IntoElement, div, prelude::*, px};
use qc_diff::{DiffHunk, SplitRow};
use qc_syntax::DiffSyntax;

use super::DiffRowStyle;
use super::diff_cell_rendering::{render_code, render_gutter, render_marker, render_row};
use super::file_card_rendering::frame_file_row;
use crate::theme;

/// Which side of a split view a line is shown on.
#[derive(Clone, Copy)]
enum Side {
    Old,
    New,
}

/// One row of a side-by-side diff: the old line (left) beside the new line
/// (right) of hunk `hunk_index`, each half with its own gutter, in syntax
/// colors when known. Both halves share the row, so wrapped lines keep the
/// sides aligned, and scrolled lines shift together.
pub fn render_split_row(
    hunk: &DiffHunk,
    hunk_index: usize,
    row: SplitRow,
    syntax: Option<&DiffSyntax>,
    style: DiffRowStyle,
) -> Div {
    let half = |line_index: Option<usize>, side: Side| {
        render_half(hunk, hunk_index, line_index, side, syntax, style)
    };
    frame_file_row(
        div()
            .flex()
            .child(half(row.old, Side::Old))
            .child(render_divider())
            .child(half(row.new, Side::New)),
    )
}

/// One half of a split row: a gutter, the marker and the text of line
/// `line_index`, or an empty filler when this side has none.
fn render_half(
    hunk: &DiffHunk,
    hunk_index: usize,
    line_index: Option<usize>,
    side: Side,
    syntax: Option<&DiffSyntax>,
    style: DiffRowStyle,
) -> Div {
    let line = line_index.and_then(|line_index| hunk.lines.get(line_index));
    let half = render_row(line.map(|line| line.kind)).flex_1().min_w_0();
    let (Some(line), Some(line_index)) = (line, line_index) else {
        return half;
    };
    let number = match side {
        Side::Old => line.old_number,
        Side::New => line.new_number,
    };
    let spans = syntax.map_or(&[][..], |syntax| syntax.spans(hunk_index, line_index));
    half.child(render_gutter(number, style.gutter_width))
        .child(render_marker(line.kind))
        .child(render_code(line, spans, style))
}

fn render_divider() -> impl IntoElement {
    div().flex_none().w(px(1.)).bg(theme::border())
}
