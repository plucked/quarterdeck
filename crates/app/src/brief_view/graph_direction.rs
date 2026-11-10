use super::diagram_geometry::{DiagramPoint, DiagramRect};
use super::graph_layout::{GROUP_LABEL_HEIGHT, GROUP_PADDING};

/// Which way a graph's arrows run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphDirection {
    /// Layers are columns; boundaries are horizontal bands.
    LeftToRight,
    /// Layers are rows; boundaries are vertical bands.
    TopToBottom,
}

impl GraphDirection {
    /// Space between two layers, where arrows and their labels go: labels
    /// sit beside each other in a column gap, above each other in a row gap.
    pub(super) fn layer_gap(self) -> f32 {
        match self {
            Self::LeftToRight => 132.,
            Self::TopToBottom => 72.,
        }
    }

    /// Space between two boxes of one layer and band.
    pub(super) fn stack_gap(self) -> f32 {
        match self {
            Self::LeftToRight => 18.,
            Self::TopToBottom => 28.,
        }
    }

    /// The point at `along` (the arrows' direction) and `across`.
    pub(super) fn point(self, along: f32, across: f32) -> DiagramPoint {
        match self {
            Self::LeftToRight => DiagramPoint {
                x: along,
                y: across,
            },
            Self::TopToBottom => DiagramPoint {
                x: across,
                y: along,
            },
        }
    }

    /// The rectangle at `along` and `across`, `along_size` long in the
    /// arrows' direction and `across_size` across it.
    pub(super) fn rect(
        self,
        along: f32,
        across: f32,
        along_size: f32,
        across_size: f32,
    ) -> DiagramRect {
        let origin = self.point(along, across);
        let (width, height) = match self {
            Self::LeftToRight => (along_size, across_size),
            Self::TopToBottom => (across_size, along_size),
        };
        DiagramRect {
            x: origin.x,
            y: origin.y,
            width,
            height,
        }
    }

    /// A point's position in the arrows' direction.
    pub(super) fn along_of(self, point: DiagramPoint) -> f32 {
        match self {
            Self::LeftToRight => point.x,
            Self::TopToBottom => point.y,
        }
    }

    /// A point's position across the arrows' direction.
    pub(super) fn across_of(self, point: DiagramPoint) -> f32 {
        match self {
            Self::LeftToRight => point.y,
            Self::TopToBottom => point.x,
        }
    }

    /// A rectangle's start and end in the arrows' direction.
    pub(super) fn along_span(self, rect: DiagramRect) -> (f32, f32) {
        match self {
            Self::LeftToRight => (rect.x, rect.right()),
            Self::TopToBottom => (rect.y, rect.bottom()),
        }
    }

    /// A rectangle's start and end across the arrows' direction.
    pub(super) fn across_span(self, rect: DiagramRect) -> (f32, f32) {
        match self {
            Self::LeftToRight => (rect.y, rect.bottom()),
            Self::TopToBottom => (rect.x, rect.right()),
        }
    }

    /// A size (width, height) as its length in the arrows' direction and across it.
    pub(super) fn along_and_across(self, (width, height): (f32, f32)) -> (f32, f32) {
        match self {
            Self::LeftToRight => (width, height),
            Self::TopToBottom => (height, width),
        }
    }

    /// Room a boundary needs before its boxes, across the arrows: its name
    /// sits above the boxes, which is across only for horizontal bands.
    pub(super) fn band_lead(self) -> f32 {
        match self {
            Self::LeftToRight => GROUP_LABEL_HEIGHT + GROUP_PADDING,
            Self::TopToBottom => GROUP_PADDING,
        }
    }
}
