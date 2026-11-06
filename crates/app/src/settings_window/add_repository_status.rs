use gpui::SharedString;

/// Where adding a typed repository stands.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum AddRepositoryStatus {
    /// Nothing sent yet.
    #[default]
    Idle,
    /// Asking GitHub whether the repository exists.
    Checking,
    /// It could not be added, and why.
    Failed(SharedString),
}
