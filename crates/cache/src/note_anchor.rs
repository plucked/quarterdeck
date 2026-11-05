use qc_github::LineSide;

/// Where a review note sits.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NoteAnchor {
    /// On line `line` of `side`, or on lines `start_line..=line`.
    Line {
        side: LineSide,
        line: u32,
        start_line: Option<u32>,
    },
    /// A reply to the GitHub conversation with node id `thread_id`.
    Reply { thread_id: String },
}
