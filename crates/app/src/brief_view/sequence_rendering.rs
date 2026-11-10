use gpui::{AnyElement, FontWeight, IntoElement, Rgba, SharedString, Styled, div, prelude::*, px};
use qd_studio::{MessageStyle, NodeRole, SequenceMessage, StudioSequence};

use super::brief_handlers::BriefHandlers;
use super::brief_text_rendering::caption_line;
use super::diagram_geometry::{DiagramPoint, DiagramRect};
use super::diagram_legend::{legend_item, line_sample, render_legend, role_legend};
use super::diagram_rendering::{
    DASHED, DOTTED, ROLE_STRIPE, anchor_click, placed, role_color, stroke_canvas,
};
use super::diagram_stroke::{ArrowHead, DiagramStroke, StrokeCourse};
use super::sequence_layout::{
    HEADER_PADDING, MessagePlace, SELF_LOOP_HEIGHT, SELF_LOOP_WIDTH, SequenceLayout,
};
use super::text_estimate::{LABEL_TEXT, SMALL_TEXT};
use crate::theme;

/// A sequence: its title, the lanes and messages in a block that scrolls
/// sideways when wider than the column, and a legend.
pub fn render_sequence(
    widget: usize,
    sequence: &StudioSequence,
    layout: &SequenceLayout,
    handlers: &BriefHandlers,
) -> AnyElement {
    let roles: Vec<NodeRole> = sequence.lanes.iter().map(|lane| lane.role).collect();
    let mut legend = role_legend(&roles);
    legend.extend(
        MessageStyle::ALL
            .into_iter()
            .filter(|style| {
                sequence
                    .messages
                    .iter()
                    .any(|message| message.style == *style)
            })
            .map(|style| legend_item(message_sample(style), style.label())),
    );
    div()
        .flex()
        .flex_col()
        .gap_1p5()
        .child(caption_line(&sequence.title).font_weight(FontWeight::SEMIBOLD))
        .child(
            div()
                .id(SharedString::from(format!("brief-{widget}-sequence")))
                .overflow_x_scroll()
                .child(render_sequence_picture(widget, sequence, layout, handlers)),
        )
        .child(render_legend(legend))
        .into_any_element()
}

/// Lane headers and dotted lifelines, then each message: arrow, label
/// above it (opening its lines when it has some), note under the label.
fn render_sequence_picture(
    widget: usize,
    sequence: &StudioSequence,
    layout: &SequenceLayout,
    handlers: &BriefHandlers,
) -> AnyElement {
    let headers = sequence
        .lanes
        .iter()
        .zip(&layout.headers)
        .map(|(lane, rect)| render_lane_header(&lane.label, lane.role, *rect));
    let texts = sequence
        .messages
        .iter()
        .zip(&layout.messages)
        .enumerate()
        .flat_map(|(index, (message, place))| {
            let id = format!("brief-{widget}-message-{index}");
            render_message_texts(id, message, place, handlers)
        });
    div()
        .relative()
        .flex_none()
        .w(px(layout.width))
        .h(px(layout.height))
        .child(stroke_canvas(sequence_strokes(sequence, layout)))
        .children(headers)
        .children(texts)
        .into_any_element()
}

/// The lifelines, then each message's arrow (a self message loops right
/// and back), styled now while the theme is known.
fn sequence_strokes(sequence: &StudioSequence, layout: &SequenceLayout) -> Vec<DiagramStroke> {
    let lifelines = layout.headers.iter().map(|header| DiagramStroke {
        course: StrokeCourse::Polyline(vec![
            DiagramPoint {
                x: header.center_x(),
                y: header.bottom(),
            },
            DiagramPoint {
                x: header.center_x(),
                y: layout.lifeline_bottom,
            },
        ]),
        color: theme::border_strong(),
        dashes: Some(DOTTED),
        head: ArrowHead::Without,
    });
    let arrows = sequence
        .messages
        .iter()
        .zip(&layout.messages)
        .map(|(message, place)| {
            let (color, dashes, head) = message_look(message.style);
            let (x, y) = (place.from_x, place.arrow_y);
            let points = if message.from == message.to {
                vec![
                    DiagramPoint { x, y },
                    DiagramPoint {
                        x: x + SELF_LOOP_WIDTH,
                        y,
                    },
                    DiagramPoint {
                        x: x + SELF_LOOP_WIDTH,
                        y: y + SELF_LOOP_HEIGHT,
                    },
                    DiagramPoint {
                        x,
                        y: y + SELF_LOOP_HEIGHT,
                    },
                ]
            } else {
                vec![DiagramPoint { x, y }, DiagramPoint { x: place.to_x, y }]
            };
            DiagramStroke {
                course: StrokeCourse::Polyline(points),
                color,
                dashes,
                head,
            }
        });
    lifelines.chain(arrows).collect()
}

/// A lane's header: a role stripe on top, the name centred.
fn render_lane_header(label: &str, role: NodeRole, rect: DiagramRect) -> AnyElement {
    placed(rect)
        .overflow_hidden()
        .rounded_md()
        .bg(theme::surface())
        .border_1()
        .border_color(theme::border_strong())
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .top_0()
                .h(px(ROLE_STRIPE))
                .bg(role_color(role)),
        )
        .child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .p(px(HEADER_PADDING))
                .text_sm()
                .line_height(px(LABEL_TEXT.line_height))
                .font_weight(FontWeight::SEMIBOLD)
                .text_center()
                .child(label.to_owned()),
        )
        .into_any_element()
}

/// A message's label (a link to its lines when it has some) and its note;
/// centred between the lanes, or beside the lifeline for a self message.
fn render_message_texts(
    id: String,
    message: &SequenceMessage,
    place: &MessagePlace,
    handlers: &BriefHandlers,
) -> impl Iterator<Item = AnyElement> {
    let is_between_lanes = message.from != message.to;
    let label = div()
        .id(SharedString::from(id))
        .absolute()
        .left(px(place.label.x))
        .top(px(place.label.y))
        .w(px(place.label.width))
        .text_xs()
        .line_height(px(SMALL_TEXT.line_height))
        .when(is_between_lanes, Styled::text_center)
        .child(message.label.clone());
    let label = match &message.anchor {
        Some(anchor) => label
            .cursor_pointer()
            .text_color(theme::accent())
            .hover(Styled::underline)
            .on_click(anchor_click(anchor, handlers))
            .into_any_element(),
        None => label.into_any_element(),
    };
    let note = message.note.as_ref().zip(place.note).map(|(note, rect)| {
        div()
            .absolute()
            .left(px(rect.x))
            .top(px(rect.y))
            .w(px(rect.width))
            .text_xs()
            .italic()
            .line_height(px(SMALL_TEXT.line_height))
            .text_color(theme::subtle())
            .when(is_between_lanes, Styled::text_center)
            .child(note.clone())
            .into_any_element()
    });
    std::iter::once(label).chain(note)
}

/// A message style's color, dashes and tip.
fn message_look(style: MessageStyle) -> (Rgba, Option<(f32, f32)>, ArrowHead) {
    match style {
        MessageStyle::Call => (theme::muted(), None, ArrowHead::Filled),
        MessageStyle::Reply => (theme::subtle(), Some(DASHED), ArrowHead::Open),
        MessageStyle::Async => (theme::accent(), None, ArrowHead::Open),
    }
}

/// A short line drawn in a message style.
fn message_sample(style: MessageStyle) -> AnyElement {
    let (color, dashes, head) = message_look(style);
    line_sample(color, dashes, head)
}
