/// What a filter chip is set to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetChoice {
    /// Neither picked nor hidden.
    Open,
    /// Picked: the row shows only its picked values.
    Only,
    /// Hidden, whatever else is picked.
    Hidden,
}
