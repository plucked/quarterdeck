use std::sync::Arc;

use gpui::{AnyElement, FontWeight, IntoElement, div, prelude::*, px};
use qd_markdown::MarkdownDocument;
use qd_studio::CalloutTone;

use super::brief_text_rendering::{caption_line, markdown};
use super::brief_widget_context::BriefWidgetContext;
use super::code_excerpt_rendering::render_code;
use super::graph_rendering::render_graph;
use super::prepared_widget::PreparedWidget;
use super::scope_rendering::render_scope;
use super::sequence_rendering::render_sequence;
use super::table_rendering::render_table;
use super::tour_rendering::render_tour;
use crate::markdown_view::MarkdownView;
use crate::theme;

/// Space left and right of the brief's column, inside the pane.
pub const BRIEF_SIDE_PADDING: f32 = 24.;
/// Space above the first row of a brief.
const BRIEF_TOP_PADDING: f32 = 20.;
/// Space above every later row.
const BRIEF_ROW_GAP: f32 = 8.;
/// Width of the colored bar at a callout's left edge.
const CALLOUT_BAR_WIDTH: f32 = 3.;

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
