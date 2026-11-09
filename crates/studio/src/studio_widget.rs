use super::callout_tone::CalloutTone;
use super::change_scope::ChangeScope;
use super::code_anchor::CodeAnchor;
use super::studio_graph::StudioGraph;
use super::studio_sequence::StudioSequence;
use super::studio_table::StudioTable;
use super::studio_tour::StudioTour;

/// One block of a studio, in reading order. Code is never part of a widget:
/// widgets point at lines with [`CodeAnchor`]s and the app quotes them from
/// the diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StudioWidget {
    /// A heading naming a mechanism or consequence, and Markdown prose.
    Section {
        heading: String,
        body: String,
    },
    Callout {
        tone: CalloutTone,
        text: String,
    },
    /// Lines of one file, with a caption saying what to see in them.
    CodeExcerpt {
        caption: String,
        anchor: CodeAnchor,
    },
    /// The same place before and after the change, side by side.
    CodeCompare {
        caption: String,
        before: CodeAnchor,
        after: CodeAnchor,
    },
    Table(StudioTable),
    /// What changes, what stays the same, what is mechanical.
    Scope(ChangeScope),
    /// The order to read the change in.
    Tour(StudioTour),
    /// Boxes and labelled arrows, laid out by the app.
    Graph(StudioGraph),
    /// Participants and the messages between them, in time order.
    Sequence(StudioSequence),
}
