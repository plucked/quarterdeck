use gpui::{AnyElement, FontWeight, IntoElement, Rgba, SharedString, Styled, div, prelude::*, px};
use qd_studio::{DiagramChange, EdgeStyle, GraphNode, NodeRole, StudioGraph};

use super::brief_handlers::BriefHandlers;
use super::brief_text_rendering::caption_line;
use super::diagram_geometry::DiagramRect;
use super::diagram_legend::{change_swatch, legend_item, line_sample, render_legend, role_legend};
use super::diagram_rendering::{
    DASHED, DOTTED, ROLE_STRIPE, anchor_click, placed, role_color, stroke_canvas,
};
use super::diagram_stroke::{ArrowHead, DiagramStroke, StrokeCourse};
use super::graph_layout::{
    EDGE_LABEL_PADDING, GROUP_LABEL_HEIGHT, GraphLayout, NODE_PADDING_X, NODE_PADDING_Y,
};
use super::text_estimate::{LABEL_TEXT, SMALL_TEXT};
use crate::section_label::SectionLabel;
use crate::theme;

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
                                .children(heading.map(SectionLabel::new))
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

/// An arrow style's color and dashes.
fn edge_look(style: EdgeStyle) -> (Rgba, Option<(f32, f32)>) {
    match style {
        EdgeStyle::Control => (theme::muted(), None),
        EdgeStyle::Data => (theme::accent(), None),
        EdgeStyle::Async => (theme::muted(), Some(DASHED)),
        EdgeStyle::Optional => (theme::subtle(), Some(DOTTED)),
    }
}

/// A short line drawn in an arrow style.
fn edge_sample(style: EdgeStyle) -> AnyElement {
    let (color, dashes) = edge_look(style);
    line_sample(color, dashes, ArrowHead::Filled)
}
