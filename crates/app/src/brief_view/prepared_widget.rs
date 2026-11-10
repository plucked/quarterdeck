use std::sync::Arc;

use qd_markdown::MarkdownDocument;
use qd_studio::{CalloutTone, CodeAnchor, StudioGraph, StudioSequence, StudioWidget, TableRole};

use super::diagram_geometry::{DEFAULT_DIAGRAM_WIDTH, DiagramSize};
use super::graph_layout::{GraphLayout, lay_out_graph};
use super::prepared_step::PreparedStep;
use super::sequence_layout::{SequenceLayout, lay_out_sequence};

/// The least change of a diagram's width or height that counts as resized.
const RESIZE_THRESHOLD: f32 = 0.5;

/// A studio widget with its Markdown parsed and its diagram laid out once,
/// ready to draw.
pub enum PreparedWidget {
    Section {
        heading: String,
        body: Arc<MarkdownDocument>,
    },
    Callout {
        tone: CalloutTone,
        text: Arc<MarkdownDocument>,
    },
    CodeExcerpt {
        caption: String,
        anchor: CodeAnchor,
    },
    CodeCompare {
        caption: String,
        before: CodeAnchor,
        after: CodeAnchor,
    },
    Table {
        title: String,
        columns: Vec<String>,
        rows: Vec<Vec<Arc<MarkdownDocument>>>,
        role: TableRole,
    },
    /// What changes, what stays the same, what is mechanical; inline Markdown items.
    Scope {
        changes: Vec<Arc<MarkdownDocument>>,
        unchanged: Vec<Arc<MarkdownDocument>>,
        mechanical: Vec<Arc<MarkdownDocument>>,
    },
    /// The reading order.
    Tour {
        title: String,
        steps: Vec<PreparedStep>,
    },
    /// A graph with its layout worked out.
    Graph {
        graph: StudioGraph,
        layout: GraphLayout,
    },
    /// A sequence with its layout worked out.
    Sequence {
        sequence: StudioSequence,
        layout: SequenceLayout,
    },
}

impl PreparedWidget {
    /// `widget` with its Markdown parsed and its diagram laid out for the
    /// default width (before the column is measured).
    pub fn prepare(widget: StudioWidget) -> Self {
        match widget {
            StudioWidget::Section { heading, body } => Self::Section {
                heading,
                body: parse(&body),
            },
            StudioWidget::Callout { tone, text } => Self::Callout {
                tone,
                text: parse(&text),
            },
            StudioWidget::CodeExcerpt { caption, anchor } => Self::CodeExcerpt { caption, anchor },
            StudioWidget::CodeCompare {
                caption,
                before,
                after,
            } => Self::CodeCompare {
                caption,
                before,
                after,
            },
            StudioWidget::Table(table) => Self::Table {
                rows: table
                    .rows
                    .iter()
                    .map(|row| row.iter().map(|cell| parse(cell)).collect())
                    .collect(),
                title: table.title,
                columns: table.columns,
                role: table.role,
            },
            StudioWidget::Scope(scope) => {
                let parse_all = |items: &[String]| items.iter().map(|item| parse(item)).collect();
                Self::Scope {
                    changes: parse_all(&scope.changes),
                    unchanged: parse_all(&scope.unchanged),
                    mechanical: parse_all(&scope.mechanical),
                }
            }
            StudioWidget::Tour(tour) => Self::Tour {
                steps: tour
                    .steps
                    .into_iter()
                    .map(|step| PreparedStep {
                        explanation: parse(&step.explanation),
                        title: step.title,
                        weight: step.weight,
                        anchor: step.anchor,
                    })
                    .collect(),
                title: tour.title,
            },
            StudioWidget::Graph(graph) => Self::Graph {
                layout: lay_out_graph(&graph, DEFAULT_DIAGRAM_WIDTH),
                graph,
            },
            StudioWidget::Sequence(sequence) => Self::Sequence {
                layout: lay_out_sequence(&sequence, DEFAULT_DIAGRAM_WIDTH),
                sequence,
            },
        }
    }

    /// Lays a diagram out again for a column `width` wide; whether its size
    /// changed, so its row needs measuring again. Other widgets stay as they are.
    pub fn fit_diagram(&mut self, width: f32) -> bool {
        match self {
            Self::Graph { graph, layout } => {
                replace_layout(layout, lay_out_graph(graph, width), GraphLayout::size)
            }
            Self::Sequence { sequence, layout } => replace_layout(
                layout,
                lay_out_sequence(sequence, width),
                SequenceLayout::size,
            ),
            Self::Section { .. }
            | Self::Callout { .. }
            | Self::CodeExcerpt { .. }
            | Self::CodeCompare { .. }
            | Self::Table { .. }
            | Self::Scope { .. }
            | Self::Tour { .. } => false,
        }
    }
}

/// Markdown text parsed once, shared by every frame that draws it.
fn parse(text: &str) -> Arc<MarkdownDocument> {
    Arc::new(MarkdownDocument::parse(text))
}

/// Puts `fitted` in place of `layout`; whether that changed its size by
/// half a pixel or more.
fn replace_layout<Layout>(
    layout: &mut Layout,
    fitted: Layout,
    size_of: fn(&Layout) -> DiagramSize,
) -> bool {
    let before = size_of(layout);
    let after = size_of(&fitted);
    *layout = fitted;
    (before.width - after.width).abs() >= RESIZE_THRESHOLD
        || (before.height - after.height).abs() >= RESIZE_THRESHOLD
}
