/// How much of a folder is viewed, for its tree checkbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderViewedState {
    None,
    Some,
    All,
}

impl FolderViewedState {
    /// The state for `viewed` of `total` files.
    pub fn of(viewed: usize, total: usize) -> Self {
        match viewed {
            0 => Self::None,
            viewed if viewed >= total => Self::All,
            _ => Self::Some,
        }
    }
}
