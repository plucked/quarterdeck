use gpui::{
    AnyElement, Div, FontWeight, IntoElement, Rgba, SharedString, Styled, canvas, div, prelude::*,
    px,
};
use qd_studio::{
    CodeAnchor, DiagramChange, EdgeStyle, GraphNode, MessageStyle, NodeRole, SequenceMessage,
    StudioGraph, StudioSequence,
};

use super::brief_handlers::BriefHandlers;
use super::brief_rendering::{caption_line, section_label};
use super::diagram_geometry::{DiagramPoint, DiagramRect};
use super::diagram_stroke::{ArrowHead, DiagramStroke, StrokeCourse, paint_strokes};
use super::graph_layout::{
    EDGE_LABEL_PADDING, GROUP_LABEL_HEIGHT, GraphLayout, NODE_PADDING_X, NODE_PADDING_Y,
};
use super::sequence_layout::{
    HEADER_PADDING, MessagePlace, SELF_LOOP_HEIGHT, SELF_LOOP_WIDTH, SequenceLayout,
};
use super::text_estimate::{LABEL_TEXT, SMALL_TEXT};
use crate::theme;

/// Width of the role stripe at a box's left edge (a lane header's top).
const ROLE_STRIPE: f32 = 3.;
/// Dash and gap of async arrows and replies.
const DASHED: (f32, f32) = (5., 4.);
/// Dash and gap of optional arrows and lifelines.
const DOTTED: (f32, f32) = (2., 3.);
/// Size of a legend's line sample.
const LEGEND_SAMPLE_WIDTH: f32 = 26.;
const LEGEND_SAMPLE_HEIGHT: f32 = 10.;
/// Size of a legend's role swatch.
const LEGEND_SWATCH: f32 = 10.;

/// Which parts of a graph a picture shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GraphPicture {
    /// A graph without changes: everything.
    Whole,
    /// What was there before the change; removed parts marked.
    Before,
    /// What is there after the change; added parts marked.
    After,
}

impl GraphPicture {
    fn shows(self, change: DiagramChange) -> bool {
        match self {
            Self::Whole => true,
            Self::Before => change.is_before(),
            Self::After => change.is_after(),
        }
    }

