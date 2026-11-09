use std::sync::Arc;

use gpui::{
    AnyElement, Div, FontWeight, IntoElement, Rgba, SharedString, Styled, div, prelude::*, px,
};
use qd_diff::DiffLineKind;
use qd_markdown::MarkdownDocument;
use qd_studio::{
    AgentProgress, CalloutTone, CodeAccess, CodeAnchor, ReaderFamiliarity, RejectedWidget,
    TableRole,
};

use super::brief_excerpt::{ExcerptLine, brief_excerpt};
use super::brief_handlers::{BriefHandlers, on_click};
use super::brief_state::BriefState;
use super::brief_widget_context::BriefWidgetContext;
use super::diagram_rendering::{render_graph, render_sequence};
use super::familiarity_picker::render_familiarity_picker;
use super::prepared_brief::PreparedBrief;
use super::prepared_widget::PreparedWidget;
use super::scope_rendering::render_scope;
use super::tour_rendering::render_tour;
use crate::button::Button;
use crate::diff_view::DIFF_ROW_HEIGHT;
use crate::markdown_view::MarkdownView;
use crate::theme;

/// Space left and right of the brief's column, inside the pane.
pub const BRIEF_SIDE_PADDING: f32 = 24.;
/// Space above the first row of a brief.
const BRIEF_TOP_PADDING: f32 = 20.;
/// Space above every later row.
const BRIEF_ROW_GAP: f32 = 8.;
/// Width of an excerpt's line-number column.
const EXCERPT_GUTTER_WIDTH: f32 = 44.;
/// Width of the colored bar at a callout's left edge.
const CALLOUT_BAR_WIDTH: f32 = 3.;
/// Width of a trace's step-number column.
const TRACE_STEP_WIDTH: f32 = 28.;
/// Characters per thousand, for the writing progress.
const CHARACTERS_PER_THOUSAND: f64 = 1000.;

/// One row of a brief in its column, as wide as the pane less its side
/// padding (the reader sizes the pane), with the space above it (more above
/// the first row).
pub fn render_brief_column(is_first_row: bool, content: AnyElement) -> AnyElement {
    let top = if is_first_row {
        BRIEF_TOP_PADDING
    } else {
        BRIEF_ROW_GAP
    };
    div()
        .w_full()
        .px(px(BRIEF_SIDE_PADDING))
        .pt(px(top))
        .pb_2()
        .child(content)
        .into_any_element()
}

/// A bar over the brief's list (the page tabs), aligned with its column
/// and staying in view while the brief scrolls.
pub fn render_brief_bar(content: AnyElement) -> AnyElement {
    div()
        .flex_none()
        .w_full()
        .px(px(BRIEF_SIDE_PADDING))
        .pt_3()
        .bg(theme::background())
        .child(content)
        .into_any_element()
}

/// No brief yet (what it is, and a button), one being written (how far,
/// and Cancel) or a failed run (why, and Try again). `can_write` is false
/// while there are no files to send.
pub fn render_brief_status(
    state: Option<&BriefState>,
    can_write: bool,
    reader: ReaderFamiliarity,
    handlers: &BriefHandlers,
) -> AnyElement {
    let card = div()
        .flex()
        .flex_col()
        .gap_3()
        .p_4()
        .rounded_md()
        .bg(theme::surface())
        .border_1()
        .border_color(theme::border_strong());
    match state {
        None => card
            .child(section_label("Brief"))
            .child(div().text_sm().text_color(theme::muted()).child(
                "Claude Code reads this change's diffs and a shallow checkout of its head \
                 commit, and walks you through it: why it exists, the context you \
                 need, what changes, the order to read it in and how it works, with every \
                 claim tied to lines you can open in the diff. It can only read files in that checkout, which \
                 is deleted when it is done; it cannot run or change anything. The code \
                 it reads is sent to Anthropic.",
            ))
            .child(render_familiarity_picker("brief-reader", reader, handlers))
            .child(
                div().flex().child(
                    Button::new("write-brief", "Write brief")
                        .primary()
                        .disabled(!can_write)
                        .on_click(on_click(&handlers.write)),
                ),
            )
            .into_any_element(),
        Some(BriefState::Writing(progress)) => card
            .child(section_label("Writing the brief"))
            .child(
                div()
                    .text_sm()
                    .text_color(theme::muted())
                    .child(progress_text(*progress)),
            )
            .child(
                div().flex().child(
                    Button::new("cancel-brief", "Cancel")
                        .ghost()
                        .on_click(on_click(&handlers.cancel)),
                ),
            )
            .into_any_element(),
        Some(BriefState::Failed(message)) => card
            .child(section_label("No brief"))
            .child(
                div()
                    .text_sm()
                    .text_color(theme::error())
                    .child(message.clone()),
            )
            .child(render_familiarity_picker("brief-reader", reader, handlers))
            .child(
                div().flex().child(
                    Button::new("retry-brief", "Try again")
                        .disabled(!can_write)
                        .on_click(on_click(&handlers.write)),
                ),
            )
            .into_any_element(),
        Some(BriefState::Written(_)) => div().into_any_element(),
    }
}

