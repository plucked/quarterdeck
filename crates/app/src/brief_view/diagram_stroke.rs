use gpui::{Bounds, PathBuilder, Pixels, Rgba, Window, point, px};

use super::diagram_geometry::{CubicCurve, DiagramPoint};

/// Width of an arrow's line.
const STROKE_WIDTH: f32 = 1.5;
/// Length of an arrowhead along the line.
const HEAD_LENGTH: f32 = 8.;
/// Half the width of an arrowhead's base.
const HEAD_HALF_WIDTH: f32 = 4.5;

/// What a line's path is.
#[derive(Debug, Clone)]
pub enum StrokeCourse {
    Curve(CubicCurve),
    /// Straight segments through the points, in order.
    Polyline(Vec<DiagramPoint>),
}

/// The tip at a line's end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowHead {
    /// None: a plain line, such as a lifeline.
    Without,
    Filled,
    /// Two strokes, for messages that do not wait or answer.
    Open,
}

/// One line of a diagram as it is painted: course, color, dashes (dash
/// and gap length) and tip. Built while rendering (colors come from the
/// active theme) and painted from a canvas.
#[derive(Debug, Clone)]
pub struct DiagramStroke {
    pub course: StrokeCourse,
    pub color: Rgba,
    pub dashes: Option<(f32, f32)>,
    pub head: ArrowHead,
}

/// Paints `strokes` in `bounds`, whose top left corner is the diagram's origin.
pub fn paint_strokes(bounds: Bounds<Pixels>, strokes: &[DiagramStroke], window: &mut Window) {
    let at = |diagram_point: DiagramPoint| {
        point(
            bounds.origin.x + px(diagram_point.x),
            bounds.origin.y + px(diagram_point.y),
        )
    };
    for stroke in strokes {
        // The line itself, dashed when asked.
        let mut line = PathBuilder::stroke(px(STROKE_WIDTH));
        if let Some((dash, gap)) = stroke.dashes {
            line = line.dash_array(&[px(dash), px(gap)]);
        }
        let (tip, toward_tip) = match &stroke.course {
            StrokeCourse::Curve(curve) => {
                line.move_to(at(curve.start));
                line.cubic_bezier_to(
                    at(curve.end),
                    at(curve.control_start),
                    at(curve.control_end),
                );
                (curve.end, direction(curve.control_end, curve.end))
            }
            StrokeCourse::Polyline(points) => {
                let Some((first, rest)) = points.split_first() else {
                    continue;
                };
                line.move_to(at(*first));
                for next in rest {
                    line.line_to(at(*next));
                }
                let before_last = points
                    .len()
                    .checked_sub(2)
                    .map_or(*first, |index| points[index]);
                let last = *points.last().unwrap_or(first);
                (last, direction(before_last, last))
            }
        };
        if let Ok(path) = line.build() {
            window.paint_path(path, stroke.color);
        }

        // The tip, pointing along the line's last stretch.
        let Some((along_x, along_y)) = toward_tip else {
            continue;
        };
        let base = DiagramPoint {
            x: tip.x - along_x * HEAD_LENGTH,
            y: tip.y - along_y * HEAD_LENGTH,
        };
        let side = |sign: f32| {
            at(DiagramPoint {
                x: base.x - along_y * HEAD_HALF_WIDTH * sign,
                y: base.y + along_x * HEAD_HALF_WIDTH * sign,
            })
        };
        let head = match stroke.head {
            ArrowHead::Without => continue,
            ArrowHead::Filled => {
                let mut head = PathBuilder::fill();
                head.move_to(at(tip));
                head.line_to(side(1.));
                head.line_to(side(-1.));
                head.close();
                head
            }
            ArrowHead::Open => {
                let mut head = PathBuilder::stroke(px(STROKE_WIDTH));
                head.move_to(side(1.));
                head.line_to(at(tip));
                head.line_to(side(-1.));
                head
            }
        };
        if let Ok(path) = head.build() {
            window.paint_path(path, stroke.color);
        }
    }
}

/// The unit direction from `from` to `to`; `None` when they coincide.
fn direction(from: DiagramPoint, to: DiagramPoint) -> Option<(f32, f32)> {
    let (x, y) = (to.x - from.x, to.y - from.y);
    let length = x.hypot(y);
    (length > f32::EPSILON).then(|| (x / length, y / length))
}