    /// The color marking a part this picture highlights; `None` for a part
    /// it shows plainly.
    fn mark(self, change: DiagramChange) -> Option<Rgba> {
        match (self, change) {
            (Self::Before, DiagramChange::Removed) => Some(theme::removed()),
            (Self::After, DiagramChange::Added) => Some(theme::added()),
            _ => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Whole => "whole",
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

/// A graph: its title, the picture (before and after, aligned one above
/// the other, when the change added or removed parts) in a block that
/// scrolls sideways when wider than the column, and a legend of the roles
/// and arrow styles it uses.
pub fn render_graph(
    widget: usize,
    graph: &StudioGraph,
    layout: &GraphLayout,
    handlers: &BriefHandlers,
) -> AnyElement {
    let pictures = if graph.has_changes() {
        vec![GraphPicture::Before, GraphPicture::After]
    } else {
        vec![GraphPicture::Whole]
    };
    let roles: Vec<NodeRole> = graph.nodes.iter().map(|node| node.role).collect();
    let mut legend = role_legend(&roles);
    legend.extend(
        EdgeStyle::ALL
            .into_iter()
            .filter(|style| graph.edges.iter().any(|edge| edge.style == *style))
            .map(|style| legend_item(edge_sample(style), style.label())),
    );
    if graph.has_changes() {
        legend.push(legend_item(change_swatch(theme::removed()), "Removed"));
        legend.push(legend_item(change_swatch(theme::added()), "Added"));
    }
    div()
        .flex()
        .flex_col()
        .gap_1p5()
        .child(caption_line(&graph.title).font_weight(FontWeight::SEMIBOLD))
        .child(
            div()
                .id(SharedString::from(format!("brief-{widget}-graph")))
                .overflow_x_scroll()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .children(pictures.into_iter().map(|picture| {
                            let heading = match picture {
                                GraphPicture::Whole => None,
                                GraphPicture::Before => Some("Before"),
                                GraphPicture::After => Some("After"),
                            };
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .children(heading.map(picture_heading))
                                .child(render_graph_picture(
                                    widget, graph, layout, picture, handlers,
                                ))
                        })),
                ),
        )
        .child(render_legend(legend))
        .into_any_element()
}

/// One picture of a graph: boundaries, then arrows, then boxes, then the
/// arrows' labels on top.
fn render_graph_picture(
    widget: usize,
    graph: &StudioGraph,
    layout: &GraphLayout,
    picture: GraphPicture,
    handlers: &BriefHandlers,
) -> AnyElement {
    // Arrows shown in this picture, styled now while the theme is known.
    let strokes: Vec<DiagramStroke> = graph
        .edges
        .iter()
        .zip(&layout.edges)
        .filter(|(edge, _)| picture.shows(edge.change))
        .map(|(edge, route)| {
            let (color, dashes) = edge_look(edge.style);
            DiagramStroke {
                course: StrokeCourse::Curve(route.curve),
                color: picture.mark(edge.change).unwrap_or(color),
                dashes,
                head: ArrowHead::Filled,
            }
        })
        .collect();
    let groups = graph
        .groups
        .iter()
        .zip(&layout.groups)
        .filter_map(|(label, rect)| rect.map(|rect| render_group_border(label, rect)));
    let nodes = graph
        .nodes
        .iter()
        .zip(&layout.nodes)
        .enumerate()
        .filter(|(_, (node, _))| picture.shows(node.change))
        .map(|(index, (node, rect))| {
            let id = format!("brief-{widget}-{}-node-{index}", picture.id());
            render_graph_node(id, node, *rect, picture.mark(node.change), handlers)
        });
    let labels = graph
        .edges
        .iter()
        .zip(&layout.edges)
        .filter(|(edge, _)| picture.shows(edge.change))
        .map(|(edge, route)| {
            render_edge_label(&edge.label, route.label, picture.mark(edge.change))
        });
    div()
        .relative()
        .flex_none()
        .w(px(layout.width))
        .h(px(layout.height))
        .children(groups)
        .child(stroke_canvas(strokes))
        .children(nodes)
        .children(labels)
        .into_any_element()
}

/// A boundary's border with its name at the top.
fn render_group_border(label: &str, rect: DiagramRect) -> AnyElement {
    placed(rect)
        .rounded_md()
        .border_1()
        .border_color(theme::border_strong())
        .child(
            div()
                .h(px(GROUP_LABEL_HEIGHT))
                .px_2()
                .flex()
                .items_center()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme::subtle())
                .child(label.to_owned()),
        )
        .into_any_element()
}

/// A box: role stripe, name and detail, bordered in `mark` (a change's
/// color) when given; a box with lines opens them when clicked.
fn render_graph_node(
    id: String,
    node: &GraphNode,
    rect: DiagramRect,
    mark: Option<Rgba>,
    handlers: &BriefHandlers,
) -> AnyElement {
    let text = div()
        .flex()
        .flex_col()
        .px(px(NODE_PADDING_X))
        .py(px(NODE_PADDING_Y))
        .child(
            div()
                .text_sm()
                .line_height(px(LABEL_TEXT.line_height))
                .font_weight(FontWeight::SEMIBOLD)
                .child(node.label.clone()),
        )
        .when(!node.detail.is_empty(), |text| {
            text.child(
                div()
                    .text_xs()
                    .line_height(px(SMALL_TEXT.line_height))
                    .text_color(theme::muted())
                    .child(node.detail.clone()),
            )
        });
    let box_element = placed(rect)
        .id(SharedString::from(id))
        .overflow_hidden()
        .rounded_md()
        .bg(theme::surface())
        .border_color(mark.unwrap_or_else(theme::border_strong))
        .when(mark.is_some(), Styled::border_2)
        .when(mark.is_none(), Styled::border_1)
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(ROLE_STRIPE))
                .bg(role_color(node.role)),
        )
        .child(text);
    match &node.anchor {
        Some(anchor) => box_element
            .cursor_pointer()
            .hover(|style| style.border_color(theme::accent()))
            .on_click(anchor_click(anchor, handlers))
            .into_any_element(),
        None => box_element.into_any_element(),
    }
}

/// An arrow's label over the arrow, masking the line behind it.
fn render_edge_label(label: &str, rect: DiagramRect, mark: Option<Rgba>) -> AnyElement {
    div()
        .absolute()
        .left(px(rect.x))
        .top(px(rect.y))
        .w(px(rect.width))
        .px(px(EDGE_LABEL_PADDING))
        .rounded_sm()
        .bg(theme::background())
        .text_xs()
        .line_height(px(SMALL_TEXT.line_height))
        .text_center()
        .text_color(mark.unwrap_or_else(theme::muted))
        .child(label.to_owned())
        .into_any_element()
}

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

