use super::code_anchor::CodeAnchor;
use super::step_weight::StepWeight;

/// One step of a reading order: a coherent piece of the change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TourStep {
    pub title: String,
    /// One or two sentences, Markdown: what this piece does for the change.
    pub explanation: String,
    pub weight: StepWeight,
    /// Where the piece starts in the diff; the step links there.
    pub anchor: Option<CodeAnchor>,
}
