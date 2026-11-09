use crate::brief_view::BriefPageRow;

/// One row of the brief's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BriefRow {
    /// No brief yet, one being written, or a failed run: what to do next.
    Status,
    /// A row of the written brief's shown page.
    Page(BriefPageRow),
}
