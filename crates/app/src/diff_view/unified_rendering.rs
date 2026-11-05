use gpui::{AnyElement, Div, prelude::*};
use qc_cache::LineWrapping;
use qc_diff::{DiffLine, FileDiff};

use super::diff_cell_rendering::{
    gutter_width, render_body, render_code, render_gutter, render_hunk_header, render_marker,
    render_row,
};

/// A diff as one column: old and new line numbers, the marker, the text.
/// `index` keeps each file's horizontal scroll apart.
pub fn render_unified(diff: &FileDiff, wrapping: LineWrapping, index: usize) -> AnyElement {
    let gutter = gutter_width(diff.max_line_number());
    let mut rows = Vec::new();
    for hunk in diff.hunks() {
        rows.push(render_hunk_header(&hunk.header, 2. * gutter));
        rows.extend(
            hunk.lines
                .iter()
                .map(|line| render_unified_line(line, gutter, wrapping)),
        );
    }
    render_body(rows, wrapping, ("unified-diff", index).into())
}

fn render_unified_line(line: &DiffLine, gutter: f32, wrapping: LineWrapping) -> Div {
    render_row(Some(line.kind))
        .child(render_gutter(line.old_number, gutter))
        .child(render_gutter(line.new_number, gutter))
        .child(render_marker(line.kind))
        .child(render_code(line, wrapping))
}
