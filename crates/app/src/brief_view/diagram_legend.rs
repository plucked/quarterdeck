use gpui::{AnyElement, IntoElement, Rgba, Styled, canvas, div, prelude::*, px};
use qd_studio::NodeRole;

use super::diagram_geometry::DiagramPoint;
use super::diagram_rendering::role_color;
use super::diagram_stroke::{ArrowHead, DiagramStroke, StrokeCourse, paint_strokes};
use crate::theme;

/// Size of a legend's line sample.
const LEGEND_SAMPLE_WIDTH: f32 = 26.;
const LEGEND_SAMPLE_HEIGHT: f32 = 10.;
/// Size of a legend's role swatch.
const LEGEND_SWATCH: f32 = 10.;

/// A legend entry per role used, in the roles' order.
pub(super) fn role_legend(roles: &[NodeRole]) -> Vec<AnyElement> {
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

/// A legend's line sample.
pub(super) fn line_sample(color: Rgba, dashes: Option<(f32, f32)>, head: ArrowHead) -> AnyElement {
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
pub(super) fn change_swatch(color: Rgba) -> AnyElement {
    div()
        .size(px(LEGEND_SWATCH))
        .rounded_sm()
        .border_2()
        .border_color(color)
        .into_any_element()
}

/// A sample and its name.
pub(super) fn legend_item(sample: AnyElement, label: &'static str) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap_1p5()
        .child(sample)
        .child(label)
        .into_any_element()
}

/// The legend under a diagram: its entries in a wrapping row.
pub(super) fn render_legend(items: Vec<AnyElement>) -> AnyElement {
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
