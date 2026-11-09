use std::ops::Range;

/// A page of a written brief: its tab title, its one-line summary, and the
/// range of the brief's widgets on it.
pub struct PreparedPage {
    pub title: String,
    pub summary: String,
    pub widgets: Range<usize>,
}
