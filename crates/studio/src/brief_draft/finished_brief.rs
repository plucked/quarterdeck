use crate::Studio;

/// The exact frozen artifact, retaining the full wire answer for reports and metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinishedBrief {
    pub studio: Studio,
    pub artifact: serde_json::Value,
}