/// Widget `widget` of a written brief.
pub fn render_brief_widget(
    widget: usize,
    prepared: &PreparedWidget,
    context: &BriefWidgetContext,
) -> AnyElement {
    match prepared {
        PreparedWidget::Section { heading, body } => div()
            .flex()
            .flex_col()
            .gap_2()
            .pt_3()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(heading.clone()),
            )
            .child(markdown(format!("brief-{widget}-body"), body))
            .into_any_element(),
        PreparedWidget::Callout { tone, text } => render_callout(widget, *tone, text),
        PreparedWidget::CodeExcerpt { caption, anchor } => div()
            .flex()
            .flex_col()
            .gap_1p5()
            .child(caption_line(caption))
            .child(render_code(widget, "excerpt", anchor, context))
            .into_any_element(),
        PreparedWidget::CodeCompare {
            caption,
            before,
            after,
        } => div()
            .flex()
            .flex_col()
            .gap_1p5()
            .child(caption_line(caption))
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(render_code(widget, "before", before, context)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(render_code(widget, "after", after, context)),
                    ),
            )
            .into_any_element(),
        PreparedWidget::Table {
            title,
            columns,
            rows,
            role,
        } => render_table(widget, title, columns, rows, *role),
        PreparedWidget::Scope {
            changes,
            unchanged,
            mechanical,
        } => render_scope(widget, changes, unchanged, mechanical),
        PreparedWidget::Tour { title, steps } => {
            render_tour(widget, title, steps, context.handlers)
        }
        PreparedWidget::Graph { graph, layout } => {
            render_graph(widget, graph, layout, context.handlers)
        }
        PreparedWidget::Sequence { sequence, layout } => {
            render_sequence(widget, sequence, layout, context.handlers)
        }
    }
}

/// The lead paragraph: a step above the body text, so it reads as the
/// opening paragraph rather than a heading.
pub fn render_lead(lead: &Arc<MarkdownDocument>) -> AnyElement {
    div()
        .text_base()
        .child(MarkdownView::new("brief-lead", lead.clone()))
        .into_any_element()
}

/// Who wrote the brief, for which reader and what it cost, and writing it
/// again for the reader now picked (`reader`).
pub fn render_brief_footer(
    brief: &PreparedBrief,
    reader: ReaderFamiliarity,
    handlers: &BriefHandlers,
) -> AnyElement {
    let cost = brief
        .record
        .cost_usd
        .map(|cost| format!(" · ${cost:.2}"))
        .unwrap_or_default();
    div()
        .flex()
        .flex_col()
        .gap_3()
        .pt_4()
        .border_t_1()
        .border_color(theme::border())
        .child(
            div()
                .text_xs()
                .text_color(theme::subtle())
                .child(written_from(&brief.record.code, brief.record.reader, &cost)),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_end()
                .justify_between()
                .gap_3()
                .child(render_familiarity_picker(
                    "brief-rewrite-reader",
                    reader,
                    handlers,
                ))
                .child(
                    Button::new("rewrite-brief", "Write again")
                        .ghost()
                        .on_click(on_click(&handlers.write)),
                ),
        )
        .into_any_element()
}

