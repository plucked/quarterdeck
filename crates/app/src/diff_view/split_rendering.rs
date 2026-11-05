use gpui::{AnyElement, Div, IntoElement, div, prelude::*, px};
use qc_cache::LineWrapping;
use qc_diff::{DiffHunk, DiffLine, FileDiff};

use super::diff_cell_rendering::{
    gutter_width, render_body, render_code, render_gutter, render_hunk_header, render_marker,
    render_row,
};
use crate::theme;

/// Which side of a split view a line is shown on.
#[derive(Clone, Copy)]
enum Side {
    Old,
    New,
}

/// A diff as two columns: old lines on the left, new on the right, a
/// replaced line beside its replacement. Wrapping rows are laid out pair by
/// pair so both halves share a height; scrolling columns scroll on their
/// own and stay aligned because every row is one line high.
pub fn render_split(diff: &FileDiff, wrapping: LineWrapping, index: usize) -> AnyElement {
    let gutter = gutter_width(diff.max_line_number());
    match wrapping {
        LineWrapping::Wrap => {
            let mut rows = Vec::new();
            for hunk in diff.hunks() {
                rows.push(render_hunk_header(&hunk.header, gutter));
                rows.extend(hunk.split_rows().into_iter().map(|row| {
                    div()
                        .flex()
                        .child(render_half(
                            line_at(hunk, row.old),
                            Side::Old,
                            gutter,
                            wrapping,
                        ))
                        .child(render_divider())
                        .child(render_half(
                            line_at(hunk, row.new),
                            Side::New,
                            gutter,
                            wrapping,
                        ))
                }));
            }
            render_body(rows, wrapping, ("split-diff", index).into())
        }
        LineWrapping::Scroll => div()
            .flex()
            .child(render_column(diff, Side::Old, gutter, index))
            .child(render_divider())
            .child(render_column(diff, Side::New, gutter, index))
            .into_any_element(),
    }
}

/// One side of every row of `diff`, as its own horizontally scrolling column.
fn render_column(diff: &FileDiff, side: Side, gutter: f32, index: usize) -> Div {
    let mut rows = Vec::new();
    for hunk in diff.hunks() {
        // The header shows on the left; the right keeps an empty row so both sides line up.
        let header = match side {
            Side::Old => hunk.header.as_str(),
            Side::New => "",
        };
        rows.push(render_hunk_header(header, gutter));
        rows.extend(hunk.split_rows().into_iter().map(|row| {
            let line_index = match side {
                Side::Old => row.old,
                Side::New => row.new,
            };
            render_half(
                line_at(hunk, line_index),
                side,
                gutter,
                LineWrapping::Scroll,
            )
        }));
    }
    let id = match side {
        Side::Old => ("split-old", index),
        Side::New => ("split-new", index),
    };
    div()
        .flex_1()
        .min_w_0()
        .child(render_body(rows, LineWrapping::Scroll, id.into()))
}

/// One half of a split row: a gutter, the marker and the text, or an empty
/// filler when this side has no line.
fn render_half(line: Option<&DiffLine>, side: Side, gutter: f32, wrapping: LineWrapping) -> Div {
    let half = match wrapping {
        LineWrapping::Wrap => render_row(line.map(|line| line.kind)).flex_1().min_w_0(),
        LineWrapping::Scroll => render_row(line.map(|line| line.kind)),
    };
    let Some(line) = line else {
        return half;
    };
    let number = match side {
        Side::Old => line.old_number,
        Side::New => line.new_number,
    };
    half.child(render_gutter(number, gutter))
        .child(render_marker(line.kind))
        .child(render_code(line, wrapping))
}

fn render_divider() -> impl IntoElement {
    div().flex_none().w(px(1.)).bg(theme::border())
}

fn line_at(hunk: &DiffHunk, index: Option<usize>) -> Option<&DiffLine> {
    index.and_then(|index| hunk.lines.get(index))
}
