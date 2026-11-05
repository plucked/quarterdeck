use gpui::{AnyElement, Div, FontWeight, IntoElement, div, prelude::*, px};
use qc_markdown::{
    ColumnAlignment, HeadingLevel, InlineText, ListItem, MarkdownBlock, MarkdownList,
    MarkdownTable, TaskState,
};

use super::selectable_texts::SelectableTexts;
use crate::theme;

/// Width of the bullet or number column of a list item.
const LIST_MARKER_WIDTH: f32 = 20.;
const BULLET: &str = "•";
const OPEN_TASK: &str = "☐";
const DONE_TASK: &str = "☑";

/// Renders `blocks` in order; `texts` places and highlights their text.
pub fn render_blocks(blocks: &[MarkdownBlock], texts: &mut SelectableTexts) -> Vec<AnyElement> {
    blocks
        .iter()
        .map(|block| render_block(block, texts))
        .collect()
}

fn render_block(block: &MarkdownBlock, texts: &mut SelectableTexts) -> AnyElement {
    match block {
        MarkdownBlock::Paragraph(text) => texts.render_inline(text),
        MarkdownBlock::Heading { level, text } => render_heading(*level, text, texts),
        MarkdownBlock::CodeBlock { code, .. } => render_code_block(code, texts),
        MarkdownBlock::BlockQuote(blocks) => div()
            .flex()
            .flex_col()
            .gap_2()
            .pl_3()
            .border_l_2()
            .border_color(theme::border())
            .text_color(theme::muted())
            .children(render_blocks(blocks, texts))
            .into_any_element(),
        MarkdownBlock::List(list) => render_list(list, texts),
        MarkdownBlock::Table(table) => render_table(table, texts),
        MarkdownBlock::Rule => div().h(px(1.)).bg(theme::border()).into_any_element(),
    }
}

/// A bold heading sized by level; the top two levels get a rule below, as on GitHub.
fn render_heading(
    level: HeadingLevel,
    text: &InlineText,
    texts: &mut SelectableTexts,
) -> AnyElement {
    let heading = div()
        .font_weight(FontWeight::BOLD)
        .child(texts.render_inline(text));
    match level {
        HeadingLevel::One => heading
            .text_2xl()
            .pb_1()
            .border_b_1()
            .border_color(theme::border()),
        HeadingLevel::Two => heading
            .text_xl()
            .pb_1()
            .border_b_1()
            .border_color(theme::border()),
        HeadingLevel::Three => heading.text_lg(),
        HeadingLevel::Four | HeadingLevel::Five | HeadingLevel::Six => heading.text_base(),
    }
    .into_any_element()
}

/// Monospaced code on a code background, one selectable text; long lines
/// scroll sideways instead of wrapping.
fn render_code_block(code: &str, texts: &mut SelectableTexts) -> AnyElement {
    div()
        .id(("code-block", texts.next_element_id()))
        .overflow_x_scroll()
        .p_3()
        .rounded_md()
        .bg(theme::code_background())
        .font_family(theme::MONO_FONT)
        .text_xs()
        .whitespace_nowrap()
        .child(texts.render_plain(code))
        .into_any_element()
}

/// Items with a bullet, number or checkbox beside their blocks.
fn render_list(list: &MarkdownList, texts: &mut SelectableTexts) -> AnyElement {
    let mut items = Vec::with_capacity(list.items.len());
    for (offset, item) in (0_u64..).zip(&list.items) {
        let marker = list_marker(list.start.map(|start| start + offset), item);
        items.push(
            div()
                .flex()
                .gap_1()
                .child(
                    div()
                        .flex_none()
                        .w(px(LIST_MARKER_WIDTH))
                        .text_color(theme::muted())
                        .child(marker),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(render_blocks(&item.blocks, texts)),
                ),
        );
    }
    div()
        .flex()
        .flex_col()
        .gap_1()
        .children(items)
        .into_any_element()
}

/// A task item's checkbox, else the item's number, else a bullet.
fn list_marker(number: Option<u64>, item: &ListItem) -> String {
    match (item.task, number) {
        (Some(TaskState::Open), _) => OPEN_TASK.to_owned(),
        (Some(TaskState::Done), _) => DONE_TASK.to_owned(),
        (None, Some(number)) => format!("{number}."),
        (None, None) => BULLET.to_owned(),
    }
}

/// A bordered grid: a bold header row, then the body rows. Missing cells
/// render empty so columns stay aligned.
fn render_table(table: &MarkdownTable, texts: &mut SelectableTexts) -> AnyElement {
    let column_count = table
        .rows
        .iter()
        .map(Vec::len)
        .chain([table.header.len(), table.alignments.len()])
        .max()
        .unwrap_or_default();

    let header = render_table_row(&table.header, &table.alignments, column_count, texts)
        .bg(theme::panel())
        .font_weight(FontWeight::SEMIBOLD);
    let rows: Vec<Div> = table
        .rows
        .iter()
        .map(|row| {
            render_table_row(row, &table.alignments, column_count, texts)
                .border_t_1()
                .border_color(theme::border())
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(theme::border())
        .rounded_md()
        .overflow_hidden()
        .child(header)
        .children(rows)
        .into_any_element()
}

/// One table row of `column_count` equal-width cells.
fn render_table_row(
    cells: &[InlineText],
    alignments: &[ColumnAlignment],
    column_count: usize,
    texts: &mut SelectableTexts,
) -> Div {
    let mut row = div().flex();
    for column in 0..column_count {
        let alignment = alignments
            .get(column)
            .copied()
            .unwrap_or(ColumnAlignment::Unspecified);
        let cell = div().flex_1().min_w_0().px_2().py_1();
        let cell = match alignment {
            ColumnAlignment::Center => cell.text_center(),
            ColumnAlignment::Right => cell.text_right(),
            ColumnAlignment::Left | ColumnAlignment::Unspecified => cell,
        };
        row = row.child(match cells.get(column) {
            Some(text) => cell.child(texts.render_inline(text)),
            None => cell,
        });
    }
    row
}
