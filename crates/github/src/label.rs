use serde::Deserialize;

/// A label on a pull request, as GitHub shows it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct Label {
    pub name: String,
    /// Six hex digits without `#`, as GitHub sends it.
    pub color: String,
}