/// What the footer says the brief was written from, for whom, and what it cost.
fn written_from(code: &CodeAccess, reader: ReaderFamiliarity, cost: &str) -> String {
    let for_whom = format!("for a reader who {}", reader.reader_description());
    match code {
        CodeAccess::Workspace => format!(
            "Written by Claude Code {for_whom}, from the diffs and a checkout of the head commit{cost}. It read code but ran nothing; check claims against the code."
        ),
        CodeAccess::DiffOnly(reason) => format!(
            "Written by Claude Code {for_whom}, from the diffs alone ({reason}){cost}. It ran nothing; check claims against the code."
        ),
    }
}

/// Quoted lines with their anchor as a link into the diff above them.
fn render_code(
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
    let target = anchor.clone();
    let show_anchor = handlers.show_anchor.clone();
    div()
        .id(SharedString::from(format!("brief-{widget}-{part}-anchor")))
        .cursor_pointer()
        .font_family(theme::MONO_FONT)
        .text_xs()
        .text_color(theme::accent())
        .hover(Styled::underline)
        .child(anchor.to_string())
        .on_click(move |_, window, cx| show_anchor(&target, window, cx))
        .into_any_element()
}

/// What the status row says while the agent works.
fn progress_text(progress: Option<AgentProgress>) -> String {
    match progress {
        None => "Starting Claude Code…".to_owned(),
        Some(AgentProgress::FetchingCode { percent: None }) => {
            "Checking out the head commit…".to_owned()
        }
        Some(AgentProgress::FetchingCode {
            percent: Some(percent),
        }) => format!("Checking out the head commit… {percent}%"),
        Some(AgentProgress::UnpackingCode) => "Unpacking the code…".to_owned(),
        Some(AgentProgress::Started) => "Claude is reading the change…".to_owned(),
        Some(AgentProgress::Thinking) => "Claude is working through the change…".to_owned(),
        Some(AgentProgress::Exploring { tool_calls }) => {
            let noun = if tool_calls == 1 { "look" } else { "looks" };
            format!("Claude is reading the code… {tool_calls} {noun} so far")
        }
        Some(AgentProgress::Writing { characters }) => {
            let thousands =
                f64::from(u32::try_from(characters).unwrap_or(u32::MAX)) / CHARACTERS_PER_THOUSAND;
            format!("Writing the brief… {thousands:.1}k characters")
        }
    }
}

/// Markdown prose of the brief, selectable, links opening only on the web.
pub(super) fn markdown(id: String, document: &Arc<MarkdownDocument>) -> AnyElement {
    div()
        .text_sm()
        .child(MarkdownView::new(SharedString::from(id), document.clone()))
        .into_any_element()
}

/// A small uppercase label heading a card.
pub(super) fn section_label(label: &'static str) -> Div {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(theme::subtle())
        .child(label.to_uppercase())
}

/// The caption above code, saying what to see in it.
pub(super) fn caption_line(caption: &str) -> Div {
    div()
        .text_sm()
        .text_color(theme::muted())
        .child(caption.to_owned())
}

/// A callout: a colored bar by tone, its label and text.
fn render_callout(widget: usize, tone: CalloutTone, text: &Arc<MarkdownDocument>) -> AnyElement {
    let color = match tone {
        CalloutTone::Note => theme::border_strong(),
        CalloutTone::Constraint => theme::removed(),
        CalloutTone::Verdict => theme::accent(),
        CalloutTone::Evidence => theme::success(),
    };
    div()
        .flex()
        .gap_3()
        .child(
            div()
                .flex_none()
                .w(px(CALLOUT_BAR_WIDTH))
                .rounded_sm()
                .bg(color),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .py_1()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(color)
                        .child(tone.label().to_uppercase()),
                )
                .child(markdown(format!("brief-{widget}-callout"), text)),
        )
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
    let noun = if hidden == 1 { "line" } else { "lines" };
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
        .child(format!("{hidden} {noun} not in the diff"))
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

/// A table of Markdown cells; a trace numbers its rows.
fn render_table(
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

/// The widgets left out, each with why, so a thin brief is explained.
pub fn render_rejected(rejected: &[RejectedWidget]) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .pt_3()
        .text_xs()
        .text_color(theme::subtle())
        .child(format!(
            "Left out: {} part{} of the brief pointed at lines outside the diff or was malformed.",
            rejected.len(),
            if rejected.len() == 1 { "" } else { "s" }
        ))
        .children(
            rejected
                .iter()
                .map(|rejected| div().child(format!("{}: {}", rejected.widget, rejected.reason))),
        )
        .into_any_element()
}