/// A canvas filling its diagram that paints `strokes` behind the boxes.
fn stroke_canvas(strokes: Vec<DiagramStroke>) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| paint_strokes(bounds, &strokes, window),
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// A div placed at `rect` inside its diagram.
fn placed(rect: DiagramRect) -> Div {
    div()
        .absolute()
        .left(px(rect.x))
        .top(px(rect.y))
        .w(px(rect.width))
        .h(px(rect.height))
}

/// A click handler opening `anchor`'s lines.
fn anchor_click(
    anchor: &CodeAnchor,
    handlers: &BriefHandlers,
) -> impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static {
    let target = anchor.clone();
    let show_anchor = handlers.show_anchor.clone();
    move |_, window, cx| show_anchor(&target, window, cx)
}

/// "Before" or "After" over a picture.
fn picture_heading(heading: &'static str) -> Div {
    section_label(heading)
}

/// The color standing for a role, as a second cue next to the label:
/// orange for a state owner, blue for a transport, teal for stored state,
/// purple for an outside system, olive for a constraint, green for an actor.
fn role_color(role: NodeRole) -> Rgba {
    let colors = theme::label_colors();
    match role {
        NodeRole::Transport => colors[0],
        NodeRole::External => colors[1],
        NodeRole::Store => colors[2],
        NodeRole::Owner => colors[3],
        NodeRole::Actor => colors[5],
        NodeRole::Constraint => colors[6],
    }
}

/// An arrow style's color and dashes.
fn edge_look(style: EdgeStyle) -> (Rgba, Option<(f32, f32)>) {
    match style {
        EdgeStyle::Control => (theme::muted(), None),
        EdgeStyle::Data => (theme::accent(), None),
        EdgeStyle::Async => (theme::muted(), Some(DASHED)),
        EdgeStyle::Optional => (theme::subtle(), Some(DOTTED)),
    }
}

/// A message style's color, dashes and tip.
fn message_look(style: MessageStyle) -> (Rgba, Option<(f32, f32)>, ArrowHead) {
    match style {
        MessageStyle::Call => (theme::muted(), None, ArrowHead::Filled),
        MessageStyle::Reply => (theme::subtle(), Some(DASHED), ArrowHead::Open),
        MessageStyle::Async => (theme::accent(), None, ArrowHead::Open),
    }
}

/// A legend entry per role used, in the roles' order.
fn role_legend(roles: &[NodeRole]) -> Vec<AnyElement> {
    NodeRole::ALL
        .into_iter()
        .filter(|role| roles.contains(role))
        .map(|role| {
            let swatch = div()
                .size(px(LEGEND_SWATCH))
                .rounded_sm()
                .bg(role_color(role))
                .into_any_element();
            legend_item(swatch, role.label())
        })
        .collect()
}

/// A short line drawn in an arrow style.
fn edge_sample(style: EdgeStyle) -> AnyElement {
    let (color, dashes) = edge_look(style);
    line_sample(color, dashes, ArrowHead::Filled)
}

/// A short line drawn in a message style.
fn message_sample(style: MessageStyle) -> AnyElement {
    let (color, dashes, head) = message_look(style);
    line_sample(color, dashes, head)
}

/// A legend's line sample.
fn line_sample(color: Rgba, dashes: Option<(f32, f32)>, head: ArrowHead) -> AnyElement {
    let middle = LEGEND_SAMPLE_HEIGHT / 2.;
    let strokes = vec![DiagramStroke {
        course: StrokeCourse::Polyline(vec![
            DiagramPoint { x: 1., y: middle },
            DiagramPoint {
                x: LEGEND_SAMPLE_WIDTH - 1.,
                y: middle,
            },
        ]),
        color,
        dashes,
        head,
    }];
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| paint_strokes(bounds, &strokes, window),
    )
    .w(px(LEGEND_SAMPLE_WIDTH))
    .h(px(LEGEND_SAMPLE_HEIGHT))
    .into_any_element()
}

/// A small box bordered in a change's color.
fn change_swatch(color: Rgba) -> AnyElement {
    div()
        .size(px(LEGEND_SWATCH))
        .rounded_sm()
        .border_2()
        .border_color(color)
        .into_any_element()
}

/// A sample and its name.
fn legend_item(sample: AnyElement, label: &'static str) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_1p5()
        .child(sample)
        .child(label)
        .into_any_element()
}

/// The legend under a diagram: its entries in a wrapping row.
fn render_legend(items: Vec<AnyElement>) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .gap_x_4()
        .gap_y_1()
        .text_xs()
        .text_color(theme::muted())
        .children(items)
        .into_any_element()
}
