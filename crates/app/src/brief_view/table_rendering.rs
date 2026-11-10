use std::sync::Arc;

use gpui::{AnyElement, FontWeight, IntoElement, div, prelude::*, px};
use qd_markdown::MarkdownDocument;
use qd_studio::TableRole;

use super::brief_text_rendering::{caption_line, markdown};
use crate::theme;

/// Width of a trace's step-number column.
const TRACE_STEP_WIDTH: f32 = 28.;

/// A table of Markdown cells; a trace numbers its rows.
pub(super) fn render_table(
    widget: usize,
    title: &str,
    columns: &[String],
    rows: &[Vec<Arc<MarkdownDocument>>],
    role: TableRole,
) -> AnyElement {
    let is_trace = role == TableRole::Trace;
    let step_cell = |content: AnyElement| {
        div()
            .flex_none()
            .w(px(TRACE_STEP_WIDTH))
            .text_xs()
            .text_color(theme::subtle())
            .child(content)
    };
    let header = div()
        .flex()
        .gap_3()
        .px_3()
        .py_1p5()
        .bg(theme::panel())
        .border_b_1()
        .border_color(theme::border())
        .when(is_trace, |row| row.child(step_cell("#".into_any_element())))
        .children(columns.iter().map(|column| {
            div()
                .flex_1()
                .min_w_0()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::muted())
                .child(column.clone())
        }));
    div()
        .flex()
        .flex_col()
        .gap_1p5()
        .child(caption_line(title).font_weight(FontWeight::SEMIBOLD))
        .child(
            div()
                .flex()
                .flex_col()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .overflow_hidden()
                .child(header)
                .children(rows.iter().enumerate().map(|(row_index, cells)| {
                    div()
                        .flex()
                        .gap_3()
                        .px_3()
                        .py_1p5()
                        .when(row_index > 0, |row| {
                            row.border_t_1().border_color(theme::border())
                        })
                        .when(is_trace, |row| {
                            row.child(step_cell((row_index + 1).to_string().into_any_element()))
                        })
                        .children(cells.iter().enumerate().map(|(column, cell)| {
                            div().flex_1().min_w_0().child(markdown(
                                format!("brief-{widget}-cell-{row_index}-{column}"),
                                cell,
                            ))
                        }))
                })),
        )
        .into_any_element()
}
