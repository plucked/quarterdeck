use gpui::{AnyElement, Div, IntoElement, div, prelude::*, px};
use qc_cache::LineWrapping;
use qc_diff::{DiffHunk, FileDiff};
use qc_syntax::DiffSyntax;

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

/// What every half row of one file's split view shares.
struct SplitLayout<'diff> {
    syntax: Option<&'diff DiffSyntax>,
    gutter: f32,
    wrapping: LineWrapping,
}

/// A diff as two columns: old lines on the left, new on the right, a
/// replaced line beside its replacement, in syntax colors when `syntax` is
/// known. Wrapping rows are laid out pair by pair so both halves share a
/// height; scrolling columns scroll on their own and stay aligned because
/// every row is one line high.
pub fn render_split(
    diff: &FileDiff,
    syntax: Option<&DiffSyntax>,
    wrapping: LineWrapping,
    index: usize,
) -> AnyElement {
    let layout = SplitLayout {
        syntax,
        gutter: gutter_width(diff.max_line_number()),
        wrapping,
    };
    match wrapping {
        LineWrapping::Wrap => {
            let mut rows = Vec::new();
            for (hunk_index, hunk) in diff.hunks().iter().enumerate() {
                rows.push(render_hunk_header(&hunk.header, layout.gutter));
                rows.extend(hunk.split_rows().into_iter().map(|row| {
                    div()
                        .flex()
                        .child(layout.render_half(hunk, hunk_index, row.old, Side::Old))
                        .child(render_divider())
                        .child(layout.render_half(hunk, hunk_index, row.new, Side::New))
                }));
            }
            render_body(rows, wrapping, ("split-diff", index).into())
        }
        LineWrapping::Scroll => div()
            .flex()
            .child(layout.render_column(diff, Side::Old, index))
            .child(render_divider())
            .child(layout.render_column(diff, Side::New, index))
            .into_any_element(),
    }
}

impl SplitLayout<'_> {
    /// One side of every row of `diff`, as its own horizontally scrolling column.
    fn render_column(&self, diff: &FileDiff, side: Side, index: usize) -> Div {
        let mut rows = Vec::new();
        for (hunk_index, hunk) in diff.hunks().iter().enumerate() {
            // The header shows on the left; the right keeps an empty row so both sides line up.
            let header = match side {
                Side::Old => hunk.header.as_str(),
                Side::New => "",
            };
            rows.push(render_hunk_header(header, self.gutter));
            rows.extend(hunk.split_rows().into_iter().map(|row| {
                let line_index = match side {
                    Side::Old => row.old,
                    Side::New => row.new,
                };
                self.render_half(hunk, hunk_index, line_index, side)
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

    /// One half of a split row: a gutter, the marker and the text of line
    /// `line_index` of the hunk, or an empty filler when this side has none.
    fn render_half(
        &self,
        hunk: &DiffHunk,
        hunk_index: usize,
        line_index: Option<usize>,
        side: Side,
    ) -> Div {
        let line = line_index.and_then(|line_index| hunk.lines.get(line_index));
        let half = match self.wrapping {
            LineWrapping::Wrap => render_row(line.map(|line| line.kind)).flex_1().min_w_0(),
            LineWrapping::Scroll => render_row(line.map(|line| line.kind)),
        };
        let (Some(line), Some(line_index)) = (line, line_index) else {
            return half;
        };
        let number = match side {
            Side::Old => line.old_number,
            Side::New => line.new_number,
        };
        let spans = self
            .syntax
            .map_or(&[][..], |syntax| syntax.spans(hunk_index, line_index));
        half.child(render_gutter(number, self.gutter))
            .child(render_marker(line.kind))
            .child(render_code(line, spans, self.wrapping))
    }
}

fn render_divider() -> impl IntoElement {
    div().flex_none().w(px(1.)).bg(theme::border())
}
