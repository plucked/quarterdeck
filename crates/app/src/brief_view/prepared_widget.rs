use std::sync::Arc;

use qd_markdown::MarkdownDocument;
use qd_studio::{CalloutTone, CodeAnchor, StudioGraph, StudioSequence, TableRole};

use super::graph_layout::GraphLayout;
use super::prepared_step::PreparedStep;
use super::sequence_layout::SequenceLayout;

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
