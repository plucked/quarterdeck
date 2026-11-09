use super::tour_step::TourStep;

/// The order to read the change in: foundations before what builds on
/// them, the core early, mechanical pieces marked so they can be skimmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudioTour {
    pub title: String,
    pub steps: Vec<TourStep>,
}
