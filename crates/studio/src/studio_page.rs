use super::studio_widget::StudioWidget;

/// One page of a studio: a step of the walk through the change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StudioPage {
    /// A short name for the page's tab, such as `Overview` or `The hold`.
    pub title: String,
    /// One sentence on what the page answers, shown under its title and
    /// on the button leading to it.
    pub summary: String,
    pub widgets: Vec<StudioWidget>,
}
