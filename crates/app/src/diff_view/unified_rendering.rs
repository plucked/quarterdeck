use gpui::{Div, prelude::*};
use qd_diff::DiffLine;
use qd_syntax::SyntaxSpan;

use super::diff_cell_rendering::{render_code, render_gutter, render_marker, render_row};
use super::file_card_rendering::frame_file_row;
use super::{CodeCells, DiffRowStyle, DiffSide};

/// One row of a unified diff: old and new line numbers, the marker, and the
/// selectable text in `syntax` colors.
pub fn render_unified_line(
    line: &DiffLine,
    syntax: &[SyntaxSpan],
    style: DiffRowStyle,
    cells: &CodeCells,
) -> Div {
    frame_file_row(
        render_row(Some(line.kind))
            .child(render_gutter(line.old_number, style.gutter_width))
            .child(render_gutter(line.new_number, style.gutter_width))
            .child(render_marker(line.kind))
            .child(render_code(line, syntax, style, cells, DiffSide::Unified)),
    )
}
