use gpui::{AnyElement, Div, IntoElement, Rgba, SharedString, Styled, div, prelude::*, px};
use qd_diff::DiffLineKind;
use qd_studio::CodeAnchor;

use super::brief_excerpt::{ExcerptLine, brief_excerpt};
use super::brief_handlers::{BriefHandlers, on_anchor_click};
use super::brief_widget_context::BriefWidgetContext;
use crate::diff_view::DIFF_ROW_HEIGHT;
use crate::display_text::counted;
use crate::theme;

/// Width of an excerpt's line-number column.
const EXCERPT_GUTTER_WIDTH: f32 = 44.;

/// Quoted lines with their anchor as a link into the diff above them.
pub(super) fn render_code(
    widget: usize,
    part: &str,
    anchor: &CodeAnchor,
    context: &BriefWidgetContext,
) -> AnyElement {
    let lines = brief_excerpt(context.diffs, anchor);
    div()
        .flex()
        .flex_col()
        .rounded_md()
        .border_1()
        .border_color(theme::border())
        .overflow_hidden()
        .child(
            div()
                .px_2()
                .py_1()
                .bg(theme::panel())
                .border_b_1()
                .border_color(theme::border())
                .child(render_anchor_link(widget, part, anchor, context.handlers)),
        )
        .child(render_code_lines(widget, part, &lines))
        .into_any_element()
}

/// `path:12–20 (head)`, opening the diff at that line when clicked.
pub(super) fn render_anchor_link(
    widget: usize,
    part: &str,
    anchor: &CodeAnchor,
    handlers: &BriefHandlers,
) -> AnyElement {
    div()
        .id(SharedString::from(format!("brief-{widget}-{part}-anchor")))
        .cursor_pointer()
        .font_family(theme::MONO_FONT)
        .text_xs()
        .text_color(theme::accent())
        .hover(Styled::underline)
        .child(anchor.to_string())
        .on_click(on_anchor_click(&handlers.show_anchor, anchor))
        .into_any_element()
}

/// Quoted lines: number, mark and text, with the diff's added and removed
/// backgrounds, and a gap row where the range steps over lines the diff
/// does not show; long lines scroll sideways.
fn render_code_lines(widget: usize, part: &str, lines: &[ExcerptLine]) -> AnyElement {
    if lines.is_empty() {
        return div()
            .px_2()
            .py_1()
            .text_xs()
            .text_color(theme::muted())
            .child("These lines are not in the loaded diff.")
            .into_any_element();
    }
    // Each line, preceded by a gap row when numbers skip lines between hunks.
    let mut rows = Vec::with_capacity(lines.len());
    for (index, line) in lines.iter().enumerate() {
        let previous = index.checked_sub(1).map(|previous| lines[previous].number);
        if let Some(hidden) = previous.and_then(|previous| line.number.checked_sub(previous + 1))
            && hidden > 0
        {
            rows.push(render_gap_row(hidden));
        }
        rows.push(render_code_line(line));
    }
    div()
        .id(SharedString::from(format!("brief-{widget}-{part}-code")))
        .overflow_x_scroll()
        .bg(theme::code_background())
        .child(div().flex().flex_col().min_w_full().children(rows))
        .into_any_element()
}

/// One quoted line in the diff font.
fn render_code_line(line: &ExcerptLine) -> AnyElement {
    let (background, mark, mark_color) = line_look(line.kind);
    code_row()
        .when_some(background, Styled::bg)
        .child(
            div()
                .flex_none()
                .w(px(EXCERPT_GUTTER_WIDTH))
                .pr_2()
                .text_right()
                .text_color(theme::line_number())
                .child(line.number.to_string()),
        )
        .child(div().flex_none().w_4().text_color(mark_color).child(mark))
        .child(
            div()
                .flex_none()
                .pr_3()
                .whitespace_nowrap()
                .child(line.text.clone()),
        )
        .into_any_element()
}

/// The row standing for `hidden` lines between two hunks.
fn render_gap_row(hidden: u32) -> AnyElement {
    code_row()
        .bg(theme::hunk_background())
        .text_color(theme::subtle())
        .child(
            div()
                .flex_none()
                .w(px(EXCERPT_GUTTER_WIDTH))
                .pr_2()
                .text_right()
                .child("⋯"),
        )
        .child(format!(
            "{} not in the diff",
            counted(hidden, "line", "lines")
        ))
        .into_any_element()
}

/// A row in the diff font at the diff's row height.
fn code_row() -> Div {
    div()
        .flex()
        .min_h(px(DIFF_ROW_HEIGHT))
        .font_family(theme::MONO_FONT)
        .text_xs()
        .line_height(px(DIFF_ROW_HEIGHT))
}

/// A quoted line's background, mark and mark color by kind.
fn line_look(kind: DiffLineKind) -> (Option<Rgba>, &'static str, Rgba) {
    match kind {
        DiffLineKind::Added => (Some(theme::added_background()), "+", theme::added()),
        DiffLineKind::Removed => (Some(theme::removed_background()), "-", theme::removed()),
        DiffLineKind::Context => (None, " ", theme::muted()),
    }
}
