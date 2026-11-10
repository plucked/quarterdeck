/// The width diagrams are laid out for until the brief's column has been
/// measured (the first frame); then they are laid out for the column.
pub const DEFAULT_DIAGRAM_WIDTH: f32 = 832.;
/// Space around a whole diagram, graph or sequence.
pub const DIAGRAM_MARGIN: f32 = 6.;

/// A point of a diagram, in pixels from its top left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiagramPoint {
    pub x: f32,
    pub y: f32,
}

/// A rectangle of a diagram, in pixels from its top left corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiagramRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl DiagramRect {
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn center_x(&self) -> f32 {
        self.x + self.width / 2.
    }

    pub fn center_y(&self) -> f32 {
        self.y + self.height / 2.
    }

    /// The rectangle moved by `x` and `y`.
    pub fn moved(&self, x: f32, y: f32) -> Self {
        Self {
            x: self.x + x,
            y: self.y + y,
            ..*self
        }
    }

    /// Whether the two rectangles share any area.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }
}

/// A cubic Bézier curve: an arrow's path from `start` to `end`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CubicCurve {
    pub start: DiagramPoint,
    pub control_start: DiagramPoint,
    pub control_end: DiagramPoint,
    pub end: DiagramPoint,
}

impl CubicCurve {
    /// The point at `t` (0 at the start, 1 at the end).
    pub fn point_at(&self, t: f32) -> DiagramPoint {
        let rest = 1. - t;
        let weights = [
            rest * rest * rest,
            3. * rest * rest * t,
            3. * rest * t * t,
            t * t * t,
        ];
        let points = [self.start, self.control_start, self.control_end, self.end];
        DiagramPoint {
            x: weights
                .iter()
                .zip(&points)
                .map(|(weight, point)| weight * point.x)
                .sum(),
            y: weights
                .iter()
                .zip(&points)
                .map(|(weight, point)| weight * point.y)
                .sum(),
        }
    }

    /// The point where `coordinate` of the curve reaches `value`, for a
    /// curve along which that coordinate grows; found by halving the interval.
    pub fn point_where(
        &self,
        coordinate: impl Fn(DiagramPoint) -> f32,
        value: f32,
    ) -> DiagramPoint {
        let (mut low, mut high) = (0_f32, 1_f32);
        for _ in 0..24 {
            let middle = f32::midpoint(low, high);
            if coordinate(self.point_at(middle)) < value {
                low = middle;
            } else {
                high = middle;
            }
        }
        self.point_at(f32::midpoint(low, high))
    }

    /// A rectangle the curve stays within: the box around its control points.
    pub fn bounds(&self) -> DiagramRect {
        let points = [self.start, self.control_start, self.control_end, self.end];
        let (min_x, min_y, max_x, max_y) = points.iter().fold(
            (f32::MAX, f32::MAX, f32::MIN, f32::MIN),
            |(min_x, min_y, max_x, max_y), point| {
                (
                    min_x.min(point.x),
                    min_y.min(point.y),
                    max_x.max(point.x),
                    max_y.max(point.y),
                )
            },
        );
        DiagramRect {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        }
    }

    /// The curve moved by `x` and `y`.
    pub fn moved(&self, x: f32, y: f32) -> Self {
        let move_point = |point: DiagramPoint| DiagramPoint {
            x: point.x + x,
            y: point.y + y,
        };
        Self {
            start: move_point(self.start),
            control_start: move_point(self.control_start),
            control_end: move_point(self.control_end),
            end: move_point(self.end),
        }
    }
}
