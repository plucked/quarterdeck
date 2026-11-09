use super::rejected_widget::RejectedWidget;
use super::studio::Studio;

/// A studio as read from an agent's output: the widgets that passed the
/// checks, and the ones left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckedStudio {
    pub studio: Studio,
    pub rejected: Vec<RejectedWidget>,
}
